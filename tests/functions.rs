use std::sync::{Arc, Mutex};
use std::{fs, time::Duration};

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
async fn assignment_prefix_is_scoped_to_function_invocation() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("show() { echo $NAME; }; NAME=gencho show; echo $NAME")
        .expect("parse should succeed");
    let result = executor
        .execute(state.clone(), &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "gencho\n\n");
            assert_eq!(state.read().await.env_var("NAME"), None);
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

#[tokio::test]
async fn function_sequence_streams_intermediate_output_when_sink_is_installed() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");
    let captured = Arc::new(Mutex::new(Vec::<String>::new()));
    let sink = captured.clone();
    state
        .write()
        .await
        .runtime_services_mut()
        .set_output_sink(Some(Arc::new(move |output| {
            if !output.stdout.is_empty() {
                sink.lock()
                    .expect("sink lock should not be poisoned")
                    .push(output.stdout.clone());
            }
        })));

    let parsed = parser
        .parse("greet() { echo hi; sleep 0.2; echo bye; }; greet")
        .expect("parse should succeed");

    let state_for_task = state.clone();
    let task = tokio::spawn(async move {
        executor
            .execute(state_for_task, &parsed)
            .await
            .expect("execution should succeed")
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    assert_eq!(
        captured
            .lock()
            .expect("sink lock should not be poisoned")
            .as_slice(),
        ["hi\n"]
    );
    assert!(
        !task.is_finished(),
        "function should still be waiting on sleep"
    );

    let result = task.await.expect("task should join successfully");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert!(output.stdout.is_empty());
        }
        ShellAction::Exit(_) => panic!("function execution should not exit"),
    }

    assert_eq!(
        captured
            .lock()
            .expect("sink lock should not be poisoned")
            .as_slice(),
        ["hi\n", "bye\n"]
    );
}

#[tokio::test]
async fn function_call_redirection_captures_external_output() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let out = dir.path().join("out.txt");

    let parsed = parser
        .parse(&format!(
            "greet() {{ printf 'hi\\n'; sleep 0.05; printf 'bye\\n'; }}; greet > {}",
            out.display()
        ))
        .expect("parse should succeed");
    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert!(output.stdout.is_empty());
            assert!(output.stderr.is_empty());
        }
        ShellAction::Exit(_) => panic!("function execution should not exit"),
    }

    let content = fs::read_to_string(out).expect("output file should be readable");
    assert_eq!(content, "hi\nbye\n");
}
