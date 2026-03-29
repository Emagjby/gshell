use std::sync::{Arc, Mutex};

use gshell::{
    parser::ParsedCommand,
    runtime::{Executor, ExecutorFuture},
    shell::{CommandOutput, ExitCode, SharedShellState, ShellState},
    ui::{ReplCore, ReplFlow},
};
use reedline::Signal;

#[derive(Clone, Default)]
struct RecordingExecutor {
    calls: Arc<Mutex<Vec<ParsedCommand>>>,
}

impl RecordingExecutor {
    fn calls(&self) -> Vec<ParsedCommand> {
        self.calls.lock().expect("calls lock poisoned").clone()
    }
}

impl Executor<ParsedCommand> for RecordingExecutor {
    fn execute<'a>(
        &'a self,
        _state: SharedShellState,
        command: &'a ParsedCommand,
    ) -> ExecutorFuture<'a> {
        let calls = self.calls.clone();

        Box::pin(async move {
            calls
                .lock()
                .expect("calls lock poisoned")
                .push(command.clone());
            Ok(CommandOutput::success())
        })
    }
}

#[tokio::test]
async fn shell_launces_and_waits_for_input() {
    let executor = RecordingExecutor::default();
    let core = ReplCore::new(executor.clone());
    let state = ShellState::shared().expect("state should initialize");

    let flow = core
        .handle_signal(Signal::Success("echo hello".to_string()), state.clone())
        .await;

    assert_eq!(flow, ReplFlow::Continue);
    assert_eq!(
        executor.calls(),
        vec![ParsedCommand::Raw("echo hello".to_string())]
    );
    assert_eq!(
        state
            .read()
            .expect("state lock should be readable")
            .last_exit_status(),
        ExitCode::SUCCESS
    );
}

#[tokio::test]
async fn empty_line_redraws_prompt() {
    let executor = RecordingExecutor::default();
    let core = ReplCore::new(executor.clone());
    let state = ShellState::shared().expect("state should initialize");

    let flow = core
        .handle_signal(Signal::Success(String::new()), state.clone())
        .await;

    assert_eq!(flow, ReplFlow::Continue);
    assert!(executor.calls().is_empty());
    assert_eq!(
        state
            .read()
            .expect("state lock should be readable")
            .last_exit_status(),
        ExitCode::SUCCESS
    );
}

#[tokio::test]
async fn explicit_exit_terminates_session_cleanly() {
    let executor = RecordingExecutor::default();
    let core = ReplCore::new(executor.clone());
    let state = ShellState::shared().expect("state should initialize");

    let flow = core
        .handle_signal(Signal::Success("exit".to_string()), state.clone())
        .await;

    assert_eq!(flow, ReplFlow::Break);
    assert!(executor.calls().is_empty());
    assert_eq!(
        state
            .read()
            .expect("state lock should be readable")
            .last_exit_status(),
        ExitCode::SUCCESS
    );
}
