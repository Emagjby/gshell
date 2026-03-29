use crate::{
    builtins::{Builtin, BuiltinFuture},
    shell::{CommandOutput, ExitCode, SharedShellState, ShellAction},
};

pub struct PwdBuiltin;

impl Builtin for PwdBuiltin {
    fn name(&self) -> &'static str {
        "pwd"
    }

    fn execute<'a>(&'a self, state: SharedShellState, _args: &'a [String]) -> BuiltinFuture<'a> {
        Box::pin(async move {
            let state = state.read().await;
            let stdout = format!("{}\n", state.cwd().display());

            Ok(ShellAction::continue_with(CommandOutput {
                exit_code: ExitCode::SUCCESS,
                stdout,
                stderr: String::new(),
            }))
        })
    }
}
