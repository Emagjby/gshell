use std::fs;

use gshell::{
    completion::{ShellCompleter, ShellHinter},
    shell::ShellState,
};
use reedline::{Completer, Hinter, History, HistoryItem};

#[tokio::test]
async fn command_completion_includes_builtins() {
    let state = ShellState::shared().await.expect("state should initialize");
    let mut completer = ShellCompleter::new(state);

    let suggestions = completer.complete("ec", 2);
    let values = suggestions.into_iter().map(|s| s.value).collect::<Vec<_>>();

    assert!(values.iter().any(|v| v == "echo"));
}

#[tokio::test]
async fn env_completion_suggests_shell_variables() {
    let state = ShellState::shared().await.expect("state should initialize");
    state.write().await.set_env_var("GSHELL_DEMO", "1");

    let mut completer = ShellCompleter::new(state);
    let suggestions = completer.complete("$GSH", 4);
    let values = suggestions.into_iter().map(|s| s.value).collect::<Vec<_>>();

    assert!(values.iter().any(|v| v == "$GSHELL_DEMO"));
}

#[tokio::test]
async fn path_completion_suggests_matching_files() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    fs::write(dir.path().join("alpha.txt"), "x").expect("file should be writable");
    fs::write(dir.path().join("alpine.txt"), "x").expect("file should be writable");
    fs::write(dir.path().join("beta.txt"), "x").expect("file should be writable");

    let state = ShellState::shared().await.expect("state should initialize");
    state.write().await.set_cwd(dir.path().to_path_buf());

    let mut completer = ShellCompleter::new(state);
    let suggestions = completer.complete("./al", 4);
    let values = suggestions.into_iter().map(|s| s.value).collect::<Vec<_>>();

    assert!(values.iter().any(|v| v == "./alpha.txt"));
    assert!(values.iter().any(|v| v == "./alpine.txt"));
    assert!(!values.iter().any(|v| v == "./beta.txt"));
}

#[tokio::test]
async fn command_completion_reads_executables_from_path() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("temp dir should be created");
        let cmd = dir.path().join("demo-tool");
        fs::write(&cmd, "#!/bin/sh\n").expect("stub command should be writable");

        let mut perms = fs::metadata(&cmd)
            .expect("metadata should load")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&cmd, perms).expect("permissions should update");

        let state = ShellState::shared().await.expect("state should initialize");
        state
            .write()
            .await
            .set_env_var("PATH", dir.path().display().to_string());

        let mut completer = ShellCompleter::new(state);
        let suggestions = completer.complete("dem", 3);
        let values = suggestions.into_iter().map(|s| s.value).collect::<Vec<_>>();

        assert!(values.iter().any(|v| v == "demo-tool"));
    }
}

#[tokio::test]
async fn hinter_returns_suffix_for_history_match() {
    let state = ShellState::shared().await.expect("state should initialize");
    let history_path = state.read().await.history().path().to_path_buf();

    let mut history = reedline::FileBackedHistory::with_file(100, history_path)
        .expect("history should initialize");

    history
        .save(HistoryItem::from_command_line("echo hello world"))
        .expect("history entry should save");

    let mut hinter = ShellHinter::default();
    let hint = hinter.handle("echo h", 6, &history, false, "");

    assert_eq!(hint, "ello world");
}

#[tokio::test]
async fn hinter_returns_nothing_when_cursor_is_not_at_end() {
    let state = ShellState::shared().await.expect("state should initialize");
    let history_path = state.read().await.history().path().to_path_buf();

    let mut history = reedline::FileBackedHistory::with_file(100, history_path)
        .expect("history should initialize");

    history
        .save(HistoryItem::from_command_line("echo hello world"))
        .expect("history entry should save");

    let mut hinter = ShellHinter::default();
    let hint = hinter.handle("echo h", 2, &history, false, "");

    assert!(hint.is_empty());
}
