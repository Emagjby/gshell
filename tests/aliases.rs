use gshell::{
    parser::Parser,
    runtime::{BootstrapExecutor, Executor},
    shell::{ExitCode, ShellAction, ShellState},
};

#[tokio::test]
async fn alias_expands_in_command_position() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");
    state.write().await.aliases_mut().set("ll", "echo hello");

    let parsed = parser.parse("ll world").expect("parse should succeed");
    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "hello world\n");
        }
        ShellAction::Exit(_) => panic!("alias expansion should not exit"),
    }
}

#[tokio::test]
async fn alias_recursion_stops_without_looping_forever() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");
    {
        let mut guard = state.write().await;
        guard.aliases_mut().set("a", "b");
        guard.aliases_mut().set("b", "a");
    }

    let parsed = parser.parse("a").expect("parse should succeed");
    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::FAILURE);
            assert!(output.stderr.contains("command not found: a"));
        }
        ShellAction::Exit(_) => panic!("alias expansion should not exit"),
    }
}

#[tokio::test]
async fn quoted_command_name_does_not_trigger_alias_expansion() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");
    state.write().await.aliases_mut().set("ll", "echo hello");

    let parsed = parser.parse("'ll' world").expect("parse should succeed");
    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::FAILURE);
            assert!(output.stderr.contains("command not found: ll"));
        }
        ShellAction::Exit(_) => panic!("quoted command should not exit"),
    }
}

#[tokio::test]
async fn quoted_arguments_inside_alias_are_preserved() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");
    state
        .write()
        .await
        .aliases_mut()
        .set("say", "echo \"hello world\"");

    let parsed = parser.parse("say").expect("parse should succeed");
    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "hello world\n");
        }
        ShellAction::Exit(_) => panic!("alias expansion should not exit"),
    }
}
