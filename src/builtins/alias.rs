use crate::{
    builtins::{Builtin, BuiltinFuture},
    shell::{CommandOutput, ExitCode, SharedShellState, ShellAction},
};

pub struct AliasBuiltin;

impl Builtin for AliasBuiltin {
    fn name(&self) -> &'static str {
        "alias"
    }

    fn execute<'a>(&'a self, state: SharedShellState, args: &'a [String]) -> BuiltinFuture<'a> {
        Box::pin(async move {
            if args.is_empty() {
                let guard = state.read().await;
                let mut stdout = String::new();

                for (name, value) in guard.aliases().entries() {
                    stdout.push_str(&format!("alias {name}='{}'\n", format_alias_value(value)));
                }

                return Ok(ShellAction::continue_with(CommandOutput {
                    exit_code: ExitCode::SUCCESS,
                    stdout,
                    stderr: String::new(),
                }));
            }

            let mut stdout = String::new();
            let mut stderr = String::new();
            let mut success = true;

            for arg in args {
                if let Some((name, value)) = arg.split_once('=') {
                    if !is_valid_alias_name(name) {
                        success = false;
                        stderr.push_str(&format!("alias: invalid alias name: {name}\n"));
                        continue;
                    }

                    state
                        .write()
                        .await
                        .aliases_mut()
                        .set(name.to_string(), value.to_string());
                    continue;
                }

                let guard = state.read().await;
                if let Some(value) = guard.aliases().get(arg) {
                    stdout.push_str(&format!("alias {arg}='{}'\n", format_alias_value(value)));
                } else {
                    success = false;
                    stderr.push_str(&format!("alias: {arg}: not found\n"));
                }
            }

            Ok(ShellAction::continue_with(CommandOutput {
                exit_code: if success {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                },
                stdout,
                stderr,
            }))
        })
    }
}

fn is_valid_alias_name(name: &str) -> bool {
    !name.is_empty() && !name.chars().any(|ch| ch.is_whitespace() || ch == '=')
}

fn format_alias_value(value: &str) -> String {
    value.replace('\'', r#"'\''"#)
}
