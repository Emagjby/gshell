use std::fs;

use gshell::{
    parser::Parser,
    runtime::{BootstrapExecutor, Executor},
    shell::{ExitCode, ShellAction, ShellState},
};

#[tokio::test]
async fn output_truncate_redirection_works() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let out = dir.path().join("out.txt");

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse(&format!("echo hello > {}", out.display()))
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert!(output.stdout.is_empty());
        }
        ShellAction::Exit(_) => panic!("echo should not exit"),
    }

    let content = fs::read_to_string(out).expect("output file should be readable");
    assert_eq!(content, "hello\n");
}

#[tokio::test]
async fn output_append_redirection_works() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let out = dir.path().join("out.txt");
    fs::write(&out, "first\n").expect("seed output file should be writable");

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse(&format!("echo second >> {}", out.display()))
        .expect("parse should succeed");

    let _ = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    let content = fs::read_to_string(out).expect("output file should be readable");
    assert_eq!(content, "first\nsecond\n");
}

#[tokio::test]
async fn stderr_truncate_redirection_works() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let err = dir.path().join("err.txt");

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse(&format!("type nope 2> {}", err.display()))
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert!(output.stderr.is_empty());
            assert_eq!(output.exit_code, ExitCode::FAILURE);
        }
        ShellAction::Exit(_) => panic!("type should not exit"),
    }

    let content = fs::read_to_string(err).expect("stderr file should be readable");
    assert!(content.contains("not found"));
}

#[tokio::test]
async fn stderr_append_redirection_works() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let err = dir.path().join("err.txt");
    fs::write(&err, "before\n").expect("seed stderr file should be writable");

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse(&format!("type nope 2>> {}", err.display()))
        .expect("parse should succeed");

    let _ = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    let content = fs::read_to_string(err).expect("stderr file should be readable");
    assert!(content.starts_with("before\n"));
    assert!(content.contains("not found"));
}

#[tokio::test]
async fn redirected_builtin_does_not_write_to_terminal_output_buffer() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let out = dir.path().join("out.txt");

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse(&format!("pwd > {}", out.display()))
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert!(output.stdout.is_empty());
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
        }
        ShellAction::Exit(_) => panic!("pwd should not exit"),
    }

    let content = fs::read_to_string(out).expect("output file should be readable");
    assert!(!content.trim().is_empty());
}
