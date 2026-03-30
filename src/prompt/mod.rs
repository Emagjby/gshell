use std::{
    borrow::Cow,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, RwLock},
};

use reedline::PromptViMode;
pub use reedline::{Prompt, PromptEditMode, PromptHistorySearch};
use tokio::process::Command;

use crate::{
    config::PromptMode,
    shell::{SharedShellState, ShellError, ShellResult},
};

pub type PromptFuture<'a> = Pin<Box<dyn Future<Output = ShellResult<PromptFrame>> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptFrame {
    pub insert_prompt: String,
    pub right_prompt: String,
    pub normal_indicator: String,
    pub multiline_prompt: String,
}

impl Default for PromptFrame {
    fn default() -> Self {
        Self {
            insert_prompt: "$ ".to_string(),
            right_prompt: String::new(),
            normal_indicator: ": ".to_string(),
            multiline_prompt: "> ".to_string(),
        }
    }
}

pub trait PromptRenderer: Send + Sync {
    fn render_frame<'a>(&'a self, state: SharedShellState) -> PromptFuture<'a>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FallbackPromptRenderer;

impl PromptRenderer for FallbackPromptRenderer {
    fn render_frame<'a>(&'a self, _state: SharedShellState) -> PromptFuture<'a> {
        Box::pin(async { Ok(PromptFrame::default()) })
    }
}

#[derive(Debug, Clone)]
pub struct StarshipPromptRenderer {
    binary: String,
}

impl Default for StarshipPromptRenderer {
    fn default() -> Self {
        Self {
            binary: "starship".to_string(),
        }
    }
}

impl StarshipPromptRenderer {
    pub fn new(binary: impl Into<String>) -> Self {
        Self {
            binary: binary.into(),
        }
    }

    async fn render_left_prompt(&self, cwd: PathBuf, status: u8) -> ShellResult<String> {
        let output = Command::new(&self.binary)
            .arg("prompt")
            .arg(format!("--status={status}"))
            .current_dir(cwd)
            .output()
            .await
            .map_err(|err| ShellError::message(format!("starship launch failed: {err}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let reason = if stderr.is_empty() {
                "starship prompt failed".to_string()
            } else {
                format!("starship prompt failed: {stderr}")
            };

            return Err(ShellError::message(reason));
        }

        let rendered = String::from_utf8_lossy(&output.stdout)
            .trim_end_matches(['\n', '\r'])
            .to_string();

        if rendered.is_empty() {
            return Err(ShellError::message("starship prompt returned empty output"));
        }

        Ok(rendered)
    }
}

impl PromptRenderer for StarshipPromptRenderer {
    fn render_frame<'a>(&'a self, state: SharedShellState) -> PromptFuture<'a> {
        Box::pin(async move {
            let (cwd, status) = {
                let guard = state.read().await;
                (guard.cwd().to_path_buf(), guard.last_exit_status().as_u8())
            };

            let insert_prompt = self.render_left_prompt(cwd, status).await?;

            Ok(PromptFrame {
                insert_prompt,
                right_prompt: String::new(),
                normal_indicator: ": ".to_string(),
                multiline_prompt: "> ".to_string(),
            })
        })
    }
}

#[derive(Debug, Clone)]
pub struct ConfiguredPromptRenderer {
    fallback: FallbackPromptRenderer,
}

impl Default for ConfiguredPromptRenderer {
    fn default() -> Self {
        Self {
            fallback: FallbackPromptRenderer,
        }
    }
}

impl ConfiguredPromptRenderer {
    pub fn new() -> Self {
        Self::default()
    }
}

impl PromptRenderer for ConfiguredPromptRenderer {
    fn render_frame<'a>(&'a self, state: SharedShellState) -> PromptFuture<'a> {
        Box::pin(async move {
            let config = {
                let guard = state.read().await;
                guard.runtime_services().prompt_config().clone()
            };

            match config.mode() {
                PromptMode::Internal => self.fallback.render_frame(state).await,
                PromptMode::Starship | PromptMode::Auto => {
                    let starship = StarshipPromptRenderer::new(config.starship_binary());

                    match starship.render_frame(state.clone()).await {
                        Ok(frame) => Ok(frame),
                        Err(_) => self.fallback.render_frame(state).await,
                    }
                }
            }
        })
    }
}

pub struct ReedlinePromptAdapter<R> {
    renderer: Arc<R>,
    frame: PromptFrame,
    menu_prompt: Arc<RwLock<String>>,
}

impl<R> ReedlinePromptAdapter<R>
where
    R: PromptRenderer,
{
    pub fn new(renderer: Arc<R>) -> Self {
        Self::with_menu_prompt(renderer, Arc::new(RwLock::new(String::new())))
    }

    pub fn with_menu_prompt(renderer: Arc<R>, menu_prompt: Arc<RwLock<String>>) -> Self {
        let frame = PromptFrame::default();
        *menu_prompt
            .write()
            .expect("menu prompt lock should not be poisoned") = frame.insert_prompt.clone();

        Self {
            renderer,
            frame,
            menu_prompt,
        }
    }

    pub async fn refresh(&mut self, state: SharedShellState) {
        self.frame = self
            .renderer
            .render_frame(state)
            .await
            .unwrap_or_else(|_| PromptFrame::default());

        *self
            .menu_prompt
            .write()
            .expect("menu prompt lock should not be poisoned") = self.frame.insert_prompt.clone();
    }
}

impl<R> Prompt for ReedlinePromptAdapter<R>
where
    R: PromptRenderer,
{
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed("\n\n")
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.frame.right_prompt.as_str())
    }

    fn render_prompt_indicator(&self, edit_mode: PromptEditMode) -> Cow<'_, str> {
        match edit_mode {
            PromptEditMode::Vi(PromptViMode::Normal) => Cow::Owned(format!(
                "{}{}",
                self.frame.insert_prompt, self.frame.normal_indicator
            )),
            _ => Cow::Borrowed(self.frame.insert_prompt.as_str()),
        }
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.frame.multiline_prompt.as_str())
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
    use std::sync::Arc;

    use super::*;
    use crate::{
        config::{PromptConfig, PromptMode},
        shell::ShellState,
    };

    #[tokio::test]
    async fn fallback_prompt_renderer_returns_default_frame() {
        let renderer = FallbackPromptRenderer;
        let state = ShellState::shared().await.expect("state should initialize");

        let frame = renderer
            .render_frame(state)
            .await
            .expect("fallback rendering should succeed");

        assert_eq!(frame, PromptFrame::default());
    }

    #[tokio::test]
    async fn configured_renderer_respects_internal_mode() {
        let state = ShellState::shared().await.expect("state should initialize");
        {
            let mut guard = state.write().await;
            guard
                .runtime_services_mut()
                .set_prompt_config(PromptConfig::new(PromptMode::Internal));
        }

        let renderer = ConfiguredPromptRenderer::new();
        let frame = renderer
            .render_frame(state)
            .await
            .expect("configured renderer should succeed");

        assert_eq!(frame.insert_prompt, "$ ");
        assert_eq!(frame.normal_indicator, ": ");
        assert_eq!(frame.multiline_prompt, "> ");
    }

    #[tokio::test]
    async fn configured_renderer_falls_back_when_starship_is_missing() {
        let state = ShellState::shared().await.expect("state should initialize");
        {
            let mut guard = state.write().await;
            guard.runtime_services_mut().set_prompt_config(
                PromptConfig::new(PromptMode::Starship)
                    .with_starship_binary("definitely-not-a-real-starship-binary"),
            );
        }

        let renderer = ConfiguredPromptRenderer::new();
        let frame = renderer
            .render_frame(state)
            .await
            .expect("configured renderer should still succeed via fallback");

        assert_eq!(frame, PromptFrame::default());
    }

    #[tokio::test]
    async fn adapter_uses_insert_and_normal_prompt_parts() {
        let renderer = Arc::new(ConfiguredPromptRenderer::new());
        let state = ShellState::shared().await.expect("state should initialize");
        {
            let mut guard = state.write().await;
            guard
                .runtime_services_mut()
                .set_prompt_config(PromptConfig::new(PromptMode::Internal));
        }
        let menu_prompt = Arc::new(RwLock::new(String::new()));
        let mut adapter = ReedlinePromptAdapter::with_menu_prompt(renderer, menu_prompt.clone());

        adapter.refresh(state).await;

        assert_eq!(adapter.render_prompt_left(), "\n\n");
        assert_eq!(
            adapter.render_prompt_indicator(PromptEditMode::Vi(PromptViMode::Insert)),
            "$ "
        );
        assert_eq!(
            adapter.render_prompt_indicator(PromptEditMode::Vi(PromptViMode::Normal)),
            "$ : "
        );
        assert_eq!(adapter.render_prompt_multiline_indicator(), "> ");
        assert_eq!(
            menu_prompt
                .read()
                .expect("menu prompt lock should not be poisoned")
                .as_str(),
            "$ "
        );
    }
}
