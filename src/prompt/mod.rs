use std::{borrow::Cow, future::Future, pin::Pin};

use reedline::{DefaultPrompt as ReedlineDefaultPrompt, PromptViMode};
pub use reedline::{DefaultPromptSegment, Prompt, PromptEditMode, PromptHistorySearch};

use crate::shell::{SharedShellState, ShellResult};

pub type PromptFuture<'a> = Pin<Box<dyn Future<Output = ShellResult<String>> + Send + 'a>>;

pub trait PromptRenderer: Send + Sync {
    fn render_prompt<'a>(&'a self, state: SharedShellState) -> PromptFuture<'a>;
}

#[derive(Clone)]
pub struct DefaultPrompt {
    inner: ReedlineDefaultPrompt,
}

impl Default for DefaultPrompt {
    fn default() -> Self {
        Self {
            inner: ReedlineDefaultPrompt::new(
                DefaultPromptSegment::Basic("$ ".to_string()),
                DefaultPromptSegment::Empty,
            ),
        }
    }
}

impl DefaultPrompt {
    pub const fn new(
        left_prompt: DefaultPromptSegment,
        right_prompt: DefaultPromptSegment,
    ) -> Self {
        Self {
            inner: ReedlineDefaultPrompt::new(left_prompt, right_prompt),
        }
    }

    pub fn with_reedline_prompt(prompt: ReedlineDefaultPrompt) -> Self {
        Self { inner: prompt }
    }
}

impl PromptRenderer for DefaultPrompt {
    fn render_prompt<'a>(&'a self, _state: SharedShellState) -> PromptFuture<'a> {
        Box::pin(async {
            let left = self.inner.render_prompt_left();
            let indicator = self.render_prompt_indicator(PromptEditMode::Default);

            let mut prompt = String::with_capacity(left.len() + indicator.len() + 1);
            prompt.push_str(left.as_ref());

            if !left.is_empty() {
                prompt.push(' ');
            }

            prompt.push_str(indicator.as_ref());

            if !prompt.ends_with(' ') {
                prompt.push(' ');
            }

            Ok(prompt)
        })
    }
}

impl Prompt for DefaultPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        self.inner.render_prompt_left()
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        self.inner.render_prompt_right()
    }

    fn render_prompt_indicator(&self, edit_mode: PromptEditMode) -> Cow<'_, str> {
        match edit_mode {
            PromptEditMode::Vi(PromptViMode::Insert) => Cow::Borrowed(": "),
            _ => Cow::Borrowed(""),
        }
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        self.inner.render_prompt_multiline_indicator()
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        self.inner
            .render_prompt_history_search_indicator(history_search)
    }
}
