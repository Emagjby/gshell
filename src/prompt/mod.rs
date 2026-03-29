use std::pin::Pin;

use crate::shell::{SharedShellState, ShellResult};

pub type PromptFuture<'a> = Pin<Box<dyn Future<Output = ShellResult<String>> + Send + 'a>>;

pub trait PromptRenderer: Send + Sync {
    fn render_prompt<'a>(&'a self, state: SharedShellState) -> PromptFuture<'a>;
}

#[derive(Debug, Default)]
pub struct DefaultPrompt;

impl PromptRenderer for DefaultPrompt {
    fn render_prompt<'a>(&'a self, _state: SharedShellState) -> PromptFuture<'a> {
        Box::pin(async { Ok("$ ".to_string()) })
    }
}
