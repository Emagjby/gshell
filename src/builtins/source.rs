use crate::{
    builtins::{Builtin, BuiltinFuture},
    runtime,
    shell::{CommandOutput, ExitCode, SharedShellState, ShellAction},
};

pub struct SourceBuiltin;

impl Builtin for SourceBuiltin {
    fn name(&self) -> &'static str {
        "source"
    }

    fn execute<'a>(&'a self, state: SharedShellState, args: &'a [String]) -> BuiltinFuture<'a> {
        Box::pin(async move {
            let path = match args {
                [path] => path,
                [] => {
                    return Ok(ShellAction::continue_with(CommandOutput {
                        exit_code: ExitCode::FAILURE,
                        stdout: String::new(),
                        stderr: "source: usage: source <path>\n".to_string(),
                    }));
                }
                _ => {
                    return Ok(ShellAction::continue_with(CommandOutput {
                        exit_code: ExitCode::FAILURE,
                        stdout: String::new(),
                        stderr: "source: too many arguments\n".to_string(),
                    }));
                }
            };

            runtime::source_file(state, path).await
        })
    }
}
