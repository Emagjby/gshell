use std::sync::Arc;

use reedline::{Reedline, Signal};

use crate::{
    parser::{ParsedCommand, Parser},
    prompt::{FallbackPromptRenderer, ReedlinePromptAdapter},
    runtime::Executor,
    shell::{ExitCode, SharedShellState, ShellResult},
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
    pub fn new(executor: E) -> Self {
        Self {
            line_editor: Reedline::create(),
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
                    state
                        .write()
                        .expect("shell state lock poisoned")
                        .set_last_exit_status(ExitCode::FAILURE);
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
            parser: Parser,
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
                        state
                            .write()
                            .expect("shell state lock poisoned")
                            .set_last_exit_status(ExitCode::FAILURE);
                        return ReplFlow::Continue;
                    }
                };

                if matches!(command, ParsedCommand::Empty) {
                    return ReplFlow::Continue;
                }

                if matches!(command, ParsedCommand::Exit) {
                    return ReplFlow::Break;
                }

                match self.executor.execute(state.clone(), &command).await {
                    Ok(output) => {
                        if !output.stdout.is_empty() {
                            print!("{}", output.stdout);
                        }

                        if !output.stderr.is_empty() {
                            eprint!("{}", output.stderr);
                        }

                        state
                            .write()
                            .expect("shell state lock poisoned")
                            .set_last_exit_status(output.exit_code);
                    }
                    Err(err) => {
                        eprintln!("{err}");
                        state
                            .write()
                            .expect("shell state lock poisoned")
                            .set_last_exit_status(ExitCode::FAILURE);
                    }
                }

                ReplFlow::Continue
            }
            Signal::CtrlC => {
                state
                    .write()
                    .expect("shell state lock poisoned")
                    .set_last_exit_status(ExitCode::FAILURE);
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
