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

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use tokio::{io::AsyncWriteExt, process::Command, sync::RwLock};

use crate::{
    ast::{BoolOp, CommandNode, RedirectionKind, ShellExpr, SimpleCommand},
    builtins::BuiltinRegistry,
    expand::{
        CommandSubstitutionExecutor, expand_words_pathnames_with_state, expand_words_with_state,
    },
    parser::ParsedCommand,
    shell::{CommandOutput, ExitCode, SharedShellState, ShellAction, ShellError, ShellResult},
};

#[derive(Debug)]
struct RedirectionPlan {
    stdin: Option<InputRedirection>,
    stdout: Option<OutputRedirection>,
    stderr: Option<OutputRedirection>,
}

#[derive(Debug, Clone)]
enum InputRedirection {
    File(PathBuf),
    Inline(Vec<u8>),
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
    Capture,
    Pipeline,
}

#[derive(Debug, Clone)]
struct PipelineOutput {
    exit_code: ExitCode,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

#[derive(Debug, Clone)]
struct ExpandedRedirection {
    fd: Option<u8>,
    kind: RedirectionKind,
    target: Option<String>,
}

impl ExpandedRedirection {
    fn effective_fd(&self) -> u8 {
        match (&self.fd, &self.kind) {
            (Some(fd), _) => *fd,
            (None, RedirectionKind::Input | RedirectionKind::HereDoc { .. }) => 0,
            (None, RedirectionKind::OutputTruncate | RedirectionKind::OutputAppend) => 1,
        }
    }
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

    async fn execute_command_substitution(
        &self,
        state: SharedShellState,
        expr: ShellExpr,
    ) -> ShellResult<String> {
        let isolated_state = clone_shell_state_for_pipeline(&state).await;

        let action = self
            .execute_expr(isolated_state, &expr, ExecutionMode::Capture)
            .await?;

        match action {
            ShellAction::Continue(output) => Ok(output.stdout),
            ShellAction::Exit(_) => Err(ShellError::message(
                "command substitution cannot terminate the parent shell",
            )),
        }
    }

    async fn execute_command_node(
        &self,
        state: SharedShellState,
        node: &CommandNode,
        mode: ExecutionMode,
    ) -> ShellResult<ShellAction> {
        match node {
            CommandNode::Simple(simple) => self.execute_simple_command(state, simple, mode).await,
            CommandNode::Subshell(expr) => {
                self.execute_subshell_placeholder(state, expr, mode).await
            }
        }
    }

    async fn execute_subshell_placeholder(
        &self,
        state: SharedShellState,
        expr: &ShellExpr,
        mode: ExecutionMode,
    ) -> ShellResult<ShellAction> {
        self.execute_expr(state, expr, mode).await
    }

    async fn execute_simple_command(
        &self,
        state: SharedShellState,
        simple: &SimpleCommand,
        mode: ExecutionMode,
    ) -> ShellResult<ShellAction> {
        let (expanded_argv, expanded_redirections) =
            expand_simple_command(state.clone(), simple).await?;

        let Some((name, args)) = expanded_argv.split_first() else {
            return Ok(ShellAction::continue_with(CommandOutput::success()));
        };

        let registry = BuiltinRegistry::defaults();

        if let Some(builtin) = registry.get(name) {
            return self
                .execute_builtin_simple(state, builtin, args, &expanded_redirections, mode)
                .await;
        }

        match mode {
            ExecutionMode::Normal => {
                execute_external(state, name, args, &expanded_redirections).await
            }
            ExecutionMode::Capture => {
                let output = self
                    .execute_external_pipeline_segment(
                        state,
                        name,
                        args,
                        &expanded_redirections,
                        None,
                    )
                    .await?;

                Ok(ShellAction::continue_with(CommandOutput {
                    exit_code: output.exit_code,
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                }))
            }
            ExecutionMode::Pipeline => {
                let output = self
                    .execute_external_pipeline_segment(
                        state,
                        name,
                        args,
                        &expanded_redirections,
                        None,
                    )
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
        redirections: &[ExpandedRedirection],
        mode: ExecutionMode,
    ) -> ShellResult<ShellAction> {
        match mode {
            ExecutionMode::Normal | ExecutionMode::Capture => {
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
                let (expanded_argv, expanded_redirections) =
                    expand_simple_command(state.clone(), simple).await?;

                let Some((name, args)) = expanded_argv.split_first() else {
                    return Ok(PipelineOutput {
                        exit_code: ExitCode::SUCCESS,
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    });
                };

                let registry = BuiltinRegistry::defaults();

                if let Some(builtin) = registry.get(name) {
                    let isolated_state = clone_shell_state_for_pipeline(&state).await;
                    let result = builtin.execute(isolated_state, args).await?;

                    return Ok(match result {
                        ShellAction::Continue(output) => {
                            let plan = match build_redirection_plan(&expanded_redirections) {
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

                self.execute_external_pipeline_segment(
                    state,
                    name,
                    args,
                    &expanded_redirections,
                    stdin_data,
                )
                .await
            }
            CommandNode::Subshell(expr) => {
                let action = self
                    .execute_subshell_placeholder(state, expr, ExecutionMode::Pipeline)
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
        program: &str,
        args: &[String],
        redirections: &[ExpandedRedirection],
        stdin_data: Option<Vec<u8>>,
    ) -> ShellResult<PipelineOutput> {
        let (cwd, env_map) = {
            let guard = state.read().await;
            (guard.cwd().to_path_buf(), guard.env().clone())
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

        let plan = match build_redirection_plan(redirections) {
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

        if let Some(InputRedirection::File(path)) = &plan.stdin {
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
        } else if matches!(plan.stdin, Some(InputRedirection::Inline(_))) || stdin_data.is_some() {
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

        let stdin_bytes = match (&plan.stdin, stdin_data) {
            (Some(InputRedirection::Inline(input)), _) => Some(input.clone()),
            (Some(InputRedirection::File(_)), _) => None,
            (None, input) => input,
        };

        if let Some(input) = stdin_bytes
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
        let first_action = self.execute_expr(state.clone(), first, mode).await?;
        let mut aggregate = match first_action {
            ShellAction::Continue(output) => output,
            ShellAction::Exit(code) => return Ok(ShellAction::Exit(code)),
        };

        for (op, expr) in rest {
            let should_run = match op {
                BoolOp::And => aggregate.exit_code.is_success(),
                BoolOp::Or => aggregate.exit_code.is_failure(),
            };

            if should_run {
                let next_action = self.execute_expr(state.clone(), expr, mode).await?;
                match next_action {
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
    redirections: &[ExpandedRedirection],
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

    if let Some(InputRedirection::File(path)) = &plan.stdin {
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
    } else if matches!(plan.stdin, Some(InputRedirection::Inline(_))) {
        command.stdin(Stdio::piped());
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

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            return Ok(ShellAction::continue_with(CommandOutput {
                exit_code: ExitCode::FAILURE,
                stdout: String::new(),
                stderr: format!("failed to execute {program}: {err}\n"),
            }));
        }
    };

    if let Some(InputRedirection::Inline(input)) = &plan.stdin
        && let Some(mut stdin) = child.stdin.take()
    {
        stdin.write_all(input).await?;
    }

    let status = match child.wait().await {
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
    let Ok(metadata) = path.metadata() else {
        return false;
    };

    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        false
    }
}

fn build_redirection_plan(redirections: &[ExpandedRedirection]) -> ShellResult<RedirectionPlan> {
    let mut plan = RedirectionPlan::empty();

    for redirect in redirections {
        let fd = redirect.effective_fd();

        match (&redirect.kind, fd) {
            (RedirectionKind::Input, 0) => {
                plan.stdin = Some(InputRedirection::File(PathBuf::from(
                    redirect.target.as_deref().unwrap_or_default(),
                )));
            }
            (RedirectionKind::HereDoc { body, .. }, 0) => {
                plan.stdin = Some(InputRedirection::Inline(body.as_bytes().to_vec()));
            }
            (RedirectionKind::OutputTruncate, 1) => {
                plan.stdout = Some(OutputRedirection {
                    path: PathBuf::from(redirect.target.as_deref().unwrap_or_default()),
                    append: false,
                });
            }
            (RedirectionKind::OutputAppend, 1) => {
                plan.stdout = Some(OutputRedirection {
                    path: PathBuf::from(redirect.target.as_deref().unwrap_or_default()),
                    append: true,
                });
            }
            (RedirectionKind::OutputTruncate, 2) => {
                plan.stderr = Some(OutputRedirection {
                    path: PathBuf::from(redirect.target.as_deref().unwrap_or_default()),
                    append: false,
                });
            }
            (RedirectionKind::OutputAppend, 2) => {
                plan.stderr = Some(OutputRedirection {
                    path: PathBuf::from(redirect.target.as_deref().unwrap_or_default()),
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

async fn expand_simple_command(
    state: SharedShellState,
    simple: &SimpleCommand,
) -> ShellResult<(Vec<String>, Vec<ExpandedRedirection>)> {
    let substitution_executor: CommandSubstitutionExecutor = Arc::new(move |state, expr| {
        let executor = BootstrapExecutor;
        Box::pin(async move { executor.execute_command_substitution(state, expr).await })
    });

    for (name, value) in &simple.assignments {
        let expanded = expand_words_with_state(
            state.clone(),
            std::slice::from_ref(value),
            &substitution_executor,
        )
        .await?
        .into_iter()
        .next()
        .unwrap_or_default();

        state.write().await.set_env_var(name.clone(), expanded);
    }

    let argv =
        expand_words_pathnames_with_state(state.clone(), &simple.argv, &substitution_executor)
            .await?;

    let mut redirections = Vec::new();
    for redirection in &simple.redirections {
        let target = expand_words_with_state(
            state.clone(),
            std::slice::from_ref(&redirection.target),
            &substitution_executor,
        )
        .await?
        .into_iter()
        .next()
        .unwrap_or_default();

        let fd = redirection
            .fd
            .map(|fd| {
                u8::try_from(fd).map_err(|_| {
                    crate::shell::ShellError::message(format!("unsupported redirection fd: {fd}"))
                })
            })
            .transpose()?;

        redirections.push(ExpandedRedirection {
            fd,
            kind: match &redirection.kind {
                RedirectionKind::HereDoc { body, expand } => RedirectionKind::HereDoc {
                    body: if *expand {
                        expand_heredoc_body(state.clone(), body).await?
                    } else {
                        body.clone()
                    },
                    expand: *expand,
                },
                other => other.clone(),
            },
            target: match &redirection.kind {
                RedirectionKind::HereDoc { .. } => None,
                _ => Some(target),
            },
        });
    }

    Ok((argv, redirections))
}

async fn expand_heredoc_body(state: SharedShellState, body: &str) -> ShellResult<String> {
    let guard = state.read().await;
    let mut out = String::new();
    let mut chars = body.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => match chars.peek().copied() {
                Some('$') | Some('\\') => {
                    out.push(chars.next().expect("peeked character should exist"));
                }
                _ => out.push('\\'),
            },
            '$' => match chars.peek().copied() {
                Some('?') => {
                    chars.next();
                    out.push_str(&guard.last_exit_status().as_u8().to_string());
                }
                Some(next) if is_var_start(next) => {
                    let mut name = String::new();
                    while let Some(next) = chars.peek().copied() {
                        if is_var_continue(next) {
                            name.push(next);
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    if let Some(value) = guard.env_var(&name) {
                        out.push_str(value);
                    }
                }
                _ => out.push('$'),
            },
            _ => out.push(ch),
        }
    }

    Ok(out)
}

fn is_var_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_var_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}
