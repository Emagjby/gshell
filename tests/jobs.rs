use gshell::{
    jobs::JobState,
    parser::Parser,
    runtime::{BootstrapExecutor, Executor},
    shell::{ExitCode, ShellAction, ShellState},
};

#[cfg(unix)]
use nix::{
    sys::signal::{Signal, kill},
    unistd::Pid,
};

#[cfg(unix)]
#[tokio::test]
async fn trailing_ampersand_starts_external_command_in_background() {
    use std::time::Duration;

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser.parse("sleep 1 &").expect("parse should succeed");
    let result = executor
        .execute(state.clone(), &parsed)
        .await
        .expect("execution should succeed");

    let job = {
        let guard = state.read().await;
        let jobs = guard.jobs().iter().collect::<Vec<_>>();
        assert_eq!(jobs.len(), 1);
        jobs[0].clone()
    };

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, format!("[{}] {}\n", job.id(), job.summary()));
        }
        ShellAction::Exit(_) => panic!("background command should not exit the shell"),
    }

    assert_eq!(job.state(), JobState::Running);

    kill(Pid::from_raw(job.pgid() as i32), Signal::SIGKILL).expect("SIGKILL should be delivered");
    tokio::time::sleep(Duration::from_millis(100)).await;
    gshell::runtime::refresh_job_statuses(state.clone())
        .await
        .expect("job refresh should succeed");

    let guard = state.read().await;
    assert_eq!(
        guard
            .jobs()
            .get(job.id())
            .expect("job should exist")
            .state(),
        JobState::Completed
    );
}

#[cfg(unix)]
#[tokio::test]
async fn trailing_ampersand_starts_pipeline_in_background() {
    use std::time::Duration;

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("sleep 1 | cat &")
        .expect("parse should succeed");
    let result = executor
        .execute(state.clone(), &parsed)
        .await
        .expect("execution should succeed");

    let (job_id, pgid, summary, process_count, job_state) = {
        let guard = state.read().await;
        let jobs = guard.jobs().iter().collect::<Vec<_>>();
        assert_eq!(jobs.len(), 1);
        let job = jobs[0];
        (
            job.id(),
            job.pgid(),
            job.summary().to_string(),
            job.processes().len(),
            job.state(),
        )
    };

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, format!("[{job_id}] {summary}\n"));
        }
        ShellAction::Exit(_) => panic!("background pipeline should not exit the shell"),
    }

    assert_eq!(process_count, 2);
    assert_eq!(job_state, JobState::Running);

    kill(Pid::from_raw(pgid as i32), Signal::SIGKILL).expect("SIGKILL should be delivered");
    tokio::time::sleep(Duration::from_millis(100)).await;
    gshell::runtime::refresh_job_statuses(state.clone())
        .await
        .expect("job refresh should succeed");

    let guard = state.read().await;
    assert_eq!(
        guard.jobs().get(job_id).expect("job should exist").state(),
        JobState::Completed
    );
}

#[tokio::test]
async fn external_command_creates_completed_foreground_job_record() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser.parse("true").expect("parse should succeed");
    let result = executor
        .execute(state.clone(), &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => assert_eq!(output.exit_code, ExitCode::SUCCESS),
        ShellAction::Exit(_) => panic!("true should not exit the shell"),
    }

    let guard = state.read().await;
    let jobs = guard.jobs().iter().collect::<Vec<_>>();
    assert_eq!(jobs.len(), 1);
    let job = jobs[0];
    assert_eq!(job.state(), JobState::Completed);
    assert_eq!(job.processes().len(), 1);
    assert_eq!(job.pgid(), job.processes()[0].pid());
    assert_eq!(guard.jobs().foreground_job(), None);
}

#[tokio::test]
async fn pipeline_records_one_job_with_multiple_processes() {
    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");

    let parsed = parser
        .parse("printf hi | cat")
        .expect("parse should succeed");
    let result = executor
        .execute(state.clone(), &parsed)
        .await
        .expect("execution should succeed");

    match result {
        ShellAction::Continue(output) => {
            assert_eq!(output.exit_code, ExitCode::SUCCESS);
            assert_eq!(output.stdout, "hi");
        }
        ShellAction::Exit(_) => panic!("pipeline should not exit the shell"),
    }

    let guard = state.read().await;
    let jobs = guard.jobs().iter().collect::<Vec<_>>();
    assert_eq!(jobs.len(), 1);
    let job = jobs[0];
    assert_eq!(job.state(), JobState::Completed);
    assert_eq!(job.processes().len(), 2);
    assert_eq!(job.summary(), "printf hi | cat");
}

#[cfg(unix)]
#[tokio::test]
async fn foreground_stop_marks_job_stopped() {
    use std::time::Duration;

    let parser = Parser::default();
    let executor = BootstrapExecutor;
    let state = ShellState::shared().await.expect("state should initialize");
    let parsed = parser.parse("sleep 10").expect("parse should succeed");

    let state_for_task = state.clone();
    let task = tokio::spawn(async move {
        executor
            .execute(state_for_task, &parsed)
            .await
            .expect("execution should succeed")
    });

    let pid = loop {
        if let Some(pid) = {
            let guard = state.read().await;
            guard.jobs().foreground_job().and_then(|job_id| {
                guard
                    .jobs()
                    .get(job_id)
                    .and_then(|job| job.processes().first().map(|process| process.pid()))
            })
        } {
            break pid;
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

    let guard = state.read().await;
    let jobs = guard.jobs().iter().collect::<Vec<_>>();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].state(), JobState::Stopped);
    assert_eq!(guard.jobs().foreground_job(), None);
}
