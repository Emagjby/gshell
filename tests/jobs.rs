use gshell::{
    jobs::JobState,
    parser::Parser,
    runtime::{BootstrapExecutor, Executor},
    shell::{ExitCode, ShellAction, ShellState},
};

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
