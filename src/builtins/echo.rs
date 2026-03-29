use crate::{
    builtins::{Builtin, BuiltinFuture},
    shell::{CommandOutput, ExitCode, SharedShellState, ShellAction},
};

pub struct EchoBuiltin;

impl Builtin for EchoBuiltin {
    fn name(&self) -> &'static str {
        "echo"
    }

    fn execute<'a>(&'a self, _state: SharedShellState, args: &'a [String]) -> BuiltinFuture<'a> {
        Box::pin(async move {
            let stdout = if args.is_empty() {
                "\n".to_string()
            } else {
                format!("{}\n", args.join(" "))
            };

            Ok(ShellAction::continue_with(CommandOutput {
                exit_code: ExitCode::SUCCESS,
                stdout,
                stderr: String::new(),
            }))
        })
    }
}
