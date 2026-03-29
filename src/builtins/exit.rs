use crate::{
    builtins::{Builtin, BuiltinFuture},
    shell::{ExitCode, SharedShellState, ShellAction},
};

pub struct ExitBuiltin;

impl Builtin for ExitBuiltin {
    fn name(&self) -> &'static str {
        "exit"
    }

    fn execute<'a>(&'a self, _state: SharedShellState, args: &'a [String]) -> BuiltinFuture<'a> {
        Box::pin(async move {
            let code = match args {
                [] => ExitCode::SUCCESS,
                [value] => match value.parse::<u8>() {
                    Ok(code) => ExitCode::new(code),
                    Err(_) => {
                        return Ok(ShellAction::continue_with(crate::shell::CommandOutput {
                            exit_code: ExitCode::FAILURE,
                            stdout: String::new(),
                            stderr: format!("exit: numeric argument required: {value}\n"),
                        }));
                    }
                },
                _ => {
                    return Ok(ShellAction::continue_with(crate::shell::CommandOutput {
                        exit_code: ExitCode::FAILURE,
                        stdout: String::new(),
                        stderr: "exit: too many arguments\n".to_string(),
                    }));
                }
            };

            Ok(ShellAction::exit(code))
        })
    }
}
