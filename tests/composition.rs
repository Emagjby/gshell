use gshell::{
    parser::Parser,
    runtime::{BootstrapExecutor, Executor},
    shell::{ExitCode, ShellAction, ShellState},
};

#[tokio::test]
async fn two_command_pipeline_passes_output_to_next_command() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("echo hello | cat")
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "hello\n");
            assert!(output.stderr.is_empty());
        }
        ShellAction::Exit(_) => panic!("pipeline should not exit the shell"),
    }
}

#[tokio::test]
async fn multi_command_pipeline_passes_through_all_segments() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("echo hello | cat | cat")
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "hello\n");
            assert!(output.stderr.is_empty());
        }
        ShellAction::Exit(_) => panic!("pipeline should not exit the shell"),
    }
}

#[tokio::test]
async fn and_if_runs_rhs_when_lhs_succeeds() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("true && echo ran")
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "ran\n");
        }
        ShellAction::Exit(_) => panic!("boolean chain should not exit the shell"),
    }
}

#[tokio::test]
async fn and_if_skips_rhs_when_lhs_fails() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("false && echo should_not_run")
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert!(output.exit_code.is_failure());
            assert!(output.stdout.is_empty());
        }
        ShellAction::Exit(_) => panic!("boolean chain should not exit the shell"),
    }
}

#[tokio::test]
async fn or_if_skips_rhs_when_lhs_succeeds() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("true || echo should_not_run")
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
        ShellAction::Exit(_) => panic!("boolean chain should not exit the shell"),
    }
}

#[tokio::test]
async fn or_if_runs_rhs_when_lhs_fails() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("false || echo recovered")
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "recovered\n");
        }
        ShellAction::Exit(_) => panic!("boolean chain should not exit the shell"),
    }
}

#[tokio::test]
async fn semicolon_executes_commands_in_order() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("echo first ; echo second")
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "first\nsecond\n");
        }
        ShellAction::Exit(_) => panic!("sequence should not exit the shell"),
    }
}
