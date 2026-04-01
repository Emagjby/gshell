use crate::{
    builtins::{Builtin, BuiltinFuture},
    shell::{CommandOutput, ExitCode, SharedShellState, ShellAction},
};

pub struct ExportBuiltin;

impl Builtin for ExportBuiltin {
    fn name(&self) -> &'static str {
        "export"
    }

    fn execute<'a>(&'a self, state: SharedShellState, args: &'a [String]) -> BuiltinFuture<'a> {
        Box::pin(async move {
            if args.is_empty() {
                let guard = state.read().await;
                let mut entries = guard.env().iter().collect::<Vec<_>>();
                entries.sort_by(|(left, _), (right, _)| left.cmp(right));

                let stdout = entries
                    .into_iter()
                    .map(|(name, value)| {
                        format!("export {name}=\"{}\"\n", escape_export_value(value))
                    })
                    .collect();

                return Ok(ShellAction::continue_with(CommandOutput {
                    exit_code: ExitCode::SUCCESS,
                    stdout,
                    stderr: String::new(),
                }));
            }

            let mut stderr = String::new();
            let mut success = true;

            for arg in args {
                if let Some((name, value)) = arg.split_once('=') {
                    if !is_valid_env_name(name) {
                        success = false;
                        stderr.push_str(&format!("export: invalid variable name: {name}\n"));
                        continue;
                    }

                    state.write().await.set_env_var(name, value);
                    continue;
                }

                if !is_valid_env_name(arg) {
                    success = false;
                    stderr.push_str(&format!("export: invalid variable name: {arg}\n"));
                    continue;
                }

                let mut guard = state.write().await;
                if guard.env_var(arg).is_none() {
                    guard.set_var(arg.clone(), String::new());
                }

                if !guard.export_var(arg) {
                    success = false;
                    stderr.push_str(&format!("export: variable not found: {arg}\n"));
                }
            }

            Ok(ShellAction::continue_with(CommandOutput {
                exit_code: if success {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                },
                stdout: String::new(),
                stderr,
            }))
        })
    }
}

fn is_valid_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }

    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn escape_export_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
