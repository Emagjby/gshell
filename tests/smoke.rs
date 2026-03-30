use std::sync::Arc;

use gshell::{
    ast::{CommandNode, ShellExpr, SimpleCommand},
    builtins::{Builtin, BuiltinFuture, BuiltinRegistry},
    shell::{CommandOutput, ExitCode, SharedShellState, ShellAction, ShellState},
};

struct TestBuiltin;

impl Builtin for TestBuiltin {
    fn name(&self) -> &'static str {
        "test"
    }

    fn execute<'a>(&'a self, _state: SharedShellState, _args: &'a [String]) -> BuiltinFuture<'a> {
        Box::pin(async { Ok(ShellAction::continue_with(CommandOutput::success())) })
    }
}

#[test]
fn shell_state_initialization_defaults() {
    let state = ShellState::new().expect("shell state should initialize");

    assert_eq!(state.last_exit_status(), ExitCode::SUCCESS);
    assert!(state.cwd().is_absolute());
    assert!(state.history().entries().is_empty());
    assert!(state.aliases().get("missing").is_none());
    assert!(state.functions().get("missing").is_none());
    assert!(state.jobs().is_empty());
}

#[test]
fn shell_state_environment_read_and_write() {
    let mut state = ShellState::new().expect("shell state should initialize");

    state.set_env_var("GSHELL_TEST_KEY", "value-1");
    assert_eq!(state.env_var("GSHELL_TEST_KEY"), Some("value-1"));

    state.set_env_var("GSHELL_TEST_KEY", "value-2");
    assert_eq!(state.env_var("GSHELL_TEST_KEY"), Some("value-2"));

    let removed = state.remove_env_var("GSHELL_TEST_KEY");
    assert_eq!(removed.as_deref(), Some("value-2"));
    assert_eq!(state.env_var("GSHELL_TEST_KEY"), None);
}

#[test]
fn alias_store_supports_set_get_remove_and_sorted_entries() {
    let mut state = ShellState::new().expect("shell state should initialize");

    state.aliases_mut().set("z", "echo z");
    state.aliases_mut().set("a", "echo a");

    assert_eq!(state.aliases().get("a"), Some("echo a"));
    assert_eq!(
        state.aliases().entries(),
        vec![("a", "echo a"), ("z", "echo z")]
    );

    assert_eq!(state.aliases_mut().remove("a"), Some("echo a".to_string()));
    assert!(state.aliases().get("a").is_none());
}

#[test]
fn function_store_supports_set_get_remove_and_sorted_names() {
    let mut state = ShellState::new().expect("shell state should initialize");
    let body = ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(Vec::new())));

    state.functions_mut().set("z", body.clone());
    state.functions_mut().set("a", body.clone());

    assert!(state.functions().get("a").is_some());
    assert_eq!(
        state.functions().names(),
        vec!["a".to_string(), "z".to_string()]
    );

    assert!(state.functions_mut().remove("a").is_some());
    assert!(state.functions().get("a").is_none());
}

#[test]
fn shell_state_last_exit_status_updates() {
    let mut state = ShellState::new().expect("shell state should initialize");

    assert_eq!(state.last_exit_status(), ExitCode::SUCCESS);

    state.set_last_exit_status(ExitCode::new(42));
    assert_eq!(state.last_exit_status(), ExitCode::new(42));
}

#[test]
fn builtin_registry_basics() {
    let mut registry = BuiltinRegistry::new();

    assert!(registry.is_empty());

    registry.register(Arc::new(TestBuiltin));

    assert_eq!(registry.len(), 1);
    assert!(registry.contains("test"));
    assert!(registry.get("test").is_some());
    assert!(registry.get("missing").is_none());
}
