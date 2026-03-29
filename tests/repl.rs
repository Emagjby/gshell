use std::sync::{Arc, Mutex};

use gshell::{
    ast::SimpleCommand,
    parser::ParsedCommand,
    prompt::{FallbackPromptRenderer, Prompt, ReedlinePromptAdapter},
    runtime::{Executor, ExecutorFuture},
    shell::{CommandOutput, ExitCode, SharedShellState, ShellAction, ShellState},
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
            Ok(ShellAction::continue_with(CommandOutput::success()))
        })
    }
}

#[tokio::test]
async fn shell_launces_and_waits_for_input() {
    let executor = RecordingExecutor::default();
    let core = ReplCore::new(executor.clone());
    let state = ShellState::shared().await.expect("state should initialize");

    let flow = core
        .handle_signal(Signal::Success("echo hello".to_string()), state.clone())
        .await;

    assert_eq!(flow, ReplFlow::Continue);
    assert_eq!(
        executor.calls(),
        vec![ParsedCommand::Simple(SimpleCommand::new(vec![
            "echo".to_string(),
            "hello".to_string()
        ]))]
    );
    assert_eq!(state.read().await.last_exit_status(), ExitCode::SUCCESS);
}

#[tokio::test]
async fn empty_line_redraws_prompt() {
    let executor = RecordingExecutor::default();
    let core = ReplCore::new(executor.clone());
    let state = ShellState::shared().await.expect("state should initialize");

    let flow = core
        .handle_signal(Signal::Success(String::new()), state.clone())
        .await;

    assert_eq!(flow, ReplFlow::Continue);
    assert!(executor.calls().is_empty());
    assert_eq!(state.read().await.last_exit_status(), ExitCode::SUCCESS);
}

#[tokio::test]
async fn explicit_exit_terminates_session_cleanly() {
    let executor = gshell::runtime::BootstrapExecutor;
    let core = ReplCore::new(executor);
    let state = ShellState::shared().await.expect("state should initialize");

    let flow = core
        .handle_signal(Signal::Success("exit".to_string()), state.clone())
        .await;

    assert_eq!(flow, ReplFlow::Break);
    assert_eq!(state.read().await.last_exit_status(), ExitCode::SUCCESS);
}

#[tokio::test]
async fn prompt_shows_dollar_space() {
    let renderer = std::sync::Arc::new(FallbackPromptRenderer);
    let state = ShellState::shared().await.expect("state should initialize");
    let mut prompt = ReedlinePromptAdapter::new(renderer);

    prompt.refresh(state).await;

    assert_eq!(prompt.render_prompt_left(), "$ ");
}

#[tokio::test]
async fn prompt_still_available_after_command_execution() {
    let renderer = std::sync::Arc::new(FallbackPromptRenderer);
    let state = ShellState::shared().await.expect("state should initialize");
    let mut prompt = ReedlinePromptAdapter::new(renderer);

    prompt.refresh(state.clone()).await;

    let executor = RecordingExecutor::default();
    let core = ReplCore::new(executor);

    let flow = core
        .handle_signal(Signal::Success("echo hello".to_string()), state.clone())
        .await;

    assert_eq!(flow, ReplFlow::Continue);

    prompt.refresh(state).await;

    assert_eq!(prompt.render_prompt_left(), "$ ");
}

#[tokio::test]
async fn cd_followed_by_pwd_updates_shell_state() {
    let executor = gshell::runtime::BootstrapExecutor;
    let core = ReplCore::new(executor);
    let state = ShellState::shared().await.expect("state should initialize");

    let tmp = tempfile::tempdir().expect("temp dir should be created");
    let cmd = format!("cd {}", tmp.path().display());

    let flow = core
        .handle_signal(Signal::Success(cmd), state.clone())
        .await;

    assert_eq!(flow, ReplFlow::Continue);
    assert_eq!(state.read().await.cwd(), tmp.path());

    let flow = core
        .handle_signal(Signal::Success("pwd".to_string()), state.clone())
        .await;

    assert_eq!(flow, ReplFlow::Continue);
}

#[tokio::test]
async fn external_command_runs_through_repl_core() {
    let executor = gshell::runtime::BootstrapExecutor;
    let core = ReplCore::new(executor);
    let state = ShellState::shared().await.expect("state should initialize");

    let flow = core
        .handle_signal(Signal::Success("false".to_string()), state.clone())
        .await;

    assert_eq!(flow, ReplFlow::Continue);
    assert!(state.read().await.last_exit_status().is_failure());
}

#[tokio::test]
async fn echo_command_through_repl_core_updates_exit_status() {
    let executor = gshell::runtime::BootstrapExecutor;
    let core = ReplCore::new(executor);
    let state = ShellState::shared().await.expect("state should initialize");

    let flow = core
        .handle_signal(Signal::Success("echo hello".to_string()), state.clone())
        .await;

    assert_eq!(flow, ReplFlow::Continue);
    assert_eq!(state.read().await.last_exit_status(), ExitCode::SUCCESS);
}
