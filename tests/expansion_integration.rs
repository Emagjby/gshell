use gshell::{
    parser::Parser,
    runtime::{BootstrapExecutor, Executor},
    shell::{ExitCode, ShellAction, ShellState},
};

#[tokio::test]
async fn variable_expansion_reaches_echo_builtin() {
    let state = ShellState::shared()
        .await
        .expect("failed to create shell state");
    {
        let mut guard = state.write().await;
        guard.set_env_var("GREETING", "hello");
    }

    let parser = Parser::default();
    let executor = BootstrapExecutor;

    let parsed = parser
        .parse("echo $GREETING")
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
async fn status_expansion_reaches_echo_builtin() {
    let state = ShellState::shared()
        .await
        .expect("failed to create shell state");
    {
        let mut guard = state.write().await;
        guard.set_last_exit_status(ExitCode::new(7));
    }

    let parser = Parser::default();
    let executor = BootstrapExecutor;

    let parsed = parser.parse("echo $?").expect("parse should succeed");
    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "7\n");
        }
        ShellAction::Exit(_) => panic!("echo should not exit"),
    }
}
