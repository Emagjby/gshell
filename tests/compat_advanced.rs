use std::{fs, process::Stdio};

use gshell::{
    parser::Parser,
    runtime::{BootstrapExecutor, Executor},
    shell::{ShellAction, ShellState},
};
use tokio::process::Command;

#[derive(Debug, PartialEq, Eq)]
struct ShellSnapshot {
    code: u8,
    stdout: String,
    stderr: String,
}

#[tokio::test]
async fn bash_compat_for_command_substitution_then_glob() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    fs::write(dir.path().join("compat-a.txt"), "").expect("file should be writable");
    fs::write(dir.path().join("compat-b.txt"), "").expect("file should be writable");

    let command = "echo $(printf 'compat-*.txt')";
    let gshell = run_gshell(command, dir.path()).await;
    let bash = run_bash(command, dir.path()).await;

    assert_eq!(gshell, bash);
}

#[tokio::test]
async fn bash_compat_for_alias_function_composition() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let command = "shopt -s expand_aliases\ngreet() { echo hi; }\nalias say=greet\nsay";

    let gshell = run_gshell("greet() { echo hi; }; alias say=greet; say", dir.path()).await;
    let bash = run_bash(command, dir.path()).await;

    assert_eq!(gshell, bash);
}

#[tokio::test]
async fn bash_compat_for_unquoted_heredoc_expansions() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let command = "VALUE=world; out=$(cat <<EOF\n$(printf 'hello') $VALUE\nEOF\n); echo \"$out\"";

    let gshell = run_gshell(command, dir.path()).await;
    let bash = run_bash(command, dir.path()).await;

    assert_eq!(gshell, bash);
}

async fn run_gshell(command: &str, cwd: &std::path::Path) -> ShellSnapshot {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");
    state.write().await.set_cwd(cwd.to_path_buf());

    let parsed = parser.parse(command).expect("parse should succeed");
    let action = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match action {
        ShellAction::Continue(output) => ShellSnapshot {
            code: output.exit_code.as_u8(),
            stdout: output.stdout,
            stderr: output.stderr,
        },
        ShellAction::Exit(code) => ShellSnapshot {
            code: code.as_u8(),
            stdout: String::new(),
            stderr: String::new(),
        },
    }
}

async fn run_bash(command: &str, cwd: &std::path::Path) -> ShellSnapshot {
    let output = Command::new("bash")
        .arg("-lc")
        .arg(command)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .output()
        .await
        .expect("bash should execute");

    ShellSnapshot {
        code: output.status.code().unwrap_or(1) as u8,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}
