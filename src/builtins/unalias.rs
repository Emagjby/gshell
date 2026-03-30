use crate::{
    builtins::{Builtin, BuiltinFuture},
    shell::{CommandOutput, ExitCode, SharedShellState, ShellAction},
};

pub struct UnaliasBuiltin;

impl Builtin for UnaliasBuiltin {
    fn name(&self) -> &'static str {
        "unalias"
    }

    fn execute<'a>(&'a self, state: SharedShellState, args: &'a [String]) -> BuiltinFuture<'a> {
        Box::pin(async move {
            if args.is_empty() {
                return Ok(ShellAction::continue_with(CommandOutput {
                    exit_code: ExitCode::FAILURE,
                    stdout: String::new(),
                    stderr: "unalias: expected at least one argument\n".to_string(),
                }));
            }

            let mut stderr = String::new();
            let mut success = true;

            for name in args {
                if state.write().await.aliases_mut().remove(name).is_none() {
                    success = false;
                    stderr.push_str(&format!("unalias: {name}: not found\n"));
                }
            }

            Ok(ShellAction::continue_with(CommandOutput {
                exit_code: if success {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                },
                stdout: String::new(),
                stderr,
            }))
        })
    }
}
