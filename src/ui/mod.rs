use std::sync::Arc;

use reedline::{FileBackedHistory, Reedline, Signal};

use crate::{
    history::{HistoryConfig, should_record_history_entry},
    parser::{ParsedCommand, Parser},
    prompt::{FallbackPromptRenderer, ReedlinePromptAdapter},
    runtime::Executor,
    shell::{ExitCode, SharedShellState, ShellAction, ShellError, ShellResult},
};

pub struct Repl<E> {
    line_editor: Reedline,
    core: ReplCore<E>,
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
        let history = build_history(state.clone()).await;

        let line_editor = match history {
            Ok(history) => Reedline::create().with_history(Box::new(history)),
            Err(err) => {
                eprintln!("warning: failed to initialize history: {err}");
                Reedline::create()
            }
        };

        Self {
            line_editor,
            core: ReplCore::new(executor),
        }
    }

    pub async fn run(&mut self, state: SharedShellState) -> ShellResult<()> {
        let renderer = Arc::new(FallbackPromptRenderer);
        let mut prompt = ReedlinePromptAdapter::new(renderer.clone());

        loop {
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
