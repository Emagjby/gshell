use crate::{
    builtins::{Builtin, BuiltinFuture},
    shell::{CommandOutput, SharedShellState},
};

pub struct HistoryBuiltin;

impl Builtin for HistoryBuiltin {
    fn name(&self) -> &'static str {
        "history"
    }

    fn execute<'a>(&'a self, state: SharedShellState, _args: &'a [String]) -> BuiltinFuture<'a> {
        Box::pin(async move {
            let state = state.read().await;
            let mut stdout = String::new();

            for (i, entry) in state.history().entries().iter().enumerate() {
                stdout.push_str(&format!("{:>5} {}\n", i + 1, entry));
            }

            Ok(CommandOutput {
                exit_code: crate::shell::ExitCode::SUCCESS,
                stdout,
                stderr: String::new(),
            })
        })
    }
}
