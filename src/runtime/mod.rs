use std::{
    collections::HashMap,
    env,
    ffi::OsString,
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    pin::Pin,
    process::Stdio,
};

use tokio::process::Command;

use crate::{
    ast::{CommandNode, Redirection, RedirectionKind, ShellExpr},
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
                    if let ShellExpr::Command(CommandNode::Simple(simple)) = expr
                        && let Some((name, args)) = simple.argv.split_first()
                    {
                        let registry = BuiltinRegistry::with_defaults();

                        if let Some(builtin) = registry.get(name) {
                            let result = builtin.execute(state.clone(), args).await?;

                            return match result {
                                ShellAction::Continue(output) => {
                                    let plan = match build_redirection_plan(&simple.redirections) {
                                        Ok(plan) => plan,
                                        Err(err) => {
                                            return Ok(ShellAction::continue_with(CommandOutput {
                                                exit_code: ExitCode::FAILURE,
                                                stdout: String::new(),
                                                stderr: format!("{err}\n"),
                                            }));
                                        }
                                    };

                                    let redirected = match apply_builtin_redirections(output, &plan)
                                    {
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
                            };
                        }

                        return execute_external(state.clone(), name, args, &simple.redirections)
                            .await;
                    }

                    Ok(ShellAction::continue_with(CommandOutput::success()))
                }
            }
        })
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
