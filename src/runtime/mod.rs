use std::{
    collections::HashMap,
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    pin::Pin,
    process::Stdio,
};

use tokio::process::Command;

use crate::{
    ast::{CommandNode, ShellExpr},
    builtins::BuiltinRegistry,
    parser::ParsedCommand,
    shell::{CommandOutput, ExitCode, SharedShellState, ShellAction, ShellResult},
};

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
                ParsedCommand::Expr(expr) => match expr {
                    ShellExpr::Command(CommandNode::Simple(simple)) => {
                        if let Some((name, args)) = simple.argv.split_first() {
                            let registry = BuiltinRegistry::with_defaults();

                            if let Some(builtin) = registry.get(name) {
                                return builtin.execute(state.clone(), args).await;
                            }

                            return execute_external(state.clone(), name, args).await;
                        }
                        Ok(ShellAction::continue_with(CommandOutput::success()))
                    }
                    _ => Ok(ShellAction::continue_with(CommandOutput {
                        exit_code: ExitCode::FAILURE,
                        stdout: String::new(),
                        stderr: "compound command execution is not yet implemented\n".to_string(),
                    })),
                },
            }
        })
    }
}

async fn execute_external(
    state: SharedShellState,
    program: &str,
    args: &[String],
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

    let mut command = Command::new(&resolved);
    command.args(args);
    command.current_dir(cwd);
    command.env_clear();
    command.envs(env_map);
    command.stdin(Stdio::inherit());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    let status = match command.status().await {
        Ok(status) => status,
        Err(err) => {
            return Ok(ShellAction::continue_with(CommandOutput {
                exit_code: ExitCode::FAILURE,
                stdout: String::new(),
                stderr: format!("failed to execute {}: {}\n", program, err),
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
