use crate::{
    builtins::{Builtin, BuiltinFuture, BuiltinRegistry},
    shell::{CommandOutput, ExitCode, SharedShellState, ShellAction},
};

pub struct TypeBuiltin;

impl Builtin for TypeBuiltin {
    fn name(&self) -> &'static str {
        "type"
    }

    fn execute<'a>(&'a self, _state: SharedShellState, args: &'a [String]) -> BuiltinFuture<'a> {
        Box::pin(async move {
            if args.len() != 1 {
                return Ok(ShellAction::continue_with(CommandOutput {
                    exit_code: ExitCode::FAILURE,
                    stdout: String::new(),
                    stderr: "type: expected exactly one argument\n".to_string(),
                }));
            }

            let needle = &args[0];
            let registry = BuiltinRegistry::defaults();

            if registry.contains(needle) {
                return Ok(ShellAction::continue_with(CommandOutput {
                    exit_code: ExitCode::SUCCESS,
                    stdout: format!("{needle} is a shell builtin\n"),
                    stderr: String::new(),
                }));
            }

            Ok(ShellAction::continue_with(CommandOutput {
                exit_code: ExitCode::FAILURE,
                stdout: String::new(),
                stderr: format!("type: {needle} not found\n"),
            }))
        })
    }
}
