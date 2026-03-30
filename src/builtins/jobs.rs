use crate::{
    builtins::{Builtin, BuiltinFuture},
    jobs::{JobDisposition, JobId, JobState},
    runtime::{continue_job_in_background, continue_job_in_foreground, refresh_job_statuses},
    shell::{CommandOutput, ExitCode, SharedShellState, ShellAction},
};

pub struct JobsBuiltin;

impl Builtin for JobsBuiltin {
    fn name(&self) -> &'static str {
        "jobs"
    }

    fn execute<'a>(&'a self, state: SharedShellState, args: &'a [String]) -> BuiltinFuture<'a> {
        Box::pin(async move {
            if !args.is_empty() {
                return Ok(ShellAction::continue_with(CommandOutput::failure(
                    ExitCode::FAILURE,
                    "jobs: expected no arguments\n",
                )));
            }

            refresh_job_statuses(state.clone()).await?;

            let guard = state.read().await;
            let mut stdout = String::new();

            for job in guard
                .jobs()
                .iter()
                .filter(|job| should_display_job(job.state(), job.disposition()))
            {
                stdout.push_str(&format_job(job.id(), job.state(), job.summary()));
            }

            Ok(ShellAction::continue_with(CommandOutput {
                exit_code: ExitCode::SUCCESS,
                stdout,
                stderr: String::new(),
            }))
        })
    }
}

pub struct FgBuiltin;

impl Builtin for FgBuiltin {
    fn name(&self) -> &'static str {
        "fg"
    }

    fn execute<'a>(&'a self, state: SharedShellState, args: &'a [String]) -> BuiltinFuture<'a> {
        Box::pin(async move {
            refresh_job_statuses(state.clone()).await?;

            let job_id = match resolve_job_id(state.clone(), args, "fg").await {
                Ok(job_id) => job_id,
                Err(output) => return Ok(ShellAction::continue_with(output)),
            };

            match continue_job_in_foreground(state, job_id).await? {
                Some(output) => Ok(ShellAction::continue_with(output)),
                None => Ok(ShellAction::continue_with(CommandOutput::failure(
                    ExitCode::FAILURE,
                    format!("fg: no such job: %{job_id}\n"),
                ))),
            }
        })
    }
}

pub struct BgBuiltin;

impl Builtin for BgBuiltin {
    fn name(&self) -> &'static str {
        "bg"
    }

    fn execute<'a>(&'a self, state: SharedShellState, args: &'a [String]) -> BuiltinFuture<'a> {
        Box::pin(async move {
            refresh_job_statuses(state.clone()).await?;

            let job_id = match resolve_job_id(state.clone(), args, "bg").await {
                Ok(job_id) => job_id,
                Err(output) => return Ok(ShellAction::continue_with(output)),
            };

            let summary = {
                let guard = state.read().await;
                guard
                    .jobs()
                    .get(job_id)
                    .map(|job| job.summary().to_string())
                    .unwrap_or_default()
            };

            match continue_job_in_background(state, job_id).await? {
                Some(output) if output.exit_code.is_failure() => {
                    Ok(ShellAction::continue_with(output))
                }
                Some(_) => Ok(ShellAction::continue_with(CommandOutput {
                    exit_code: ExitCode::SUCCESS,
                    stdout: format!("[{job_id}] {summary}\n"),
                    stderr: String::new(),
                })),
                None => Ok(ShellAction::continue_with(CommandOutput::failure(
                    ExitCode::FAILURE,
                    format!("bg: no such job: %{job_id}\n"),
                ))),
            }
        })
    }
}

fn format_job(job_id: JobId, state: JobState, summary: &str) -> String {
    format!("[{job_id}] {} {summary}\n", format_job_state(state))
}

fn should_display_job(state: JobState, disposition: JobDisposition) -> bool {
    match state {
        JobState::Stopped => true,
        JobState::Running => matches!(disposition, JobDisposition::Background),
        JobState::Completed => false,
    }
}

fn format_job_state(state: JobState) -> &'static str {
    match state {
        JobState::Running => "Running",
        JobState::Stopped => "Stopped",
        JobState::Completed => "Done",
    }
}

async fn resolve_job_id(
    state: SharedShellState,
    args: &[String],
    command: &str,
) -> Result<JobId, CommandOutput> {
    match args {
        [] => state.read().await.jobs().current_job().ok_or_else(|| {
            CommandOutput::failure(ExitCode::FAILURE, format!("{command}: no current job\n"))
        }),
        [job] => parse_job_id(job).map_err(|message| {
            CommandOutput::failure(ExitCode::FAILURE, format!("{command}: {message}\n"))
        }),
        _ => Err(CommandOutput::failure(
            ExitCode::FAILURE,
            format!("{command}: expected at most one job id\n"),
        )),
    }
}

fn parse_job_id(input: &str) -> Result<JobId, String> {
    let trimmed = input.strip_prefix('%').unwrap_or(input);
    if trimmed.is_empty() {
        return Err(format!("invalid job id: {input}"));
    }

    trimmed
        .parse::<JobId>()
        .map_err(|_| format!("invalid job id: {input}"))
}

#[cfg(test)]
mod tests {
    use super::{parse_job_id, should_display_job};
    use crate::jobs::{JobDisposition, JobState};

    #[test]
    fn displays_only_stopped_and_background_running_jobs() {
        assert!(should_display_job(
            JobState::Stopped,
            JobDisposition::Foreground
        ));
        assert!(should_display_job(
            JobState::Stopped,
            JobDisposition::Background
        ));
        assert!(should_display_job(
            JobState::Running,
            JobDisposition::Background
        ));
        assert!(!should_display_job(
            JobState::Running,
            JobDisposition::Foreground
        ));
        assert!(!should_display_job(
            JobState::Completed,
            JobDisposition::Background
        ));
    }

    #[test]
    fn parses_small_v1_job_id_forms() {
        assert_eq!(parse_job_id("1"), Ok(1));
        assert_eq!(parse_job_id("%42"), Ok(42));
    }

    #[test]
    fn rejects_invalid_job_id_forms() {
        assert!(parse_job_id("%").is_err());
        assert!(parse_job_id("%+").is_err());
        assert!(parse_job_id("abc").is_err());
    }
}
