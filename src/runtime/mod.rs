use std::pin::Pin;

use crate::shell::{CommandOutput, SharedShellState, ShellResult};

pub type ExecutorFuture<'a> = Pin<Box<dyn Future<Output = ShellResult<CommandOutput>> + Send + 'a>>;

pub trait Executor<C>: Send + Sync {
    fn execute<'a>(&'a self, state: SharedShellState, command: &'a C) -> ExecutorFuture<'a>;
}

#[derive(Debug, Default)]
pub struct Runtime;
