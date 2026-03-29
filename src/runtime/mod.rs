use std::pin::Pin;

use crate::{
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
                ParsedCommand::Raw(input) => {
                    let parts = input
                        .split_whitespace()
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>();

                    if let Some((name, args)) = parts.split_first() {
                        let registry = BuiltinRegistry::with_defaults();

                        if let Some(builtin) = registry.get(name) {
                            return builtin.execute(state.clone(), args).await;
                        }
                    }

                    Ok(ShellAction::continue_with(CommandOutput {
                        exit_code: ExitCode::FAILURE,
                        stdout: String::new(),
                        stderr: format!("command not found: {input}\n"),
                    }))
                }
            }
        })
    }
}
