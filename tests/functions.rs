use gshell::{
    parser::Parser,
    runtime::{BootstrapExecutor, Executor},
    shell::{ExitCode, ShellAction, ShellState},
};

#[tokio::test]
async fn function_definition_and_invocation_work() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("greet() { echo hello; }; greet")
        .expect("parse should succeed");
    let result = executor
        .execute(state.clone(), &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "hello\n");
            assert!(state.read().await.functions().get("greet").is_some());
        }
        ShellAction::Exit(_) => panic!("function execution should not exit"),
    }
}

#[tokio::test]
async fn function_body_updates_environment_for_later_commands() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("setname() { NAME=gencho; }; setname; echo $NAME")
        .expect("parse should succeed");
    let result = executor
        .execute(state.clone(), &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "gencho\n");
            assert_eq!(state.read().await.env_var("NAME"), Some("gencho"));
        }
        ShellAction::Exit(_) => panic!("function execution should not exit"),
    }
}

#[tokio::test]
async fn function_body_sees_updated_last_exit_status() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("show_status() { false; echo $?; }; show_status")
        .expect("parse should succeed");
    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "1\n");
        }
        ShellAction::Exit(_) => panic!("function execution should not exit"),
    }
}

#[tokio::test]
async fn function_recursion_is_rejected() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("loop() { loop; }; loop")
        .expect("parse should succeed");
    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::FAILURE);
            assert!(output.stderr.contains("function recursion detected: loop"));
        }
        ShellAction::Exit(_) => panic!("function execution should not exit"),
    }
}

#[tokio::test]
async fn alias_takes_precedence_over_function_lookup() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");
    state.write().await.aliases_mut().set("run", "echo alias");

    let parsed = parser
        .parse("run() { echo function; }; run")
        .expect("parse should succeed");
    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "alias\n");
        }
        ShellAction::Exit(_) => panic!("function execution should not exit"),
    }
}

#[tokio::test]
async fn alias_can_expand_to_function_name() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("greet() { echo function; }; alias say=greet; say")
        .expect("parse should succeed");
    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "function\n");
        }
        ShellAction::Exit(_) => panic!("function execution should not exit"),
    }
}
