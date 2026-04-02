pub mod highlighter;
pub mod validator;

use std::{
    io::{self, Write},
    sync::{Arc, RwLock},
};

use nu_ansi_term::Style;
use reedline::{
    ColumnarMenu, Completer, EditCommand, Editor, FileBackedHistory, KeyCode, KeyModifiers, Menu,
    MenuBuilder, MenuEvent, Painter, Reedline, ReedlineEvent, ReedlineMenu, Signal, Suggestion, Vi,
    default_vi_insert_keybindings, default_vi_normal_keybindings,
};

use crate::{
    completion::{ShellCompleter, ShellHinter},
    history::{HistoryConfig, should_record_history_entry},
    parser::{ParsedCommand, Parser},
    prompt::{ConfiguredPromptRenderer, ReedlinePromptAdapter},
    runtime::Executor,
    shell::{ExitCode, SharedShellState, ShellAction, ShellError, ShellResult},
    ui::{
        highlighter::{HighlighterPalette, ShellHighlighter},
        validator::ParserValidator,
    },
};

pub struct Repl<E> {
    line_editor: Reedline,
    core: ReplCore<E>,
    menu_prompt: Arc<RwLock<String>>,
}

struct PromptAwareColumnarMenu {
    inner: ColumnarMenu,
    prompt: Arc<RwLock<String>>,
    indicator: String,
}

impl PromptAwareColumnarMenu {
    fn new(inner: ColumnarMenu, prompt: Arc<RwLock<String>>) -> Self {
        Self {
            inner,
            prompt,
            indicator: String::new(),
        }
    }

    fn refresh_indicator(&mut self) {
        let prompt = self
            .prompt
            .read()
            .expect("menu prompt lock should not be poisoned");
        self.indicator = format!("{prompt}| ");
    }
}

impl Menu for PromptAwareColumnarMenu {
    fn settings(&self) -> &reedline::MenuSettings {
        self.inner.settings()
    }

    fn indicator(&self) -> &str {
        &self.indicator
    }

    fn is_active(&self) -> bool {
        self.inner.is_active()
    }

    fn menu_event(&mut self, event: MenuEvent) {
        if matches!(event, MenuEvent::Activate(_) | MenuEvent::Edit(_)) {
            self.refresh_indicator();
        }
        self.inner.menu_event(event);
    }

    fn can_quick_complete(&self) -> bool {
        self.inner.can_quick_complete()
    }

    fn can_partially_complete(
        &mut self,
        values_updated: bool,
        editor: &mut Editor,
        completer: &mut dyn Completer,
    ) -> bool {
        self.inner
            .can_partially_complete(values_updated, editor, completer)
    }

    fn update_values(&mut self, editor: &mut Editor, completer: &mut dyn Completer) {
        self.inner.update_values(editor, completer);
    }

    fn update_working_details(
        &mut self,
        editor: &mut Editor,
        completer: &mut dyn Completer,
        painter: &Painter,
    ) {
        self.inner
            .update_working_details(editor, completer, painter);
    }

    fn replace_in_buffer(&self, editor: &mut Editor) {
        self.inner.replace_in_buffer(editor);
    }

    fn menu_required_lines(&self, terminal_columns: u16) -> u16 {
        self.inner.menu_required_lines(terminal_columns)
    }

    fn menu_string(&self, available_lines: u16, use_ansi_coloring: bool) -> String {
        self.inner.menu_string(available_lines, use_ansi_coloring)
    }

    fn min_rows(&self) -> u16 {
        self.inner.min_rows()
    }

    fn get_values(&self) -> &[Suggestion] {
        self.inner.get_values()
    }

    fn set_cursor_pos(&mut self, cursor_pos: (u16, u16)) {
        self.inner.set_cursor_pos(cursor_pos);
    }
}

pub struct ReplCore<E> {
    parser: Parser,
    executor: E,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplFlow {
    Continue,
    Break,
}

impl<E> Repl<E>
where
    E: Executor<ParsedCommand>,
{
    pub async fn new(executor: E, state: SharedShellState) -> Self {
        state
            .write()
            .await
            .runtime_services_mut()
            .set_output_sink(Some(Arc::new(|output| {
                if !output.stdout.is_empty() {
                    print!("{}", output.stdout);
                }

                if !output.stderr.is_empty() {
                    eprint!("{}", output.stderr);
                }
            })));

        let menu_prompt = Arc::new(RwLock::new(String::new()));
        let history = build_history(state.clone()).await;
        let highlighter_palette = {
            let guard = state.read().await;
            let config = guard.runtime_services().highlighter_config();
            HighlighterPalette::new(
                config.command_color(),
                config.builtin_color(),
                config.argument_color(),
                config.flag_color(),
                config.operator_color(),
                config.redirect_color(),
            )
        };
        let hint_style = {
            let guard = state.read().await;
            let config = guard.runtime_services().highlighter_config();
            Style::new().fg(config.hint_color())
        };

        let completer = Box::new(ShellCompleter::new(state.clone()));
        let hinter = Box::new(ShellHinter::default().with_style(hint_style));

        let completion_menu = Box::new(PromptAwareColumnarMenu::new(
            ColumnarMenu::default()
                .with_name("completion_menu")
                .with_text_style(Style::new()),
            menu_prompt.clone(),
        ));

        let mut insert_keybindings = default_vi_insert_keybindings();
        let normal_keybindings = default_vi_normal_keybindings();

        insert_keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Tab,
            ReedlineEvent::UntilFound(vec![
                ReedlineEvent::Menu("completion_menu".to_string()),
                ReedlineEvent::MenuNext,
            ]),
        );

        insert_keybindings.add_binding(
            KeyModifiers::SHIFT,
            KeyCode::BackTab,
            ReedlineEvent::MenuPrevious,
        );

        insert_keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Right,
            ReedlineEvent::UntilFound(vec![
                ReedlineEvent::HistoryHintComplete,
                ReedlineEvent::MenuRight,
                ReedlineEvent::Edit(vec![EditCommand::MoveRight { select: false }]),
            ]),
        );

        let edit_mode = Box::new(Vi::new(insert_keybindings, normal_keybindings));

        let base_editor = Reedline::create()
            .with_validator(Box::new(ParserValidator::default()))
            .with_completer(completer)
            .with_hinter(hinter)
            .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
            .with_edit_mode(edit_mode)
            .with_highlighter(Box::new(ShellHighlighter::new(highlighter_palette)));

        let line_editor = match history {
            Ok(history) => base_editor.with_history(Box::new(history)),
            Err(err) => {
                eprintln!("warning: failed to initialize history: {err}");
                base_editor
            }
        };

        Self {
            line_editor,
            core: ReplCore::new(executor),
            menu_prompt,
        }
    }

    pub async fn run(&mut self, state: SharedShellState) -> ShellResult<()> {
        crate::runtime::initialize_interactive_shell().await?;
        let _cursor_shape = BlockCursorGuard::new().ok();

        let renderer = Arc::new(ConfiguredPromptRenderer::new());
        let mut prompt =
            ReedlinePromptAdapter::with_menu_prompt(renderer, self.menu_prompt.clone());

        loop {
            crate::runtime::refresh_job_statuses(state.clone()).await?;
            prompt.refresh(state.clone()).await;

            let signal = match self.line_editor.read_line(&prompt) {
                Ok(signal) => signal,
                Err(err) => {
                    eprintln!("reedline error: {err}");
                    state.write().await.set_last_exit_status(ExitCode::FAILURE);
                    continue;
                }
            };

            if matches!(
                self.core.handle_signal(signal, state.clone()).await,
                ReplFlow::Break
            ) {
                break;
            }
        }

        Ok(())
    }
}

impl<E> ReplCore<E>
where
    E: Executor<ParsedCommand>,
{
    pub fn new(executor: E) -> Self {
        Self {
            parser: Parser::default(),
            executor,
        }
    }

    pub async fn handle_signal(&self, signal: Signal, state: SharedShellState) -> ReplFlow {
        match signal {
            Signal::Success(buf) => {
                let command = match self.parser.parse(&buf) {
                    Ok(cmd) => cmd,
                    Err(err) => {
                        eprintln!("{err}");
                        state.write().await.set_last_exit_status(ExitCode::FAILURE);
                        return ReplFlow::Continue;
                    }
                };

                if matches!(command, ParsedCommand::Empty) {
                    return ReplFlow::Continue;
                }

                if should_record_history_entry(&buf) {
                    state
                        .write()
                        .await
                        .history_mut()
                        .push(buf.trim().to_string());
                }

                match self.executor.execute(state.clone(), &command).await {
                    Ok(ShellAction::Continue(output)) => {
                        if !output.stdout.is_empty() {
                            print!("{}", output.stdout);
                        }

                        if !output.stderr.is_empty() {
                            eprint!("{}", output.stderr);
                        }

                        state.write().await.set_last_exit_status(output.exit_code);
                    }
                    Ok(ShellAction::Exit(code)) => {
                        state.write().await.set_last_exit_status(code);
                        return ReplFlow::Break;
                    }
                    Err(err) => {
                        eprintln!("{err}");
                        state.write().await.set_last_exit_status(ExitCode::FAILURE);
                    }
                }

                ReplFlow::Continue
            }
            Signal::CtrlC => {
                state.write().await.set_last_exit_status(ExitCode::FAILURE);
                println!();
                ReplFlow::Continue
            }
            Signal::CtrlD => {
                println!();
                ReplFlow::Break
            }
        }
    }
}

async fn build_history(state: SharedShellState) -> ShellResult<FileBackedHistory> {
    let config = HistoryConfig::resolve_default()?;
    config.ensure_parent_dir()?;

    let history = FileBackedHistory::with_file(1_000, config.path().to_path_buf())
        .map_err(|err| ShellError::message(format!("history init failed: {err}")))?;

    let entries = std::fs::read_to_string(config.path())
        .map(|content| content.lines().map(ToOwned::to_owned).collect::<Vec<_>>())
        .unwrap_or_default();

    state.write().await.history_mut().set_entries(entries);

    Ok(history)
}

struct BlockCursorGuard;

impl BlockCursorGuard {
    fn new() -> io::Result<Self> {
        set_block_cursor(&mut io::stderr())?;
        Ok(Self)
    }
}

impl Drop for BlockCursorGuard {
    fn drop(&mut self) {
        let _ = set_block_cursor(&mut io::stderr());
    }
}

fn set_block_cursor(writer: &mut impl Write) -> io::Result<()> {
    writer.write_all(b"\x1b[2 q")?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::set_block_cursor;

    #[test]
    fn block_cursor_escape_sequence_matches_decsusr_block_shape() {
        let mut output = Vec::new();

        set_block_cursor(&mut output).expect("cursor sequence should be written");

        assert_eq!(output, b"\x1b[2 q");
    }
}
