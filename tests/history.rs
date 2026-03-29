use gshell::history::{HistoryConfig, should_record_history_entry};
use gshell::shell::HistoryState;
use gshell::{
    builtins::{Builtin, HistoryBuiltin},
    shell::{ShellAction, ShellState},
};

use std::path::PathBuf;

#[test]
fn history_path_resolution_produces_history_txt() {
    let config = HistoryConfig::resolve_default().expect("history path should resolve");
    let path = config.path().to_string_lossy();

    assert!(path.ends_with("gshell/history.txt"));
}

#[test]
fn blank_command_filtering_skips_empty_entries() {
    assert!(!should_record_history_entry(""));
    assert!(!should_record_history_entry("   "));
    assert!(should_record_history_entry("ls"));
    assert!(should_record_history_entry(" history "));
}

#[test]
fn history_state_stores_entries() {
    let mut history = HistoryState::new(PathBuf::from("/tmp/gshell-history-test"));
    history.push("echo hello");
    history.push("history");

    assert_eq!(
        history.entries(),
        &["echo hello".to_string(), "history".to_string()]
    );
}

#[tokio::test]
async fn history_builtin_outputs_entries() {
    let state = ShellState::shared().await.expect("state should initialize");

    {
        let mut guard = state.write().await;
        guard.history_mut().push("echo hello");
        guard.history_mut().push("history");
    }

    let builtin = HistoryBuiltin;
    let output = builtin
        .execute(state, &[])
        .await
        .expect("history builtin should execute");

    match output {
        ShellAction::Continue(command_output) => {
            assert!(command_output.stdout.contains("echo hello"));
            assert!(command_output.stdout.contains("history"));
        }
        ShellAction::Exit(_) => panic!("history builtin should not exit"),
    }
}
