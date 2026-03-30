use gshell::{
    builtins::{
        Builtin, BuiltinRegistry, CdBuiltin, ClearBuiltin, EchoBuiltin, ExitBuiltin,
        HistoryBuiltin, PwdBuiltin, TypeBuiltin,
    },
    parser::Parser,
    runtime::{BootstrapExecutor, Executor},
    shell::{ExitCode, ShellAction, ShellState},
};

#[test]
fn builtin_registry_lookup_works() {
    let registry = BuiltinRegistry::with_defaults();

    assert!(registry.contains("cd"));
    assert!(registry.contains("exit"));
    assert!(registry.contains("clear"));
    assert!(registry.contains("type"));
    assert!(registry.contains("echo"));
    assert!(registry.contains("pwd"));
    assert!(registry.contains("history"));
    assert!(registry.get("missing").is_none());
}

#[tokio::test]
async fn echo_builtin_outputs_joined_args() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = EchoBuiltin;

    let result = builtin
        .execute(state, &["hello".into(), "world".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "hello world\n");
        }
        ShellAction::Exit(_) => panic!("echo should not exit"),
    }
}

#[tokio::test]
async fn pwd_builtin_outputs_current_directory() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = PwdBuiltin;

    let result = builtin
        .execute(state.clone(), &[])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            let cwd = state.read().await.cwd().display().to_string();
            assert_eq!(output.stdout, format!("{cwd}\n"));
        }
        ShellAction::Exit(_) => panic!("pwd should not exit"),
    }
}

#[tokio::test]
async fn clear_builtin_emits_escape_sequence() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = ClearBuiltin;

    let result = builtin
        .execute(state, &[])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "\x1B[2J\x1B[H");
        }
        ShellAction::Exit(_) => panic!("clear should not exit"),
    }
}

#[tokio::test]
async fn cd_builtin_changes_shell_state_directory() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = CdBuiltin;

    let tmp = tempfile::tempdir().expect("temp dir should be created");
    let path = tmp.path().display().to_string();

    let result = builtin
        .execute(state.clone(), &[path])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            let cwd = state.read().await.cwd().to_path_buf();
            let expected = std::fs::canonicalize(tmp.path())
                .expect("temp dir path should canonicalize successfully");
            assert_eq!(cwd, expected);
        }
        ShellAction::Exit(_) => panic!("cd should not exit"),
    }
}

#[tokio::test]
async fn cd_builtin_rejects_too_many_arguments() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = CdBuiltin;

    let result = builtin
        .execute(state, &["a".into(), "b".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::FAILURE);
            assert!(output.stderr.contains("too many arguments"));
        }
        ShellAction::Exit(_) => panic!("cd should not exit"),
    }
}

#[tokio::test]
async fn type_builtin_reports_builtin() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = TypeBuiltin;

    let result = builtin
        .execute(state, &["echo".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert!(output.stdout.contains("echo is a shell builtin"));
        }
        ShellAction::Exit(_) => panic!("type should not exit"),
    }
}

#[tokio::test]
async fn type_builtin_fails_for_unknown_command() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = TypeBuiltin;

    let result = builtin
        .execute(state, &["definitely-not-real".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::FAILURE);
            assert!(output.stderr.contains("not found"));
        }
        ShellAction::Exit(_) => panic!("type should not exit"),
    }
}

#[tokio::test]
async fn exit_builtin_requests_shell_exit() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = ExitBuiltin;

    let result = builtin
        .execute(state, &["42".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Exit(code) => assert_eq!(code, ExitCode::new(42)),
        ShellAction::Continue(_) => panic!("exit should terminate the shell"),
    }
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
    let result = builtin
        .execute(state, &[])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert!(output.stdout.contains("echo hello"));
            assert!(output.stdout.contains("history"));
        }
        ShellAction::Exit(_) => panic!("history should not exit"),
    }
}

#[tokio::test]
async fn parsed_argv_reaches_echo_builtin_with_single_quotes() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("echo 'hello world'")
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "hello world\n");
        }
        ShellAction::Exit(_) => panic!("echo should not exit"),
    }
}

#[tokio::test]
async fn parsed_argv_reaches_echo_builtin_with_double_quotes() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("echo \"hello world\"")
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "hello world\n");
        }
        ShellAction::Exit(_) => panic!("echo should not exit"),
    }
}

#[tokio::test]
async fn assignment_only_command_updates_shell_environment() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser.parse("NAME=gencho").expect("parse should succeed");

    let result = executor
        .execute(state.clone(), &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(state.read().await.env_var("NAME"), Some("gencho"));
        }
        ShellAction::Exit(_) => panic!("assignment should not exit"),
    }
}

#[tokio::test]
async fn assignment_prefix_is_visible_to_builtin_arguments() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("NAME=gencho echo $NAME")
        .expect("parse should succeed");

    let result = executor
        .execute(state.clone(), &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "gencho\n");
        }
        ShellAction::Exit(_) => panic!("echo should not exit"),
    }
}
