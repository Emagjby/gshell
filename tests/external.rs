use std::fs;

use gshell::{
    parser::Parser,
    runtime::{BootstrapExecutor, Executor},
    shell::{ExitCode, ShellAction, ShellState},
};

#[tokio::test]
async fn known_system_command_executes_successfully() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser.parse("pwd").expect("parse should succeed");
    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
        }
        ShellAction::Exit(_) => panic!("pwd should not exit the shell"),
    }
}

#[tokio::test]
async fn command_not_found_returns_failure() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("definitely_not_a_real_command_12345")
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::FAILURE);
            assert!(output.stderr.contains("command not found"));
        }
        ShellAction::Exit(_) => panic!("unknown command should not exit the shell"),
    }
}

#[tokio::test]
async fn exit_code_propagates_from_external_command() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser.parse("false").expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert!(output.exit_code.is_failure());
        }
        ShellAction::Exit(_) => panic!("false should not exit the shell"),
    }
}

#[tokio::test]
async fn non_executable_path_entry_is_not_resolved_as_command() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let command_path = dir.path().join("demo-command");
    fs::write(&command_path, "#!/bin/sh\nexit 0\n").expect("stub command should be writable");

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");
    state
        .write()
        .await
        .set_env_var("PATH", dir.path().display().to_string());

    let parsed = parser.parse("demo-command").expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::FAILURE);
            assert!(output.stderr.contains("command not found"));
        }
        ShellAction::Exit(_) => panic!("unknown command should not exit the shell"),
    }
}
