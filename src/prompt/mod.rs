use std::{borrow::Cow, future::Future, pin::Pin, sync::Arc};

use reedline::PromptViMode;
pub use reedline::{DefaultPromptSegment, Prompt, PromptEditMode, PromptHistorySearch};

use crate::shell::{SharedShellState, ShellResult};

pub type PromptFuture<'a> = Pin<Box<dyn Future<Output = ShellResult<String>> + Send + 'a>>;

pub trait PromptRenderer: Send + Sync {
    fn render_prompt<'a>(&'a self, state: SharedShellState) -> PromptFuture<'a>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FallbackPromptRenderer;

impl PromptRenderer for FallbackPromptRenderer {
    fn render_prompt<'a>(&'a self, _state: SharedShellState) -> PromptFuture<'a> {
        Box::pin(async { Ok("$ ".to_string()) })
    }
}

pub struct ReedlinePromptAdapter<R> {
    renderer: Arc<R>,
    prompt: String,
}

impl<R> ReedlinePromptAdapter<R>
where
    R: PromptRenderer,
{
    pub fn new(renderer: Arc<R>) -> Self {
        Self {
            renderer,
            prompt: "$ ".to_string(),
        }
    }

    pub async fn refresh(&mut self, state: SharedShellState) {
        self.prompt = self
            .renderer
            .render_prompt(state)
            .await
            .unwrap_or_else(|_| "$ ".to_string());
    }
}

impl<R> Prompt for ReedlinePromptAdapter<R>
where
    R: PromptRenderer,
{
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.prompt.as_str())
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, edit_mode: PromptEditMode) -> Cow<'_, str> {
        match edit_mode {
            PromptEditMode::Vi(PromptViMode::Insert) => Cow::Borrowed(": "),
            _ => Cow::Borrowed(""),
        }
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("> ")
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        Cow::Owned(format!(
            "(history search: {}) ",
            history_search.term.as_str()
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::shell::ShellState;

    #[tokio::test]
    async fn test_fallback_prompt_renderer() {
        let renderer = FallbackPromptRenderer;
        let state = ShellState::shared().expect("state should initialize");

        let rendered = renderer
            .render_prompt(state)
            .await
            .expect("rendering should succeed");

        assert_eq!(rendered, "$ ");
    }
}
