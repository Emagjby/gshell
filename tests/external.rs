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
        .execute(state.clone(), &parsed)
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
        .execute(state.clone(), &parsed)
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
        .execute(state.clone(), &parsed)
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
        .execute(state.clone(), &parsed)
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
async fn assignment_prefix_updates_path_for_external_resolution() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let command_path = dir.path().join("demo-command");
    fs::write(&command_path, "#!/bin/sh\nprintf 'ok\\n'\n")
        .expect("stub command should be writable");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&command_path)
            .expect("stub command metadata should load")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&command_path, permissions)
            .expect("stub command permissions should update");
    }

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");
    let expected_path = dir.path().display().to_string();

    let parsed = parser
        .parse(&format!("PATH={expected_path} demo-command"))
        .expect("parse should succeed");

    let result = executor
        .execute(state.clone(), &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(
                state.read().await.env_var("PATH"),
                Some(expected_path.as_str())
            );
        }
        ShellAction::Exit(_) => panic!("external command should not exit the shell"),
    }
}
