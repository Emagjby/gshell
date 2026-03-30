use gshell::{
    parser::Parser,
    runtime::{BootstrapExecutor, Executor},
    shell::{ExitCode, ShellAction, ShellState},
};

#[tokio::test]
async fn simple_command_substitution_works() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("echo $(echo hello)")
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "hello\n");
        }
        ShellAction::Exit(_) => panic!("echo should not exit"),
    }
}

#[tokio::test]
async fn nested_command_substitution_works() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("echo $(echo $(echo hi))")
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "hi\n");
        }
        ShellAction::Exit(_) => panic!("echo should not exit"),
    }
}

#[tokio::test]
async fn quoted_command_substitution_works() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("echo \"$(echo hello world)\"")
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
async fn command_substitution_trims_trailing_newlines() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("echo $(printf 'hello\\n\\n')")
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "hello\n");
        }
        ShellAction::Exit(_) => panic!("echo should not exit"),
    }
}

#[tokio::test]
async fn failed_nested_command_does_not_crash_substitution() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser.parse("echo $(false)").expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "\n");
        }
        ShellAction::Exit(_) => panic!("echo should not exit"),
    }
}

#[tokio::test]
async fn command_substitution_preserves_builtin_state_changes() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("echo $(cd / && pwd)")
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "/\n");
        }
        ShellAction::Exit(_) => panic!("echo should not exit"),
    }
}
