use std::path::PathBuf;

use crate::{
    builtins::{Builtin, BuiltinFuture},
    shell::{CommandOutput, ExitCode, SharedShellState, ShellAction},
};

pub struct CdBuiltin;

impl Builtin for CdBuiltin {
    fn name(&self) -> &'static str {
        "cd"
    }

    fn execute<'a>(&'a self, state: SharedShellState, args: &'a [String]) -> BuiltinFuture<'a> {
        Box::pin(async move {
            let target = match args {
                [] => std::env::var("HOME")
                    .map(PathBuf::from)
                    .map_err(|_| crate::shell::ShellError::message("cd: HOME not set"))?,
                [path] => PathBuf::from(path),
                _ => {
                    return Ok(ShellAction::continue_with(CommandOutput {
                        exit_code: ExitCode::FAILURE,
                        stdout: String::new(),
                        stderr: "cd: too many arguments\n".to_string(),
                    }));
                }
            };

            let resolved = if target.is_absolute() {
                target
            } else {
                let cwd = state.read().await.cwd().to_path_buf();
                cwd.join(target)
            };

            match std::fs::canonicalize(&resolved) {
                Ok(path) => {
                    state.write().await.set_cwd(path);
                    Ok(ShellAction::continue_with(CommandOutput::success()))
                }
                Err(err) => Ok(ShellAction::continue_with(CommandOutput {
                    exit_code: ExitCode::FAILURE,
                    stdout: String::new(),
                    stderr: format!("cd: {err}\n"),
                })),
            }
        })
    }
}
