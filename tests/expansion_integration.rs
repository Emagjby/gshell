use std::fs;

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
async fn tilde_expansion_uses_home_environment_variable() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    let state = ShellState::shared()
        .await
        .expect("failed to create shell state");
    state
        .write()
        .await
        .set_env_var("HOME", dir.path().display().to_string());

    let parser = Parser::default();
    let executor = BootstrapExecutor;

    let parsed = parser.parse("echo ~").expect("parse should succeed");
    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, format!("{}\n", dir.path().display()));
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

#[tokio::test]
async fn star_glob_expands_matches_in_sorted_order() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    fs::write(dir.path().join("b.txt"), "").expect("file should be writable");
    fs::write(dir.path().join("a.txt"), "").expect("file should be writable");

    let state = ShellState::shared()
        .await
        .expect("failed to create shell state");
    state.write().await.set_cwd(dir.path().to_path_buf());

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let parsed = parser.parse("echo *.txt").expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "a.txt b.txt\n");
        }
        ShellAction::Exit(_) => panic!("printf should not exit"),
    }
}

#[tokio::test]
async fn question_glob_matches_single_character() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    fs::write(dir.path().join("a1.txt"), "").expect("file should be writable");
    fs::write(dir.path().join("ab.txt"), "").expect("file should be writable");
    fs::write(dir.path().join("long.txt"), "").expect("file should be writable");

    let state = ShellState::shared()
        .await
        .expect("failed to create shell state");
    state.write().await.set_cwd(dir.path().to_path_buf());

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let parsed = parser.parse("echo a?.txt").expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "a1.txt ab.txt\n");
        }
        ShellAction::Exit(_) => panic!("printf should not exit"),
    }
}

#[tokio::test]
async fn character_class_glob_matches_expected_files() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    fs::write(dir.path().join("a.txt"), "").expect("file should be writable");
    fs::write(dir.path().join("b.txt"), "").expect("file should be writable");
    fs::write(dir.path().join("c.txt"), "").expect("file should be writable");

    let state = ShellState::shared()
        .await
        .expect("failed to create shell state");
    state.write().await.set_cwd(dir.path().to_path_buf());

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let parsed = parser.parse("echo [ab].txt").expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "a.txt b.txt\n");
        }
        ShellAction::Exit(_) => panic!("printf should not exit"),
    }
}

#[tokio::test]
async fn quoted_wildcards_remain_literal() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    fs::write(dir.path().join("a.txt"), "").expect("file should be writable");

    let state = ShellState::shared()
        .await
        .expect("failed to create shell state");
    state.write().await.set_cwd(dir.path().to_path_buf());

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let parsed = parser.parse("echo '*.txt'").expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "*.txt\n");
        }
        ShellAction::Exit(_) => panic!("printf should not exit"),
    }
}

#[tokio::test]
async fn unmatched_glob_pattern_remains_literal() {
    let dir = tempfile::tempdir().expect("temp dir should be created");

    let state = ShellState::shared()
        .await
        .expect("failed to create shell state");
    state.write().await.set_cwd(dir.path().to_path_buf());

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let parsed = parser
        .parse("echo *.missing")
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "*.missing\n");
        }
        ShellAction::Exit(_) => panic!("printf should not exit"),
    }
}

#[tokio::test]
async fn variable_expansion_happens_before_globbing() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    fs::write(dir.path().join("via-var.txt"), "").expect("file should be writable");

    let state = ShellState::shared()
        .await
        .expect("failed to create shell state");
    {
        let mut guard = state.write().await;
        guard.set_cwd(dir.path().to_path_buf());
        guard.set_env_var("PATTERN", "*.txt");
    }

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let parsed = parser.parse("echo $PATTERN").expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "via-var.txt\n");
        }
        ShellAction::Exit(_) => panic!("printf should not exit"),
    }
}

#[tokio::test]
async fn command_substitution_happens_before_globbing() {
    let dir = tempfile::tempdir().expect("temp dir should be created");
    fs::write(dir.path().join("cmd-a.txt"), "").expect("file should be writable");
    fs::write(dir.path().join("cmd-b.txt"), "").expect("file should be writable");

    let state = ShellState::shared()
        .await
        .expect("failed to create shell state");
    state.write().await.set_cwd(dir.path().to_path_buf());

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let parsed = parser
        .parse("echo $(printf 'cmd-*.txt')")
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "cmd-a.txt cmd-b.txt\n");
        }
        ShellAction::Exit(_) => panic!("echo should not exit"),
    }
}
