use std::{
    collections::HashMap,
    env,
    ffi::OsString,
    fs::{File, OpenOptions},
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    process::Stdio,
    sync::Arc,
};

use tokio::{io::AsyncWriteExt, process::Command, sync::RwLock};

use crate::{
    ast::{BoolOp, CommandNode, Redirection, RedirectionKind, ShellExpr, SimpleCommand},
    builtins::BuiltinRegistry,
    parser::ParsedCommand,
    shell::{CommandOutput, ExitCode, SharedShellState, ShellAction, ShellResult},
};

#[derive(Debug)]
struct RedirectionPlan {
    stdin: Option<PathBuf>,
    stdout: Option<OutputRedirection>,
    stderr: Option<OutputRedirection>,
}

#[derive(Debug, Clone)]
struct OutputRedirection {
    path: PathBuf,
    append: bool,
}

impl RedirectionPlan {
    fn empty() -> Self {
        Self {
            stdin: None,
            stdout: None,
            stderr: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecutionMode {
    Normal,
    Pipeline,
}

#[derive(Debug, Clone)]
struct PipelineOutput {
    exit_code: ExitCode,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

pub type ExecutorFuture<'a> = Pin<Box<dyn Future<Output = ShellResult<ShellAction>> + Send + 'a>>;

pub trait Executor<C>: Send + Sync {
    fn execute<'a>(&'a self, state: SharedShellState, command: &'a C) -> ExecutorFuture<'a>;
}

#[derive(Debug, Default)]
pub struct BootstrapExecutor;

impl Executor<ParsedCommand> for BootstrapExecutor {
    fn execute<'a>(
        &'a self,
        state: SharedShellState,
        command: &'a ParsedCommand,
    ) -> ExecutorFuture<'a> {
        Box::pin(async move {
            match command {
                ParsedCommand::Empty => Ok(ShellAction::continue_with(CommandOutput::success())),
                ParsedCommand::Expr(expr) => {
                    self.execute_expr(state, expr, ExecutionMode::Normal).await
                }
            }
        })
    }
}

impl BootstrapExecutor {
    fn execute_expr<'a>(
        &'a self,
        state: SharedShellState,
        expr: &'a ShellExpr,
        mode: ExecutionMode,
    ) -> Pin<Box<dyn Future<Output = ShellResult<ShellAction>> + Send + 'a>> {
        Box::pin(async move {
            match expr {
                ShellExpr::Command(node) => self.execute_command_node(state, node, mode).await,
                ShellExpr::Pipeline(commands) => self.execute_pipeline(state, commands).await,
                ShellExpr::BooleanChain { first, rest } => {
                    self.execute_boolean_chain(state, first, rest, mode).await
                }
                ShellExpr::Sequence(exprs) => self.execute_sequence(state, exprs, mode).await,
            }
        })
    }

    async fn execute_command_node(
        &self,
        state: SharedShellState,
        node: &CommandNode,
        mode: ExecutionMode,
    ) -> ShellResult<ShellAction> {
        match node {
            CommandNode::Simple(simple) => self.execute_simple_command(state, simple, mode).await,
            CommandNode::Group(expr) | CommandNode::Subshell(expr) => {
                self.execute_expr(state, expr, mode).await
            }
        }
    }

    async fn execute_simple_command(
        &self,
        state: SharedShellState,
        simple: &SimpleCommand,
        mode: ExecutionMode,
    ) -> ShellResult<ShellAction> {
        let Some((name, args)) = simple.argv.split_first() else {
            return Ok(ShellAction::continue_with(CommandOutput::success()));
        };

        let registry = BuiltinRegistry::with_defaults();

        if let Some(builtin) = registry.get(name) {
            return self
                .execute_builtin_simple(state, builtin, args, &simple.redirections, mode)
                .await;
        }

        match mode {
            ExecutionMode::Normal => {
                execute_external(state, name, args, &simple.redirections).await
            }
            ExecutionMode::Pipeline => {
                let output = self
                    .execute_external_pipeline_segment(state, simple, None)
                    .await?;

                Ok(ShellAction::continue_with(CommandOutput {
                    exit_code: output.exit_code,
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                }))
            }
        }
    }

    async fn execute_builtin_simple(
        &self,
        state: SharedShellState,
        builtin: Arc<dyn crate::builtins::Builtin>,
        args: &[String],
        redirections: &[Redirection],
        mode: ExecutionMode,
    ) -> ShellResult<ShellAction> {
        match mode {
            ExecutionMode::Normal => {
                let result = builtin.execute(state, args).await?;

                match result {
                    ShellAction::Continue(output) => {
                        let plan = match build_redirection_plan(redirections) {
                            Ok(plan) => plan,
                            Err(err) => {
                                return Ok(ShellAction::continue_with(CommandOutput {
                                    exit_code: ExitCode::FAILURE,
                                    stdout: String::new(),
                                    stderr: format!("{err}\n"),
                                }));
                            }
                        };

                        let redirected = match apply_builtin_redirections(output, &plan) {
                            Ok(output) => output,
                            Err(err) => CommandOutput {
                                exit_code: ExitCode::FAILURE,
                                stdout: String::new(),
                                stderr: format!("{err}\n"),
                            },
                        };

                        Ok(ShellAction::continue_with(redirected))
                    }
                    ShellAction::Exit(code) => Ok(ShellAction::Exit(code)),
                }
            }
            ExecutionMode::Pipeline => {
                let isolated_state = clone_shell_state_for_pipeline(&state).await;
                let result = builtin.execute(isolated_state, args).await?;

                match result {
                    ShellAction::Continue(output) => {
                        let plan = match build_redirection_plan(redirections) {
                            Ok(plan) => plan,
                            Err(err) => {
                                return Ok(ShellAction::continue_with(CommandOutput {
                                    exit_code: ExitCode::FAILURE,
                                    stdout: String::new(),
                                    stderr: format!("{err}\n"),
                                }));
                            }
                        };

                        let redirected = match apply_builtin_redirections(output, &plan) {
                            Ok(output) => output,
                            Err(err) => CommandOutput {
                                exit_code: ExitCode::FAILURE,
                                stdout: String::new(),
                                stderr: format!("{err}\n"),
                            },
                        };

                        Ok(ShellAction::continue_with(redirected))
                    }
                    ShellAction::Exit(code) => Ok(ShellAction::Exit(code)),
                }
            }
        }
    }

    async fn execute_pipeline(
        &self,
        state: SharedShellState,
        commands: &[CommandNode],
    ) -> ShellResult<ShellAction> {
        if commands.is_empty() {
            return Ok(ShellAction::continue_with(CommandOutput::success()));
        }

        let mut stdin_buffer: Option<Vec<u8>> = None;
        let mut last_output = PipelineOutput {
            exit_code: ExitCode::SUCCESS,
            stdout: Vec::new(),
            stderr: Vec::new(),
        };

        for command in commands {
            last_output = self
                .execute_pipeline_segment(state.clone(), command, stdin_buffer.take())
                .await?;
            stdin_buffer = Some(last_output.stdout.clone());
        }

        Ok(ShellAction::continue_with(CommandOutput {
            exit_code: last_output.exit_code,
            stdout: String::from_utf8_lossy(&last_output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&last_output.stderr).into_owned(),
        }))
    }

    async fn execute_pipeline_segment(
        &self,
        state: SharedShellState,
        node: &CommandNode,
        stdin_data: Option<Vec<u8>>,
    ) -> ShellResult<PipelineOutput> {
        match node {
            CommandNode::Simple(simple) => {
                let Some((name, args)) = simple.argv.split_first() else {
                    return Ok(PipelineOutput {
                        exit_code: ExitCode::SUCCESS,
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    });
                };

                let registry = BuiltinRegistry::with_defaults();

                if let Some(builtin) = registry.get(name) {
                    let isolated_state = clone_shell_state_for_pipeline(&state).await;
                    let result = builtin.execute(isolated_state, args).await?;

                    return Ok(match result {
                        ShellAction::Continue(output) => {
                            let plan = match build_redirection_plan(&simple.redirections) {
                                Ok(plan) => plan,
                                Err(err) => {
                                    return Ok(PipelineOutput {
                                        exit_code: ExitCode::FAILURE,
                                        stdout: Vec::new(),
                                        stderr: format!("{err}\n").into_bytes(),
                                    });
                                }
                            };

                            let redirected = match apply_builtin_redirections(output, &plan) {
                                Ok(output) => output,
                                Err(err) => CommandOutput {
                                    exit_code: ExitCode::FAILURE,
                                    stdout: String::new(),
                                    stderr: format!("{err}\n"),
                                },
                            };

                            PipelineOutput {
                                exit_code: redirected.exit_code,
                                stdout: redirected.stdout.into_bytes(),
                                stderr: redirected.stderr.into_bytes(),
                            }
                        }
                        ShellAction::Exit(code) => PipelineOutput {
                            exit_code: code,
                            stdout: Vec::new(),
                            stderr: Vec::new(),
                        },
                    });
                }

                self.execute_external_pipeline_segment(state, simple, stdin_data)
                    .await
            }
            CommandNode::Group(expr) | CommandNode::Subshell(expr) => {
                let action = self
                    .execute_expr(state, expr, ExecutionMode::Pipeline)
                    .await?;

                Ok(match action {
                    ShellAction::Continue(output) => PipelineOutput {
                        exit_code: output.exit_code,
                        stdout: output.stdout.into_bytes(),
                        stderr: output.stderr.into_bytes(),
                    },
                    ShellAction::Exit(code) => PipelineOutput {
                        exit_code: code,
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    },
                })
            }
        }
    }

    async fn execute_external_pipeline_segment(
        &self,
        state: SharedShellState,
        simple: &SimpleCommand,
        stdin_data: Option<Vec<u8>>,
    ) -> ShellResult<PipelineOutput> {
        let (cwd, env_map) = {
            let guard = state.read().await;
            (guard.cwd().to_path_buf(), guard.env().clone())
        };

        let Some((program, args)) = simple.argv.split_first() else {
            return Ok(PipelineOutput {
                exit_code: ExitCode::SUCCESS,
                stdout: Vec::new(),
                stderr: Vec::new(),
            });
        };

        let resolved = match resolve_command_path(program, &env_map) {
            Some(path) => path,
            None => {
                return Ok(PipelineOutput {
                    exit_code: ExitCode::FAILURE,
                    stdout: Vec::new(),
                    stderr: format!("command not found: {program}\n").into_bytes(),
                });
            }
        };

        let plan = match build_redirection_plan(&simple.redirections) {
            Ok(plan) => plan,
            Err(err) => {
                return Ok(PipelineOutput {
                    exit_code: ExitCode::FAILURE,
                    stdout: Vec::new(),
                    stderr: format!("{err}\n").into_bytes(),
                });
            }
        };

        let mut command = Command::new(&resolved);
        command.args(args);
        command.current_dir(cwd);
        command.env_clear();
        command.envs(env_map);

        if let Some(path) = &plan.stdin {
            match open_input_file(path) {
                Ok(file) => {
                    command.stdin(Stdio::from(file));
                }
                Err(err) => {
                    return Ok(PipelineOutput {
                        exit_code: ExitCode::FAILURE,
                        stdout: Vec::new(),
                        stderr: format!("failed to open input file {}: {err}\n", path.display())
                            .into_bytes(),
                    });
                }
            }
        } else if stdin_data.is_some() {
            command.stdin(Stdio::piped());
        } else {
            command.stdin(Stdio::null());
        }

        if let Some(redir) = &plan.stdout {
            match open_output_file(redir) {
                Ok(file) => {
                    command.stdout(Stdio::from(file));
                }
                Err(err) => {
                    return Ok(PipelineOutput {
                        exit_code: ExitCode::FAILURE,
                        stdout: Vec::new(),
                        stderr: format!(
                            "failed to open output file {}: {err}\n",
                            redir.path.display()
                        )
                        .into_bytes(),
                    });
                }
            }
        } else {
            command.stdout(Stdio::piped());
        }

        if let Some(redir) = &plan.stderr {
            match open_output_file(redir) {
                Ok(file) => {
                    command.stderr(Stdio::from(file));
                }
                Err(err) => {
                    return Ok(PipelineOutput {
                        exit_code: ExitCode::FAILURE,
                        stdout: Vec::new(),
                        stderr: format!(
                            "failed to open error file {}: {err}\n",
                            redir.path.display()
                        )
                        .into_bytes(),
                    });
                }
            }
        } else {
            command.stderr(Stdio::piped());
        }

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => {
                return Ok(PipelineOutput {
                    exit_code: ExitCode::FAILURE,
                    stdout: Vec::new(),
                    stderr: format!("failed to execute {program}: {err}\n").into_bytes(),
                });
            }
        };

        if let Some(input) = stdin_data
            && let Some(mut stdin) = child.stdin.take()
        {
            stdin.write_all(&input).await?;
        }

        let output = child.wait_with_output().await?;

        let code = output
            .status
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .unwrap_or(1);

        Ok(PipelineOutput {
            exit_code: ExitCode::new(code),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }

    async fn execute_boolean_chain(
        &self,
        state: SharedShellState,
        first: &ShellExpr,
        rest: &[(BoolOp, ShellExpr)],
        mode: ExecutionMode,
    ) -> ShellResult<ShellAction> {
        let mut last = self.execute_expr(state.clone(), first, mode).await?;
        let mut aggregate = match &last {
            ShellAction::Continue(output) => output.clone(),
            ShellAction::Exit(code) => return Ok(ShellAction::Exit(*code)),
        };

        for (op, expr) in rest {
            let should_run = match op {
                BoolOp::And => aggregate.exit_code.is_success(),
                BoolOp::Or => aggregate.exit_code.is_failure(),
            };

            if should_run {
                last = self.execute_expr(state.clone(), expr, mode).await?;

                match last {
                    ShellAction::Continue(output) => {
                        aggregate = merge_outputs(aggregate, output);
                    }
                    ShellAction::Exit(code) => return Ok(ShellAction::Exit(code)),
                }
            }
        }

        Ok(ShellAction::continue_with(aggregate))
    }

    async fn execute_sequence(
        &self,
        state: SharedShellState,
        exprs: &[ShellExpr],
        mode: ExecutionMode,
    ) -> ShellResult<ShellAction> {
        let mut aggregate = CommandOutput::success();

        for expr in exprs {
            let action = self.execute_expr(state.clone(), expr, mode).await?;

            match action {
                ShellAction::Continue(output) => {
                    aggregate = merge_outputs(aggregate, output);
                }
                ShellAction::Exit(code) => return Ok(ShellAction::Exit(code)),
            }
        }

        Ok(ShellAction::continue_with(aggregate))
    }
}

async fn execute_external(
    state: SharedShellState,
    program: &str,
    args: &[String],
    redirections: &[Redirection],
) -> ShellResult<ShellAction> {
    let (cwd, env_map) = {
        let guard = state.read().await;
        (guard.cwd().to_path_buf(), guard.env().clone())
    };

    let resolved = match resolve_command_path(program, &env_map) {
        Some(path) => path,
        None => {
            return Ok(ShellAction::continue_with(CommandOutput {
                exit_code: ExitCode::FAILURE,
                stdout: String::new(),
                stderr: format!("command not found: {program}\n"),
            }));
        }
    };

    let plan = match build_redirection_plan(redirections) {
        Ok(plan) => plan,
        Err(err) => {
            return Ok(ShellAction::continue_with(CommandOutput {
                exit_code: ExitCode::FAILURE,
                stdout: String::new(),
                stderr: format!("{err}\n"),
            }));
        }
    };

    let mut command = Command::new(&resolved);
    command.args(args);
    command.current_dir(cwd);
    command.env_clear();
    command.envs(env_map);

    if let Some(path) = &plan.stdin {
        match open_input_file(path) {
            Ok(file) => {
                command.stdin(Stdio::from(file));
            }
            Err(err) => {
                return Ok(ShellAction::continue_with(CommandOutput {
                    exit_code: ExitCode::FAILURE,
                    stdout: String::new(),
                    stderr: format!("failed to open input file {}: {err}\n", path.display()),
                }));
            }
        }
    } else {
        command.stdin(Stdio::inherit());
    }

    if let Some(redir) = &plan.stdout {
        match open_output_file(redir) {
            Ok(file) => {
                command.stdout(Stdio::from(file));
            }
            Err(err) => {
                return Ok(ShellAction::continue_with(CommandOutput {
                    exit_code: ExitCode::FAILURE,
                    stdout: String::new(),
                    stderr: format!(
                        "failed to open output file {}: {err}\n",
                        redir.path.display()
                    ),
                }));
            }
        }
    } else {
        command.stdout(Stdio::inherit());
    }

    if let Some(redir) = &plan.stderr {
        match open_output_file(redir) {
            Ok(file) => {
                command.stderr(Stdio::from(file));
            }
            Err(err) => {
                return Ok(ShellAction::continue_with(CommandOutput {
                    exit_code: ExitCode::FAILURE,
                    stdout: String::new(),
                    stderr: format!(
                        "failed to open error file {}: {err}\n",
                        redir.path.display()
                    ),
                }));
            }
        }
    } else {
        command.stderr(Stdio::inherit());
    }

    let status = match command.status().await {
        Ok(status) => status,
        Err(err) => {
            return Ok(ShellAction::continue_with(CommandOutput {
                exit_code: ExitCode::FAILURE,
                stdout: String::new(),
                stderr: format!("failed to execute {program}: {err}\n"),
            }));
        }
    };

    let code = status
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .unwrap_or(1);

    Ok(ShellAction::continue_with(CommandOutput {
        exit_code: ExitCode::new(code),
        stdout: String::new(),
        stderr: String::new(),
    }))
}

fn resolve_command_path(program: &str, env_map: &HashMap<String, String>) -> Option<PathBuf> {
    let candidate = Path::new(program);

    if candidate.components().count() > 1 {
        return is_executable_file(candidate).then(|| candidate.to_path_buf());
    }

    let path_var = env_map
        .get("PATH")
        .cloned()
        .unwrap_or_else(|| env::var("PATH").unwrap_or_default());

    env::split_paths(&OsString::from(path_var))
        .map(|dir| dir.join(program))
        .find(|path| is_executable_file(path))
}

fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

fn build_redirection_plan(redirections: &[Redirection]) -> ShellResult<RedirectionPlan> {
    let mut plan = RedirectionPlan::empty();

    for redirect in redirections {
        let fd = redirect.effective_fd();

        match (&redirect.kind, fd) {
            (RedirectionKind::Input, 0) => {
                plan.stdin = Some(PathBuf::from(&redirect.target));
            }
            (RedirectionKind::OutputTruncate, 1) => {
                plan.stdout = Some(OutputRedirection {
                    path: PathBuf::from(&redirect.target),
                    append: false,
                });
            }
            (RedirectionKind::OutputAppend, 1) => {
                plan.stdout = Some(OutputRedirection {
                    path: PathBuf::from(&redirect.target),
                    append: true,
                });
            }
            (RedirectionKind::OutputTruncate, 2) => {
                plan.stderr = Some(OutputRedirection {
                    path: PathBuf::from(&redirect.target),
                    append: false,
                });
            }
            (RedirectionKind::OutputAppend, 2) => {
                plan.stderr = Some(OutputRedirection {
                    path: PathBuf::from(&redirect.target),
                    append: true,
                });
            }
            (RedirectionKind::Input, fd) => {
                return Err(crate::shell::ShellError::message(format!(
                    "unsupported input redirection fd: {fd}"
                )));
            }
            (_, fd) => {
                return Err(crate::shell::ShellError::message(format!(
                    "unsupported redirection fd: {fd}"
                )));
            }
        }
    }

    Ok(plan)
}

fn open_input_file(path: &PathBuf) -> ShellResult<File> {
    File::open(path).map_err(crate::shell::ShellError::from)
}

fn open_output_file(redirection: &OutputRedirection) -> ShellResult<File> {
    let mut options = OpenOptions::new();
    options.create(true).write(true);

    if redirection.append {
        options.append(true);
    } else {
        options.truncate(true);
    }

    options
        .open(&redirection.path)
        .map_err(crate::shell::ShellError::from)
}

fn apply_builtin_redirections(
    output: CommandOutput,
    plan: &RedirectionPlan,
) -> ShellResult<CommandOutput> {
    if let Some(stdout_redirect) = &plan.stdout {
        let mut file = open_output_file(stdout_redirect)?;
        use std::io::Write;
        file.write_all(output.stdout.as_bytes())?;
    }

    if let Some(stderr_redirect) = &plan.stderr {
        let mut file = open_output_file(stderr_redirect)?;
        use std::io::Write;
        file.write_all(output.stderr.as_bytes())?;
    }

    Ok(CommandOutput {
        exit_code: output.exit_code,
        stdout: if plan.stdout.is_some() {
            String::new()
        } else {
            output.stdout
        },
        stderr: if plan.stderr.is_some() {
            String::new()
        } else {
            output.stderr
        },
    })
}

async fn clone_shell_state_for_pipeline(state: &SharedShellState) -> SharedShellState {
    let snapshot = state.read().await.clone();
    Arc::new(RwLock::new(snapshot))
}

fn merge_outputs(mut left: CommandOutput, right: CommandOutput) -> CommandOutput {
    left.stdout.push_str(&right.stdout);
    left.stderr.push_str(&right.stderr);
    left.exit_code = right.exit_code;
    left
}
