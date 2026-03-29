use crate::{
    builtins::{Builtin, BuiltinFuture},
    shell::{CommandOutput, ExitCode, SharedShellState, ShellAction},
};

pub struct ClearBuiltin;

impl Builtin for ClearBuiltin {
    fn name(&self) -> &'static str {
        "clear"
    }

    fn execute<'a>(&'a self, _state: SharedShellState, _args: &'a [String]) -> BuiltinFuture<'a> {
        Box::pin(async move {
            Ok(ShellAction::continue_with(CommandOutput {
                exit_code: ExitCode::SUCCESS,
                stdout: "\x1B[2J\x1B[H".to_string(),
                stderr: String::new(),
            }))
        })
    }
}
