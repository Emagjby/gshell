use std::pin::Pin;

use crate::{
    parser::ParsedCommand,
    shell::{CommandOutput, ExitCode, SharedShellState, ShellResult},
};

pub type ExecutorFuture<'a> = Pin<Box<dyn Future<Output = ShellResult<CommandOutput>> + Send + 'a>>;

pub trait Executor<C>: Send + Sync {
    fn execute<'a>(&'a self, state: SharedShellState, command: &'a C) -> ExecutorFuture<'a>;
}

#[derive(Debug, Default)]
pub struct BootstrapExecutor;

impl Executor<ParsedCommand> for BootstrapExecutor {
    fn execute<'a>(
        &'a self,
        _state: SharedShellState,
        command: &'a ParsedCommand,
    ) -> ExecutorFuture<'a> {
        Box::pin(async move {
            match command {
                ParsedCommand::Empty | ParsedCommand::Exit => Ok(CommandOutput::success()),
                ParsedCommand::Raw(input) => Ok(CommandOutput::failure(
                    ExitCode::FAILURE,
                    format!("execution not implemented yet: {input}\n"),
                )),
            }
        })
    }
}
