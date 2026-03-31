use gshell::{
    builtins::{
        AliasBuiltin, BgBuiltin, Builtin, BuiltinRegistry, CdBuiltin, ClearBuiltin, EchoBuiltin,
        ExitBuiltin, FgBuiltin, HistoryBuiltin, JobsBuiltin, KillBuiltin, PwdBuiltin, TypeBuiltin,
        UnaliasBuiltin,
    },
    jobs::{JobDisposition, JobState, ProcessState},
    parser::Parser,
    runtime::{BootstrapExecutor, Executor, refresh_job_statuses},
    shell::{ExitCode, ShellAction, ShellState},
};

#[cfg(unix)]
use tokio::process::Command;

#[cfg(unix)]
use nix::{
    sys::{
        signal::{Signal, kill},
        wait::waitpid,
    },
    unistd::Pid,
};

#[test]
fn builtin_registry_lookup_works() {
    let registry = BuiltinRegistry::with_defaults();

    assert!(registry.contains("cd"));
    assert!(registry.contains("exit"));
    assert!(registry.contains("clear"));
    assert!(registry.contains("alias"));
    assert!(registry.contains("type"));
    assert!(registry.contains("unalias"));
    assert!(registry.contains("echo"));
    assert!(registry.contains("pwd"));
    assert!(registry.contains("history"));
    assert!(registry.contains("jobs"));
    assert!(registry.contains("fg"));
    assert!(registry.contains("bg"));
    assert!(registry.contains("kill"));
    assert!(registry.get("missing").is_none());
}

#[tokio::test]
async fn jobs_builtin_lists_jobs() {
    let state = ShellState::shared().await.expect("state should initialize");
    {
        let mut guard = state.write().await;
        let first = guard.jobs_mut().insert(
            4100,
            "sleep 10",
            JobDisposition::Background,
            vec![gshell::jobs::ProcessRecord::new(4100, "sleep 10")],
        );
        let second = guard.jobs_mut().insert(
            4200,
            "printf hi | cat",
            JobDisposition::Background,
            vec![
                gshell::jobs::ProcessRecord::new(4201, "printf hi"),
                gshell::jobs::ProcessRecord::new(4202, "cat"),
            ],
        );
        let _ = guard
            .jobs_mut()
            .update_process_state(second, 4201, ProcessState::Stopped);
        let _ = guard
            .jobs_mut()
            .update_process_state(first, 4100, ProcessState::Completed(0));
        let _third = guard.jobs_mut().insert(
            4300,
            "sleep 30",
            JobDisposition::Foreground,
            vec![gshell::jobs::ProcessRecord::new(4300, "sleep 30")],
        );
    }
    let builtin = JobsBuiltin;

    let result = builtin
        .execute(state, &[])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "[2] Stopped printf hi | cat\n");
        }
        ShellAction::Exit(_) => panic!("jobs should not exit"),
    }
}

#[tokio::test]
async fn fg_builtin_rejects_invalid_job_ids() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = FgBuiltin;

    let result = builtin
        .execute(state.clone(), &["%abc".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::FAILURE);
            assert_eq!(output.stderr, "fg: invalid job id: %abc\n");
        }
        ShellAction::Exit(_) => panic!("fg should not exit"),
    }

    let result = builtin
        .execute(state, &["%1".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::FAILURE);
            assert_eq!(output.stderr, "fg: no such job: %1\n");
        }
        ShellAction::Exit(_) => panic!("fg should not exit"),
    }
}

#[tokio::test]
async fn bg_builtin_rejects_missing_current_job() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = BgBuiltin;

    let result = builtin
        .execute(state, &[])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::FAILURE);
            assert_eq!(output.stderr, "bg: no current job\n");
        }
        ShellAction::Exit(_) => panic!("bg should not exit"),
    }
}

#[tokio::test]
async fn kill_builtin_requires_at_least_one_target() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = KillBuiltin;

    let result = builtin
        .execute(state, &[])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::FAILURE);
            assert_eq!(output.stderr, "kill: usage: kill [-SIGNAL] <pid|%job>...\n");
        }
        ShellAction::Exit(_) => panic!("kill should not exit"),
    }
}

#[tokio::test]
async fn kill_builtin_rejects_invalid_job_ids() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = KillBuiltin;

    let result = builtin
        .execute(state.clone(), &["%abc".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::FAILURE);
            assert_eq!(output.stderr, "kill: invalid job id: %abc\n");
        }
        ShellAction::Exit(_) => panic!("kill should not exit"),
    }

    let result = builtin
        .execute(state, &["%1".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::FAILURE);
            assert_eq!(output.stderr, "kill: no such job: %1\n");
        }
        ShellAction::Exit(_) => panic!("kill should not exit"),
    }
}

#[cfg(unix)]
#[tokio::test]
async fn bg_builtin_resumes_stopped_job_in_background() {
    let (state, pid, job_id) = spawn_stopped_sleep_job().await;
    let builtin = BgBuiltin;

    let result = builtin
        .execute(state.clone(), &[])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, format!("[{job_id}] sleep 1\n"));
        }
        ShellAction::Exit(_) => panic!("bg should not exit"),
    }

    let guard = state.read().await;
    let job = guard.jobs().get(job_id).expect("job should exist");
    assert_eq!(job.state(), JobState::Running);
    assert_eq!(job.disposition(), JobDisposition::Background);
    drop(guard);

    kill(Pid::from_raw(pid as i32), Signal::SIGTERM).expect("SIGTERM should be delivered");
    let _ = waitpid(Pid::from_raw(pid as i32), None).expect("waitpid should succeed");
}

#[cfg(unix)]
#[tokio::test]
async fn fg_builtin_resumes_stopped_job_in_foreground() {
    let (state, _pid, job_id) = spawn_stopped_sleep_job().await;
    let builtin = FgBuiltin;

    let result = builtin
        .execute(state.clone(), &[])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "sleep 1\n");
        }
        ShellAction::Exit(_) => panic!("fg should not exit"),
    }

    let guard = state.read().await;
    let job = guard.jobs().get(job_id).expect("job should exist");
    assert_eq!(job.state(), JobState::Completed);
    assert_eq!(guard.jobs().foreground_job(), None);
}

#[cfg(unix)]
#[tokio::test]
async fn kill_builtin_terminates_job_by_job_id() {
    let (state, _pid, job_id) = spawn_stopped_sleep_job().await;
    let builtin = KillBuiltin;

    let result = builtin
        .execute(
            state.clone(),
            &[String::from("-KILL"), format!("%{job_id}")],
        )
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert!(output.stdout.is_empty());
            assert!(output.stderr.is_empty());
        }
        ShellAction::Exit(_) => panic!("kill should not exit"),
    }

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    refresh_job_statuses(state.clone())
        .await
        .expect("job refresh should succeed");

    let guard = state.read().await;
    let job = guard.jobs().get(job_id).expect("job should exist");
    assert_eq!(job.state(), JobState::Completed);
}

#[cfg(unix)]
#[tokio::test]
async fn jobs_builtin_hides_completed_background_jobs_after_refresh() {
    let state = ShellState::shared().await.expect("state should initialize");
    let child = Command::new("sleep")
        .arg("0.1")
        .spawn()
        .expect("sleep should spawn");
    let pid = child.id().expect("child should have a pid");

    {
        let mut guard = state.write().await;
        guard.jobs_mut().insert(
            pid,
            "sleep 0.1",
            JobDisposition::Background,
            vec![gshell::jobs::ProcessRecord::new(pid, "sleep 0.1")],
        );
    }

    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            refresh_job_statuses(state.clone())
                .await
                .expect("job refresh should succeed");

            let completed = {
                let guard = state.read().await;
                guard
                    .jobs()
                    .iter()
                    .any(|job| job.processes().iter().any(|process| process.pid() == pid))
                    && guard
                        .jobs()
                        .iter()
                        .find(|job| job.processes().iter().any(|process| process.pid() == pid))
                        .is_some_and(|job| job.state() == JobState::Completed)
            };

            if completed {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("job should complete within timeout");

    let builtin = JobsBuiltin;
    let result = builtin
        .execute(state.clone(), &[])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert!(output.stdout.is_empty());
        }
        ShellAction::Exit(_) => panic!("jobs should not exit"),
    }

    drop(child);
}

#[cfg(unix)]
#[tokio::test]
async fn bg_job_disappears_from_jobs_after_it_finishes() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");
    let parsed = parser.parse("sleep 1").expect("parse should succeed");

    let state_for_task = state.clone();
    let task = tokio::spawn(async move {
        executor
            .execute(state_for_task, &parsed)
            .await
            .expect("execution should succeed")
    });

    let (pid, job_id) = loop {
        if let Some((pid, job_id)) = {
            let guard = state.read().await;
            guard.jobs().foreground_job().and_then(|job_id| {
                guard.jobs().get(job_id).and_then(|job| {
                    job.processes()
                        .first()
                        .map(|process| (process.pid(), job_id))
                })
            })
        } {
            break (pid, job_id);
        }

        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    };

    kill(Pid::from_raw(pid as i32), Signal::SIGSTOP).expect("SIGSTOP should be delivered");

    let result = tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .expect("execution should return after stop")
        .expect("task should join successfully");

    match result {
        ShellAction::Continue(output) => assert!(output.exit_code.is_failure()),
        ShellAction::Exit(_) => panic!("sleep should not exit the shell"),
    }

    let bg = BgBuiltin;
    let result = bg
        .execute(state.clone(), &[])
        .await
        .expect("bg should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, format!("[{job_id}] sleep 1\n"));
        }
        ShellAction::Exit(_) => panic!("bg should not exit"),
    }

    tokio::time::sleep(std::time::Duration::from_millis(1300)).await;

    let jobs = JobsBuiltin;
    let result = jobs
        .execute(state.clone(), &[])
        .await
        .expect("jobs should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert!(
                output.stdout.is_empty(),
                "unexpected jobs output: {}",
                output.stdout
            );
        }
        ShellAction::Exit(_) => panic!("jobs should not exit"),
    }
}

#[cfg(unix)]
async fn spawn_stopped_sleep_job() -> (gshell::shell::SharedShellState, u32, u32) {
    use std::time::Duration;

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");
    let parsed = parser.parse("sleep 1").expect("parse should succeed");

    let state_for_task = state.clone();
    let task = tokio::spawn(async move {
        executor
            .execute(state_for_task, &parsed)
            .await
            .expect("execution should succeed")
    });

    let (pid, job_id) = loop {
        if let Some((pid, job_id)) = {
            let guard = state.read().await;
            guard.jobs().foreground_job().and_then(|job_id| {
                guard.jobs().get(job_id).and_then(|job| {
                    job.processes()
                        .first()
                        .map(|process| (process.pid(), job_id))
                })
            })
        } {
            break (pid, job_id);
        }

        tokio::time::sleep(Duration::from_millis(25)).await;
    };

    kill(Pid::from_raw(pid as i32), Signal::SIGSTOP).expect("SIGSTOP should be delivered");

    let result = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("execution should return after stop")
        .expect("task should join successfully");

    match result {
        ShellAction::Continue(output) => assert!(output.exit_code.is_failure()),
        ShellAction::Exit(_) => panic!("sleep should not exit the shell"),
    }

    (state, pid, job_id)
}

#[tokio::test]
async fn alias_builtin_sets_and_lists_aliases() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = AliasBuiltin;

    let result = builtin
        .execute(state.clone(), &["ll=echo hello".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert!(output.stdout.is_empty());
            assert_eq!(state.read().await.aliases().get("ll"), Some("echo hello"));
        }
        ShellAction::Exit(_) => panic!("alias should not exit"),
    }

    let result = builtin
        .execute(state, &["ll".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "alias ll='echo hello'\n");
        }
        ShellAction::Exit(_) => panic!("alias should not exit"),
    }
}

#[tokio::test]
async fn unalias_builtin_removes_existing_alias() {
    let state = ShellState::shared().await.expect("state should initialize");
    state.write().await.aliases_mut().set("ll", "echo hello");
    let builtin = UnaliasBuiltin;

    let result = builtin
        .execute(state.clone(), &["ll".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert!(state.read().await.aliases().get("ll").is_none());
        }
        ShellAction::Exit(_) => panic!("unalias should not exit"),
    }
}

#[tokio::test]
async fn echo_builtin_outputs_joined_args() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = EchoBuiltin;

    let result = builtin
        .execute(state, &["hello".into(), "world".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "hello world\n");
        }
        ShellAction::Exit(_) => panic!("echo should not exit"),
    }
}

#[tokio::test]
async fn pwd_builtin_outputs_current_directory() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = PwdBuiltin;

    let result = builtin
        .execute(state.clone(), &[])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            let cwd = state.read().await.cwd().display().to_string();
            assert_eq!(output.stdout, format!("{cwd}\n"));
        }
        ShellAction::Exit(_) => panic!("pwd should not exit"),
    }
}

#[tokio::test]
async fn clear_builtin_emits_escape_sequence() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = ClearBuiltin;

    let result = builtin
        .execute(state, &[])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "\x1B[2J\x1B[H");
        }
        ShellAction::Exit(_) => panic!("clear should not exit"),
    }
}

#[tokio::test]
async fn cd_builtin_changes_shell_state_directory() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = CdBuiltin;

    let tmp = tempfile::tempdir().expect("temp dir should be created");
    let path = tmp.path().display().to_string();

    let result = builtin
        .execute(state.clone(), &[path])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            let cwd = state.read().await.cwd().to_path_buf();
            let expected = std::fs::canonicalize(tmp.path())
                .expect("temp dir path should canonicalize successfully");
            assert_eq!(cwd, expected);
        }
        ShellAction::Exit(_) => panic!("cd should not exit"),
    }
}

#[tokio::test]
async fn cd_builtin_rejects_too_many_arguments() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = CdBuiltin;

    let result = builtin
        .execute(state, &["a".into(), "b".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::FAILURE);
            assert!(output.stderr.contains("too many arguments"));
        }
        ShellAction::Exit(_) => panic!("cd should not exit"),
    }
}

#[tokio::test]
async fn type_builtin_reports_builtin() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = TypeBuiltin;

    let result = builtin
        .execute(state, &["echo".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert!(output.stdout.contains("echo is a shell builtin"));
        }
        ShellAction::Exit(_) => panic!("type should not exit"),
    }
}

#[tokio::test]
async fn type_builtin_reports_alias() {
    let state = ShellState::shared().await.expect("state should initialize");
    state.write().await.aliases_mut().set("ll", "echo hello");
    let builtin = TypeBuiltin;

    let result = builtin
        .execute(state, &["ll".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert!(output.stdout.contains("ll is aliased to `echo hello`"));
        }
        ShellAction::Exit(_) => panic!("type should not exit"),
    }
}

#[tokio::test]
async fn type_builtin_reports_function() {
    let state = ShellState::shared().await.expect("state should initialize");
    let parser = Parser::default();
    let defined = parser
        .parse("greet() { echo hi; }")
        .expect("parse should succeed");
    let executor = BootstrapExecutor;
    let _ = executor
        .execute(state.clone(), &defined)
        .await
        .expect("function definition should succeed");
    let builtin = TypeBuiltin;

    let result = builtin
        .execute(state, &["greet".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert!(output.stdout.contains("greet is a shell function"));
        }
        ShellAction::Exit(_) => panic!("type should not exit"),
    }
}

#[tokio::test]
async fn type_builtin_fails_for_unknown_command() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = TypeBuiltin;

    let result = builtin
        .execute(state, &["definitely-not-real".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::FAILURE);
            assert!(output.stderr.contains("not found"));
        }
        ShellAction::Exit(_) => panic!("type should not exit"),
    }
}

#[tokio::test]
async fn exit_builtin_requests_shell_exit() {
    let state = ShellState::shared().await.expect("state should initialize");
    let builtin = ExitBuiltin;

    let result = builtin
        .execute(state, &["42".into()])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Exit(code) => assert_eq!(code, ExitCode::new(42)),
        ShellAction::Continue(_) => panic!("exit should terminate the shell"),
    }
}

#[tokio::test]
async fn history_builtin_outputs_entries() {
    let state = ShellState::shared().await.expect("state should initialize");
    {
        let mut guard = state.write().await;
        guard.history_mut().push("echo hello");
        guard.history_mut().push("history");
    }

    let builtin = HistoryBuiltin;
    let result = builtin
        .execute(state, &[])
        .await
        .expect("builtin execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert!(output.stdout.contains("echo hello"));
            assert!(output.stdout.contains("history"));
        }
        ShellAction::Exit(_) => panic!("history should not exit"),
    }
}

#[tokio::test]
async fn parsed_argv_reaches_echo_builtin_with_single_quotes() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("echo 'hello world'")
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "hello world\n");
        }
        ShellAction::Exit(_) => panic!("echo should not exit"),
    }
}

#[tokio::test]
async fn parsed_argv_reaches_echo_builtin_with_double_quotes() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("echo \"hello world\"")
        .expect("parse should succeed");

    let result = executor
        .execute(state, &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "hello world\n");
        }
        ShellAction::Exit(_) => panic!("echo should not exit"),
    }
}

#[tokio::test]
async fn assignment_only_command_updates_shell_environment() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser.parse("NAME=gencho").expect("parse should succeed");

    let result = executor
        .execute(state.clone(), &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(state.read().await.env_var("NAME"), Some("gencho"));
        }
        ShellAction::Exit(_) => panic!("assignment should not exit"),
    }
}

#[tokio::test]
async fn assignment_prefix_is_visible_to_builtin_arguments() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("NAME=gencho echo $NAME")
        .expect("parse should succeed");

    let result = executor
        .execute(state.clone(), &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "gencho\n");
        }
        ShellAction::Exit(_) => panic!("echo should not exit"),
    }
}

#[tokio::test]
async fn assignment_prefix_does_not_persist_after_builtin_runs() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("NAME=gencho echo $NAME; echo $NAME")
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
        ShellAction::Exit(_) => panic!("echo should not exit"),
    }
}
