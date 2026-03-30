use std::{
    collections::HashMap,
    env,
    ffi::OsString,
    fs::{File, OpenOptions},
    future::Future,
    io,
    path::{Path, PathBuf},
    pin::Pin,
    process::Stdio,
    sync::Arc,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
    sync::RwLock,
    task::JoinHandle,
};

mod unix;

use crate::{
    ast::{BoolOp, CommandNode, RedirectionKind, ShellExpr, SimpleCommand},
    builtins::BuiltinRegistry,
    expand::{
        CommandSubstitutionExecutor, Word, expand_words_pathnames_with_state,
        expand_words_with_state,
    },
    jobs::{JobDisposition, JobId, JobState, ProcessRecord, ProcessState},
    parser::{ParsedCommand, Parser},
    shell::{CommandOutput, ExitCode, SharedShellState, ShellAction, ShellError, ShellResult},
};

#[derive(Debug)]
struct RedirectionPlan {
    stdin: Option<InputRedirection>,
    stdout: Option<OutputRedirection>,
    stderr: Option<OutputRedirection>,
}

#[derive(Debug, Clone)]
enum InputRedirection {
    File(PathBuf),
    Inline(Vec<u8>),
}

#[derive(Debug, Clone)]
struct OutputRedirection {
    path: PathBuf,
    append: bool,
}

impl RedirectionPlan {
    fn empty() -> Self {
        Self {
            stdin: None,
            stdout: None,
            stderr: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecutionMode {
    Normal,
    Capture,
    Pipeline,
}

#[derive(Debug, Clone)]
struct PipelineOutput {
    exit_code: ExitCode,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

#[derive(Debug, Clone)]
struct ExpandedRedirection {
    fd: Option<u8>,
    kind: RedirectionKind,
    target: Option<String>,
}

#[derive(Debug, Clone)]
struct PipelineJobContext {
    job_id: Option<JobId>,
    pgid: Option<u32>,
    summary: String,
    foreground_claimed: bool,
}

impl ExpandedRedirection {
    fn effective_fd(&self) -> u8 {
        match (&self.fd, &self.kind) {
            (Some(fd), _) => *fd,
            (None, RedirectionKind::Input | RedirectionKind::HereDoc { .. }) => 0,
            (None, RedirectionKind::OutputTruncate | RedirectionKind::OutputAppend) => 1,
        }
    }
}

pub type ExecutorFuture<'a> = Pin<Box<dyn Future<Output = ShellResult<ShellAction>> + Send + 'a>>;

pub trait Executor<C>: Send + Sync {
    fn execute<'a>(&'a self, state: SharedShellState, command: &'a C) -> ExecutorFuture<'a>;
}

pub async fn initialize_interactive_shell() -> ShellResult<()> {
    unix::initialize_interactive_shell().await
}

pub async fn refresh_job_statuses(state: SharedShellState) -> ShellResult<()> {
    let child_handles = { state.read().await.child_handles().clone() };
    let child_pids = child_handles.pids().await;

    for pid in child_pids {
        let child_handle = child_handles.get(pid).await;

        let Some(child_handle) = child_handle else {
            continue;
        };

        let mut child = child_handle.lock().await;
        let Some(exit_status) = child.try_wait()? else {
            continue;
        };
        drop(child);
        let _ = child_handles.remove(pid).await;

        let process_state = process_state_from_exit_status(exit_status);
        let mut guard = state.write().await;
        let jobs = guard.jobs_mut();
        if let Some(job_id) = jobs.job_id_for_pid(pid) {
            let _ = jobs.update_process_state(job_id, pid, process_state);
        }
    }

    let statuses = unix::poll_child_statuses().await?;

    if statuses.is_empty() {
        return Ok(());
    }

    let mut guard = state.write().await;
    let jobs = guard.jobs_mut();

    for status in statuses {
        match status {
            unix::PolledWaitStatus::Exited { pid, code } => {
                if let Some(job_id) = jobs.job_id_for_pid(pid) {
                    let _ = jobs.update_process_state(job_id, pid, ProcessState::Completed(code));
                }
            }
            unix::PolledWaitStatus::Signaled { pid, signal } => {
                if let Some(job_id) = jobs.job_id_for_pid(pid) {
                    let code = signal_exit_code(signal);
                    let _ = jobs.update_process_state(
                        job_id,
                        pid,
                        ProcessState::Completed(i32::from(code.as_u8())),
                    );
                }
            }
            unix::PolledWaitStatus::Stopped { pid, .. } => {
                if let Some(job_id) = jobs.job_id_for_pid(pid) {
                    let _ = jobs.update_process_state(job_id, pid, ProcessState::Stopped);
                }
            }
            unix::PolledWaitStatus::Continued { pid } => {
                if let Some(job_id) = jobs.job_id_for_pid(pid) {
                    let _ = jobs.update_process_state(job_id, pid, ProcessState::Running);
                }
            }
        }
    }

    Ok(())
}

pub async fn continue_job_in_background(
    state: SharedShellState,
    job_id: JobId,
) -> ShellResult<Option<CommandOutput>> {
    let pgid = {
        let guard = state.read().await;
        let Some(job) = guard.jobs().get(job_id) else {
            return Ok(None);
        };

        if matches!(job.state(), JobState::Completed) {
            return Ok(Some(CommandOutput::failure(
                ExitCode::FAILURE,
                format!("job has already completed: %{job_id}\n"),
            )));
        }

        job.pgid()
    };

    unix::continue_process_group(pgid)?;

    let mut guard = state.write().await;
    let jobs = guard.jobs_mut();
    let _ = jobs.set_disposition(job_id, JobDisposition::Background);
    let _ = jobs.set_all_processes_running(job_id);

    Ok(Some(CommandOutput::success()))
}

pub async fn continue_job_in_foreground(
    state: SharedShellState,
    job_id: JobId,
) -> ShellResult<Option<CommandOutput>> {
    let (pgid, pids, summary) = {
        let guard = state.read().await;
        let Some(job) = guard.jobs().get(job_id) else {
            return Ok(None);
        };

        if matches!(job.state(), JobState::Completed) {
            return Ok(Some(CommandOutput::failure(
                ExitCode::FAILURE,
                format!("job has already completed: %{job_id}\n"),
            )));
        }

        (
            job.pgid(),
            job.processes()
                .iter()
                .filter(|process| !matches!(process.state(), ProcessState::Completed(_)))
                .map(ProcessRecord::pid)
                .collect::<Vec<_>>(),
            job.summary().to_string(),
        )
    };

    {
        let mut guard = state.write().await;
        let jobs = guard.jobs_mut();
        let _ = jobs.set_disposition(job_id, JobDisposition::Foreground);
        let _ = jobs.set_all_processes_running(job_id);
    }

    unix::continue_process_group(pgid)?;
    let foreground_claimed = unix::hand_terminal_to_foreground_job(pgid)?;
    let mut last_exit_code = ExitCode::SUCCESS;

    for pid in pids {
        let wait_status = unix::wait_for_foreground_process(pid).await?;
        let (process_state, exit_code) = process_state_from_wait_status(wait_status);
        last_exit_code = exit_code;

        let child_handles = { state.read().await.child_handles().clone() };
        let mut guard = state.write().await;
        let _ = guard
            .jobs_mut()
            .update_process_state(job_id, pid, process_state);

        if matches!(wait_status, unix::ForegroundWaitStatus::Stopped(_)) {
            let _ = guard.jobs_mut().set_all_processes_stopped(job_id);
            break;
        }

        drop(guard);

        if matches!(process_state, ProcessState::Completed(_)) {
            let _ = child_handles.remove(pid).await;
        }
    }

    if foreground_claimed {
        let _ = unix::reclaim_terminal_for_shell();
    }

    Ok(Some(CommandOutput {
        exit_code: last_exit_code,
        stdout: format!("{summary}\n"),
        stderr: String::new(),
    }))
}

#[derive(Debug, Default)]
pub struct BootstrapExecutor;

impl Executor<ParsedCommand> for BootstrapExecutor {
    fn execute<'a>(
        &'a self,
        state: SharedShellState,
        command: &'a ParsedCommand,
    ) -> ExecutorFuture<'a> {
        Box::pin(async move {
            match command {
                ParsedCommand::Empty => Ok(ShellAction::continue_with(CommandOutput::success())),
                ParsedCommand::Expr(expr) => {
                    self.execute_expr(state, expr, ExecutionMode::Normal).await
                }
            }
        })
    }
}

impl BootstrapExecutor {
    fn execute_expr<'a>(
        &'a self,
        state: SharedShellState,
        expr: &'a ShellExpr,
        mode: ExecutionMode,
    ) -> Pin<Box<dyn Future<Output = ShellResult<ShellAction>> + Send + 'a>> {
        Box::pin(async move {
            match expr {
                ShellExpr::Command(node) => self.execute_command_node(state, node, mode).await,
                ShellExpr::Pipeline(commands) => self.execute_pipeline(state, commands).await,
                ShellExpr::BooleanChain { first, rest } => {
                    self.execute_boolean_chain(state, first, rest, mode).await
                }
                ShellExpr::Sequence(exprs) => self.execute_sequence(state, exprs, mode).await,
            }
        })
    }

    async fn execute_command_substitution(
        &self,
        state: SharedShellState,
        expr: ShellExpr,
    ) -> ShellResult<String> {
        let isolated_state = clone_shell_state_for_pipeline(&state).await;

        let action = self
            .execute_expr(isolated_state, &expr, ExecutionMode::Capture)
            .await?;

        match action {
            ShellAction::Continue(output) => Ok(output.stdout),
            ShellAction::Exit(_) => Err(ShellError::message(
                "command substitution cannot terminate the parent shell",
            )),
        }
    }

    async fn execute_command_node(
        &self,
        state: SharedShellState,
        node: &CommandNode,
        mode: ExecutionMode,
    ) -> ShellResult<ShellAction> {
        match node {
            CommandNode::Simple(simple) => self.execute_simple_command(state, simple, mode).await,
            CommandNode::FunctionDef { name, body } => {
                self.execute_function_definition(state, name, body, mode)
                    .await
            }
            CommandNode::Subshell(expr) => {
                self.execute_subshell_placeholder(state, expr, mode).await
            }
        }
    }

    async fn execute_subshell_placeholder(
        &self,
        state: SharedShellState,
        expr: &ShellExpr,
        mode: ExecutionMode,
    ) -> ShellResult<ShellAction> {
        self.execute_expr(state, expr, mode).await
    }

    async fn execute_simple_command(
        &self,
        state: SharedShellState,
        simple: &SimpleCommand,
        mode: ExecutionMode,
    ) -> ShellResult<ShellAction> {
        let (expanded_argv, expanded_redirections) =
            expand_simple_command(state.clone(), simple).await?;

        let Some((name, args)) = expanded_argv.split_first() else {
            return Ok(ShellAction::continue_with(CommandOutput::success()));
        };

        let registry = BuiltinRegistry::defaults();

        let function = {
            let guard = state.read().await;
            guard.functions().get(name).cloned()
        };

        if let Some(function) = function {
            return self
                .execute_shell_function(state, name, &function, &expanded_redirections, mode)
                .await;
        }

        if let Some(builtin) = registry.get(name) {
            return self
                .execute_builtin_simple(state, builtin, args, &expanded_redirections, mode)
                .await;
        }

        match mode {
            ExecutionMode::Normal => {
                execute_external(state, name, args, &expanded_redirections).await
            }
            ExecutionMode::Capture => {
                let mut job_context = PipelineJobContext {
                    job_id: None,
                    pgid: None,
                    summary: summarize_command(name, args),
                    foreground_claimed: false,
                };
                let output = self
                    .execute_external_pipeline_segment(
                        state,
                        name,
                        args,
                        &expanded_redirections,
                        None,
                        &mut job_context,
                    )
                    .await?;

                Ok(ShellAction::continue_with(CommandOutput {
                    exit_code: output.exit_code,
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                }))
            }
            ExecutionMode::Pipeline => {
                let mut job_context = PipelineJobContext {
                    job_id: None,
                    pgid: None,
                    summary: summarize_command(name, args),
                    foreground_claimed: false,
                };
                let output = self
                    .execute_external_pipeline_segment(
                        state,
                        name,
                        args,
                        &expanded_redirections,
                        None,
                        &mut job_context,
                    )
                    .await?;

                Ok(ShellAction::continue_with(CommandOutput {
                    exit_code: output.exit_code,
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                }))
            }
        }
    }

    async fn execute_function_definition(
        &self,
        state: SharedShellState,
        name: &str,
        body: &ShellExpr,
        mode: ExecutionMode,
    ) -> ShellResult<ShellAction> {
        match mode {
            ExecutionMode::Normal | ExecutionMode::Capture => {
                state
                    .write()
                    .await
                    .functions_mut()
                    .set(name.to_string(), body.clone());
            }
            ExecutionMode::Pipeline => {
                let isolated_state = clone_shell_state_for_pipeline(&state).await;
                isolated_state
                    .write()
                    .await
                    .functions_mut()
                    .set(name.to_string(), body.clone());
            }
        }

        Ok(ShellAction::continue_with(CommandOutput::success()))
    }

    async fn execute_shell_function(
        &self,
        state: SharedShellState,
        name: &str,
        body: &ShellExpr,
        redirections: &[ExpandedRedirection],
        mode: ExecutionMode,
    ) -> ShellResult<ShellAction> {
        let execution_state = match mode {
            ExecutionMode::Pipeline => clone_shell_state_for_pipeline(&state).await,
            ExecutionMode::Normal | ExecutionMode::Capture => state,
        };

        if !execution_state.read().await.can_enter_function(name) {
            return Ok(ShellAction::continue_with(CommandOutput {
                exit_code: ExitCode::FAILURE,
                stdout: String::new(),
                stderr: format!("function recursion detected: {name}\n"),
            }));
        }

        execution_state
            .write()
            .await
            .enter_function(name.to_string());
        let result = self.execute_expr(execution_state.clone(), body, mode).await;
        execution_state.write().await.exit_function();

        let action = result?;

        match action {
            ShellAction::Continue(output) => {
                let plan = match build_redirection_plan(redirections) {
                    Ok(plan) => plan,
                    Err(err) => {
                        return Ok(ShellAction::continue_with(CommandOutput {
                            exit_code: ExitCode::FAILURE,
                            stdout: String::new(),
                            stderr: format!("{err}\n"),
                        }));
                    }
                };

                let redirected = match apply_builtin_redirections(output, &plan) {
                    Ok(output) => output,
                    Err(err) => CommandOutput {
                        exit_code: ExitCode::FAILURE,
                        stdout: String::new(),
                        stderr: format!("{err}\n"),
                    },
                };

                Ok(ShellAction::continue_with(redirected))
            }
            ShellAction::Exit(code) => Ok(ShellAction::Exit(code)),
        }
    }

    async fn execute_builtin_simple(
        &self,
        state: SharedShellState,
        builtin: Arc<dyn crate::builtins::Builtin>,
        args: &[String],
        redirections: &[ExpandedRedirection],
        mode: ExecutionMode,
    ) -> ShellResult<ShellAction> {
        match mode {
            ExecutionMode::Normal | ExecutionMode::Capture => {
                let result = builtin.execute(state, args).await?;

                match result {
                    ShellAction::Continue(output) => {
                        let plan = match build_redirection_plan(redirections) {
                            Ok(plan) => plan,
                            Err(err) => {
                                return Ok(ShellAction::continue_with(CommandOutput {
                                    exit_code: ExitCode::FAILURE,
                                    stdout: String::new(),
                                    stderr: format!("{err}\n"),
                                }));
                            }
                        };

                        let redirected = match apply_builtin_redirections(output, &plan) {
                            Ok(output) => output,
                            Err(err) => CommandOutput {
                                exit_code: ExitCode::FAILURE,
                                stdout: String::new(),
                                stderr: format!("{err}\n"),
                            },
                        };

                        Ok(ShellAction::continue_with(redirected))
                    }
                    ShellAction::Exit(code) => Ok(ShellAction::Exit(code)),
                }
            }
            ExecutionMode::Pipeline => {
                let isolated_state = clone_shell_state_for_pipeline(&state).await;
                let result = builtin.execute(isolated_state, args).await?;

                match result {
                    ShellAction::Continue(output) => {
                        let plan = match build_redirection_plan(redirections) {
                            Ok(plan) => plan,
                            Err(err) => {
                                return Ok(ShellAction::continue_with(CommandOutput {
                                    exit_code: ExitCode::FAILURE,
                                    stdout: String::new(),
                                    stderr: format!("{err}\n"),
                                }));
                            }
                        };

                        let redirected = match apply_builtin_redirections(output, &plan) {
                            Ok(output) => output,
                            Err(err) => CommandOutput {
                                exit_code: ExitCode::FAILURE,
                                stdout: String::new(),
                                stderr: format!("{err}\n"),
                            },
                        };

                        Ok(ShellAction::continue_with(redirected))
                    }
                    ShellAction::Exit(code) => Ok(ShellAction::Exit(code)),
                }
            }
        }
    }

    async fn execute_pipeline(
        &self,
        state: SharedShellState,
        commands: &[CommandNode],
    ) -> ShellResult<ShellAction> {
        if commands.is_empty() {
            return Ok(ShellAction::continue_with(CommandOutput::success()));
        }

        let mut stdin_buffer: Option<Vec<u8>> = None;
        let mut job_context = PipelineJobContext {
            job_id: None,
            pgid: None,
            summary: summarize_pipeline(commands),
            foreground_claimed: false,
        };
        let mut last_output = PipelineOutput {
            exit_code: ExitCode::SUCCESS,
            stdout: Vec::new(),
            stderr: Vec::new(),
        };

        let result = async {
            for command in commands {
                last_output = self
                    .execute_pipeline_segment(
                        state.clone(),
                        command,
                        stdin_buffer.take(),
                        &mut job_context,
                    )
                    .await?;
                state
                    .write()
                    .await
                    .set_last_exit_status(last_output.exit_code);
                stdin_buffer = Some(last_output.stdout.clone());
            }

            Ok(ShellAction::continue_with(CommandOutput {
                exit_code: last_output.exit_code,
                stdout: String::from_utf8_lossy(&last_output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&last_output.stderr).into_owned(),
            }))
        }
        .await;

        if job_context.foreground_claimed {
            let _ = unix::reclaim_terminal_for_shell();
        }

        result
    }

    async fn execute_pipeline_segment(
        &self,
        state: SharedShellState,
        node: &CommandNode,
        stdin_data: Option<Vec<u8>>,
        job_context: &mut PipelineJobContext,
    ) -> ShellResult<PipelineOutput> {
        match node {
            CommandNode::Simple(simple) => {
                let (expanded_argv, expanded_redirections) =
                    expand_simple_command(state.clone(), simple).await?;

                let Some((name, args)) = expanded_argv.split_first() else {
                    return Ok(PipelineOutput {
                        exit_code: ExitCode::SUCCESS,
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    });
                };

                let function = {
                    let guard = state.read().await;
                    guard.functions().get(name).cloned()
                };

                if let Some(function) = function {
                    let action = self
                        .execute_shell_function(
                            state,
                            name,
                            &function,
                            &expanded_redirections,
                            ExecutionMode::Pipeline,
                        )
                        .await?;

                    return Ok(match action {
                        ShellAction::Continue(output) => PipelineOutput {
                            exit_code: output.exit_code,
                            stdout: output.stdout.into_bytes(),
                            stderr: output.stderr.into_bytes(),
                        },
                        ShellAction::Exit(code) => PipelineOutput {
                            exit_code: code,
                            stdout: Vec::new(),
                            stderr: Vec::new(),
                        },
                    });
                }

                let registry = BuiltinRegistry::defaults();

                if let Some(builtin) = registry.get(name) {
                    let isolated_state = clone_shell_state_for_pipeline(&state).await;
                    let result = builtin.execute(isolated_state, args).await?;

                    return Ok(match result {
                        ShellAction::Continue(output) => {
                            let plan = match build_redirection_plan(&expanded_redirections) {
                                Ok(plan) => plan,
                                Err(err) => {
                                    return Ok(PipelineOutput {
                                        exit_code: ExitCode::FAILURE,
                                        stdout: Vec::new(),
                                        stderr: format!("{err}\n").into_bytes(),
                                    });
                                }
                            };

                            let redirected = match apply_builtin_redirections(output, &plan) {
                                Ok(output) => output,
                                Err(err) => CommandOutput {
                                    exit_code: ExitCode::FAILURE,
                                    stdout: String::new(),
                                    stderr: format!("{err}\n"),
                                },
                            };

                            PipelineOutput {
                                exit_code: redirected.exit_code,
                                stdout: redirected.stdout.into_bytes(),
                                stderr: redirected.stderr.into_bytes(),
                            }
                        }
                        ShellAction::Exit(code) => PipelineOutput {
                            exit_code: code,
                            stdout: Vec::new(),
                            stderr: Vec::new(),
                        },
                    });
                }

                self.execute_external_pipeline_segment(
                    state,
                    name,
                    args,
                    &expanded_redirections,
                    stdin_data,
                    job_context,
                )
                .await
            }
            CommandNode::FunctionDef { name, body } => {
                let action = self
                    .execute_function_definition(state, name, body, ExecutionMode::Pipeline)
                    .await?;

                Ok(match action {
                    ShellAction::Continue(output) => PipelineOutput {
                        exit_code: output.exit_code,
                        stdout: output.stdout.into_bytes(),
                        stderr: output.stderr.into_bytes(),
                    },
                    ShellAction::Exit(code) => PipelineOutput {
                        exit_code: code,
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    },
                })
            }
            CommandNode::Subshell(expr) => {
                let action = self
                    .execute_subshell_placeholder(state, expr, ExecutionMode::Pipeline)
                    .await?;

                Ok(match action {
                    ShellAction::Continue(output) => PipelineOutput {
                        exit_code: output.exit_code,
                        stdout: output.stdout.into_bytes(),
                        stderr: output.stderr.into_bytes(),
                    },
                    ShellAction::Exit(code) => PipelineOutput {
                        exit_code: code,
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    },
                })
            }
        }
    }

    async fn execute_external_pipeline_segment(
        &self,
        state: SharedShellState,
        program: &str,
        args: &[String],
        redirections: &[ExpandedRedirection],
        stdin_data: Option<Vec<u8>>,
        job_context: &mut PipelineJobContext,
    ) -> ShellResult<PipelineOutput> {
        let (cwd, env_map) = {
            let guard = state.read().await;
            (guard.cwd().to_path_buf(), guard.env().clone())
        };

        let resolved = match resolve_command_path(program, &env_map) {
            Some(path) => path,
            None => {
                return Ok(PipelineOutput {
                    exit_code: ExitCode::FAILURE,
                    stdout: Vec::new(),
                    stderr: format!("command not found: {program}\n").into_bytes(),
                });
            }
        };

        let plan = match build_redirection_plan(redirections) {
            Ok(plan) => plan,
            Err(err) => {
                return Ok(PipelineOutput {
                    exit_code: ExitCode::FAILURE,
                    stdout: Vec::new(),
                    stderr: format!("{err}\n").into_bytes(),
                });
            }
        };

        let mut command = Command::new(&resolved);
        command.args(args);
        command.current_dir(cwd);
        command.env_clear();
        command.envs(env_map);
        unix::configure_process_group(&mut command, job_context.pgid);

        if let Some(InputRedirection::File(path)) = &plan.stdin {
            match open_input_file(path) {
                Ok(file) => {
                    command.stdin(Stdio::from(file));
                }
                Err(err) => {
                    return Ok(PipelineOutput {
                        exit_code: ExitCode::FAILURE,
                        stdout: Vec::new(),
                        stderr: format!("failed to open input file {}: {err}\n", path.display())
                            .into_bytes(),
                    });
                }
            }
        } else if matches!(plan.stdin, Some(InputRedirection::Inline(_))) || stdin_data.is_some() {
            command.stdin(Stdio::piped());
        } else {
            command.stdin(Stdio::null());
        }

        if let Some(redir) = &plan.stdout {
            match open_output_file(redir) {
                Ok(file) => {
                    command.stdout(Stdio::from(file));
                }
                Err(err) => {
                    return Ok(PipelineOutput {
                        exit_code: ExitCode::FAILURE,
                        stdout: Vec::new(),
                        stderr: format!(
                            "failed to open output file {}: {err}\n",
                            redir.path.display()
                        )
                        .into_bytes(),
                    });
                }
            }
        } else {
            command.stdout(Stdio::piped());
        }

        if let Some(redir) = &plan.stderr {
            match open_output_file(redir) {
                Ok(file) => {
                    command.stderr(Stdio::from(file));
                }
                Err(err) => {
                    return Ok(PipelineOutput {
                        exit_code: ExitCode::FAILURE,
                        stdout: Vec::new(),
                        stderr: format!(
                            "failed to open error file {}: {err}\n",
                            redir.path.display()
                        )
                        .into_bytes(),
                    });
                }
            }
        } else {
            command.stderr(Stdio::piped());
        }

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => {
                return Ok(PipelineOutput {
                    exit_code: ExitCode::FAILURE,
                    stdout: Vec::new(),
                    stderr: format!("failed to execute {program}: {err}\n").into_bytes(),
                });
            }
        };

        let child_pid = child.id().unwrap_or_default();
        register_pipeline_process(
            state.clone(),
            job_context,
            child_pid,
            summarize_command(program, args),
        )
        .await;

        if !job_context.foreground_claimed
            && let Some(pgid) = job_context.pgid
        {
            job_context.foreground_claimed = unix::hand_terminal_to_foreground_job(pgid)?;
        }

        let stdin_bytes = match (&plan.stdin, stdin_data) {
            (Some(InputRedirection::Inline(input)), _) => Some(input.clone()),
            (Some(InputRedirection::File(_)), _) => None,
            (None, input) => input,
        };

        if let Some(input) = stdin_bytes
            && let Some(mut stdin) = child.stdin.take()
        {
            stdin.write_all(&input).await?;
        }

        let stdout_task = spawn_pipe_reader(child.stdout.take());
        let stderr_task = spawn_pipe_reader(child.stderr.take());
        let wait_status = unix::wait_for_foreground_process(child_pid).await?;

        let (stdout, stderr) = match wait_status {
            unix::ForegroundWaitStatus::Stopped(_) => {
                abort_pipe_reader(stdout_task.as_ref());
                abort_pipe_reader(stderr_task.as_ref());
                (Vec::new(), Vec::new())
            }
            unix::ForegroundWaitStatus::Exited(_) | unix::ForegroundWaitStatus::Signaled(_) => {
                let stdout = join_pipe_reader(stdout_task).await?;
                let stderr = join_pipe_reader(stderr_task).await?;
                (stdout, stderr)
            }
        };

        let (process_state, exit_code) = process_state_from_wait_status(wait_status);

        if matches!(process_state, ProcessState::Stopped) {
            let child_handles = { state.read().await.child_handles().clone() };
            child_handles.insert(child_pid, child).await;
        }

        if job_context.foreground_claimed {
            let _ = unix::reclaim_terminal_for_shell();
            job_context.foreground_claimed = false;
        }

        if matches!(
            wait_status,
            unix::ForegroundWaitStatus::Exited(_) | unix::ForegroundWaitStatus::Signaled(_)
        ) && job_context.pgid == Some(child_pid)
        {
            job_context.pgid = None;
        }

        if let Some(job_id) = job_context.job_id {
            let _ = state.write().await.jobs_mut().update_process_state(
                job_id,
                child_pid,
                process_state,
            );
        }

        Ok(PipelineOutput {
            exit_code,
            stdout,
            stderr,
        })
    }

    async fn execute_boolean_chain(
        &self,
        state: SharedShellState,
        first: &ShellExpr,
        rest: &[(BoolOp, ShellExpr)],
        mode: ExecutionMode,
    ) -> ShellResult<ShellAction> {
        let first_action = self.execute_expr(state.clone(), first, mode).await?;
        let mut aggregate = match first_action {
            ShellAction::Continue(output) => {
                state.write().await.set_last_exit_status(output.exit_code);
                output
            }
            ShellAction::Exit(code) => return Ok(ShellAction::Exit(code)),
        };

        for (op, expr) in rest {
            let should_run = match op {
                BoolOp::And => aggregate.exit_code.is_success(),
                BoolOp::Or => aggregate.exit_code.is_failure(),
            };

            if should_run {
                let next_action = self.execute_expr(state.clone(), expr, mode).await?;
                match next_action {
                    ShellAction::Continue(output) => {
                        state.write().await.set_last_exit_status(output.exit_code);
                        aggregate = merge_outputs(aggregate, output);
                    }
                    ShellAction::Exit(code) => return Ok(ShellAction::Exit(code)),
                }
            }
        }

        Ok(ShellAction::continue_with(aggregate))
    }

    async fn execute_sequence(
        &self,
        state: SharedShellState,
        exprs: &[ShellExpr],
        mode: ExecutionMode,
    ) -> ShellResult<ShellAction> {
        let mut aggregate = CommandOutput::success();

        for expr in exprs {
            let action = self.execute_expr(state.clone(), expr, mode).await?;

            match action {
                ShellAction::Continue(output) => {
                    state.write().await.set_last_exit_status(output.exit_code);
                    aggregate = merge_outputs(aggregate, output);
                }
                ShellAction::Exit(code) => return Ok(ShellAction::Exit(code)),
            }
        }

        Ok(ShellAction::continue_with(aggregate))
    }
}

async fn execute_external(
    state: SharedShellState,
    program: &str,
    args: &[String],
    redirections: &[ExpandedRedirection],
) -> ShellResult<ShellAction> {
    let (cwd, env_map) = {
        let guard = state.read().await;
        (guard.cwd().to_path_buf(), guard.env().clone())
    };

    let resolved = match resolve_command_path(program, &env_map) {
        Some(path) => path,
        None => {
            return Ok(ShellAction::continue_with(CommandOutput {
                exit_code: ExitCode::FAILURE,
                stdout: String::new(),
                stderr: format!("command not found: {program}\n"),
            }));
        }
    };

    let plan = match build_redirection_plan(redirections) {
        Ok(plan) => plan,
        Err(err) => {
            return Ok(ShellAction::continue_with(CommandOutput {
                exit_code: ExitCode::FAILURE,
                stdout: String::new(),
                stderr: format!("{err}\n"),
            }));
        }
    };

    let mut command = Command::new(&resolved);
    command.args(args);
    command.current_dir(cwd);
    command.env_clear();
    command.envs(env_map);
    unix::configure_process_group(&mut command, None);

    if let Some(InputRedirection::File(path)) = &plan.stdin {
        match open_input_file(path) {
            Ok(file) => {
                command.stdin(Stdio::from(file));
            }
            Err(err) => {
                return Ok(ShellAction::continue_with(CommandOutput {
                    exit_code: ExitCode::FAILURE,
                    stdout: String::new(),
                    stderr: format!("failed to open input file {}: {err}\n", path.display()),
                }));
            }
        }
    } else if matches!(plan.stdin, Some(InputRedirection::Inline(_))) {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::inherit());
    }

    if let Some(redir) = &plan.stdout {
        match open_output_file(redir) {
            Ok(file) => {
                command.stdout(Stdio::from(file));
            }
            Err(err) => {
                return Ok(ShellAction::continue_with(CommandOutput {
                    exit_code: ExitCode::FAILURE,
                    stdout: String::new(),
                    stderr: format!(
                        "failed to open output file {}: {err}\n",
                        redir.path.display()
                    ),
                }));
            }
        }
    } else {
        command.stdout(Stdio::inherit());
    }

    if let Some(redir) = &plan.stderr {
        match open_output_file(redir) {
            Ok(file) => {
                command.stderr(Stdio::from(file));
            }
            Err(err) => {
                return Ok(ShellAction::continue_with(CommandOutput {
                    exit_code: ExitCode::FAILURE,
                    stdout: String::new(),
                    stderr: format!(
                        "failed to open error file {}: {err}\n",
                        redir.path.display()
                    ),
                }));
            }
        }
    } else {
        command.stderr(Stdio::inherit());
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(err) => {
            return Ok(ShellAction::continue_with(CommandOutput {
                exit_code: ExitCode::FAILURE,
                stdout: String::new(),
                stderr: format!("failed to execute {program}: {err}\n"),
            }));
        }
    };

    let child_pid = child.id().unwrap_or_default();
    let job_id =
        register_foreground_job(state.clone(), child_pid, summarize_command(program, args)).await;
    let foreground_claimed = unix::hand_terminal_to_foreground_job(child_pid)?;

    if let Some(InputRedirection::Inline(input)) = &plan.stdin
        && let Some(mut stdin) = child.stdin.take()
    {
        stdin.write_all(input).await?;
    }

    let wait_result = unix::wait_for_foreground_process(child_pid).await;

    if foreground_claimed {
        let _ = unix::reclaim_terminal_for_shell();
    }

    let wait_status = match wait_result {
        Ok(status) => status,
        Err(err) => {
            return Ok(ShellAction::continue_with(CommandOutput {
                exit_code: ExitCode::FAILURE,
                stdout: String::new(),
                stderr: format!("failed to execute {program}: {err}\n"),
            }));
        }
    };

    let (process_state, exit_code) = process_state_from_wait_status(wait_status);

    if matches!(process_state, ProcessState::Stopped) {
        let child_handles = { state.read().await.child_handles().clone() };
        child_handles.insert(child_pid, child).await;
    }

    if let Some(job_id) = job_id {
        let mut guard = state.write().await;
        let _ = guard
            .jobs_mut()
            .update_process_state(job_id, child_pid, process_state);
    }

    if matches!(process_state, ProcessState::Completed(_)) {
        let child_handles = { state.read().await.child_handles().clone() };
        let _ = child_handles.remove(child_pid).await;
    }

    Ok(ShellAction::continue_with(CommandOutput {
        exit_code,
        stdout: String::new(),
        stderr: String::new(),
    }))
}

fn resolve_command_path(program: &str, env_map: &HashMap<String, String>) -> Option<PathBuf> {
    let candidate = Path::new(program);

    if candidate.components().count() > 1 {
        return is_executable_file(candidate).then(|| candidate.to_path_buf());
    }

    let path_var = env_map
        .get("PATH")
        .cloned()
        .unwrap_or_else(|| env::var("PATH").unwrap_or_default());

    env::split_paths(&OsString::from(path_var))
        .map(|dir| dir.join(program))
        .find(|path| is_executable_file(path))
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };

    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        false
    }
}

fn build_redirection_plan(redirections: &[ExpandedRedirection]) -> ShellResult<RedirectionPlan> {
    let mut plan = RedirectionPlan::empty();

    for redirect in redirections {
        let fd = redirect.effective_fd();

        match (&redirect.kind, fd) {
            (RedirectionKind::Input, 0) => {
                plan.stdin = Some(InputRedirection::File(PathBuf::from(
                    redirect.target.as_deref().unwrap_or_default(),
                )));
            }
            (RedirectionKind::HereDoc { body, .. }, 0) => {
                plan.stdin = Some(InputRedirection::Inline(body.as_bytes().to_vec()));
            }
            (RedirectionKind::OutputTruncate, 1) => {
                plan.stdout = Some(OutputRedirection {
                    path: PathBuf::from(redirect.target.as_deref().unwrap_or_default()),
                    append: false,
                });
            }
            (RedirectionKind::OutputAppend, 1) => {
                plan.stdout = Some(OutputRedirection {
                    path: PathBuf::from(redirect.target.as_deref().unwrap_or_default()),
                    append: true,
                });
            }
            (RedirectionKind::OutputTruncate, 2) => {
                plan.stderr = Some(OutputRedirection {
                    path: PathBuf::from(redirect.target.as_deref().unwrap_or_default()),
                    append: false,
                });
            }
            (RedirectionKind::OutputAppend, 2) => {
                plan.stderr = Some(OutputRedirection {
                    path: PathBuf::from(redirect.target.as_deref().unwrap_or_default()),
                    append: true,
                });
            }
            (RedirectionKind::Input, fd) => {
                return Err(crate::shell::ShellError::message(format!(
                    "unsupported input redirection fd: {fd}"
                )));
            }
            (_, fd) => {
                return Err(crate::shell::ShellError::message(format!(
                    "unsupported redirection fd: {fd}"
                )));
            }
        }
    }

    Ok(plan)
}

fn open_input_file(path: &PathBuf) -> ShellResult<File> {
    File::open(path).map_err(crate::shell::ShellError::from)
}

fn open_output_file(redirection: &OutputRedirection) -> ShellResult<File> {
    let mut options = OpenOptions::new();
    options.create(true).write(true);

    if redirection.append {
        options.append(true);
    } else {
        options.truncate(true);
    }

    options
        .open(&redirection.path)
        .map_err(crate::shell::ShellError::from)
}

fn apply_builtin_redirections(
    output: CommandOutput,
    plan: &RedirectionPlan,
) -> ShellResult<CommandOutput> {
    if let Some(stdout_redirect) = &plan.stdout {
        let mut file = open_output_file(stdout_redirect)?;
        use std::io::Write;
        file.write_all(output.stdout.as_bytes())?;
    }

    if let Some(stderr_redirect) = &plan.stderr {
        let mut file = open_output_file(stderr_redirect)?;
        use std::io::Write;
        file.write_all(output.stderr.as_bytes())?;
    }

    Ok(CommandOutput {
        exit_code: output.exit_code,
        stdout: if plan.stdout.is_some() {
            String::new()
        } else {
            output.stdout
        },
        stderr: if plan.stderr.is_some() {
            String::new()
        } else {
            output.stderr
        },
    })
}

async fn clone_shell_state_for_pipeline(state: &SharedShellState) -> SharedShellState {
    let snapshot = state.read().await.clone();
    Arc::new(RwLock::new(snapshot))
}

fn merge_outputs(mut left: CommandOutput, right: CommandOutput) -> CommandOutput {
    left.stdout.push_str(&right.stdout);
    left.stderr.push_str(&right.stderr);
    left.exit_code = right.exit_code;
    left
}

fn process_state_from_wait_status(status: unix::ForegroundWaitStatus) -> (ProcessState, ExitCode) {
    match status {
        unix::ForegroundWaitStatus::Exited(code) => {
            let code = code.clamp(0, i32::from(u8::MAX)) as u8;
            (
                ProcessState::Completed(i32::from(code)),
                ExitCode::new(code),
            )
        }
        unix::ForegroundWaitStatus::Signaled(signal) => {
            let code = signal_exit_code(signal);
            (ProcessState::Completed(i32::from(code.as_u8())), code)
        }
        unix::ForegroundWaitStatus::Stopped(signal) => {
            (ProcessState::Stopped, signal_exit_code(signal))
        }
    }
}

fn process_state_from_exit_status(status: std::process::ExitStatus) -> ProcessState {
    if let Some(code) = status.code() {
        return ProcessState::Completed(code.clamp(0, i32::from(u8::MAX)));
    }

    #[cfg(unix)]
    let signal = status.signal().unwrap_or(1);
    #[cfg(not(unix))]
    let signal = 1;
    let code = signal_exit_code(signal);
    ProcessState::Completed(i32::from(code.as_u8()))
}

fn signal_exit_code(signal: i32) -> ExitCode {
    let code = 128_i32.saturating_add(signal).clamp(1, i32::from(u8::MAX)) as u8;
    ExitCode::new(code)
}

fn spawn_pipe_reader<T>(reader: Option<T>) -> Option<JoinHandle<io::Result<Vec<u8>>>>
where
    T: AsyncRead + Unpin + Send + 'static,
{
    reader.map(|mut reader| {
        tokio::spawn(async move {
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer).await?;
            Ok(buffer)
        })
    })
}

fn abort_pipe_reader(handle: Option<&JoinHandle<io::Result<Vec<u8>>>>) {
    if let Some(handle) = handle {
        handle.abort();
    }
}

async fn join_pipe_reader(handle: Option<JoinHandle<io::Result<Vec<u8>>>>) -> ShellResult<Vec<u8>> {
    match handle {
        Some(handle) => handle
            .await
            .map_err(|err| crate::shell::ShellError::message(format!("pipe reader failed: {err}")))?
            .map_err(crate::shell::ShellError::from),
        None => Ok(Vec::new()),
    }
}

async fn register_foreground_job(
    state: SharedShellState,
    pid: u32,
    summary: String,
) -> Option<JobId> {
    if pid == 0 {
        return None;
    }

    let job_id = state.write().await.jobs_mut().insert(
        pid,
        summary.clone(),
        JobDisposition::Foreground,
        vec![ProcessRecord::new(pid, summary)],
    );

    Some(job_id)
}

async fn register_pipeline_process(
    state: SharedShellState,
    context: &mut PipelineJobContext,
    pid: u32,
    summary: String,
) {
    if pid == 0 {
        return;
    }

    let pgid = context.pgid.unwrap_or(pid);
    context.pgid = Some(pgid);

    let mut guard = state.write().await;
    if let Some(job_id) = context.job_id {
        let _ = guard
            .jobs_mut()
            .add_process(job_id, ProcessRecord::new(pid, summary));
    } else {
        let job_id = guard.jobs_mut().insert(
            pgid,
            context.summary.clone(),
            JobDisposition::Foreground,
            vec![ProcessRecord::new(pid, summary)],
        );
        context.job_id = Some(job_id);
    }
}

fn summarize_command(program: &str, args: &[String]) -> String {
    if args.is_empty() {
        program.to_string()
    } else {
        format!("{} {}", program, args.join(" "))
    }
}

fn summarize_pipeline(commands: &[CommandNode]) -> String {
    commands
        .iter()
        .map(|node| match node {
            CommandNode::Simple(simple) => simple
                .argv
                .iter()
                .map(Word::quote_removed_text)
                .collect::<Vec<_>>()
                .join(" "),
            CommandNode::FunctionDef { name, .. } => format!("{name}()"),
            CommandNode::Subshell(_) => "(subshell)".to_string(),
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

async fn expand_simple_command(
    state: SharedShellState,
    simple: &SimpleCommand,
) -> ShellResult<(Vec<String>, Vec<ExpandedRedirection>)> {
    // Expansion order is: alias rewrite of the command word, assignment-prefix env updates,
    // variable/command substitution, then pathname globbing on argv only.
    let simple = resolve_aliases(state.clone(), simple).await?;
    let substitution_executor: CommandSubstitutionExecutor = Arc::new(move |state, expr| {
        let executor = BootstrapExecutor;
        Box::pin(async move { executor.execute_command_substitution(state, expr).await })
    });

    for (name, value) in &simple.assignments {
        let expanded = expand_words_with_state(
            state.clone(),
            std::slice::from_ref(value),
            &substitution_executor,
        )
        .await?
        .into_iter()
        .next()
        .unwrap_or_default();

        state.write().await.set_env_var(name.clone(), expanded);
    }

    let argv =
        expand_words_pathnames_with_state(state.clone(), &simple.argv, &substitution_executor)
            .await?;

    let mut redirections = Vec::new();
    for redirection in &simple.redirections {
        let target = expand_words_with_state(
            state.clone(),
            std::slice::from_ref(&redirection.target),
            &substitution_executor,
        )
        .await?
        .into_iter()
        .next()
        .unwrap_or_default();

        let fd = redirection
            .fd
            .map(|fd| {
                u8::try_from(fd).map_err(|_| {
                    crate::shell::ShellError::message(format!("unsupported redirection fd: {fd}"))
                })
            })
            .transpose()?;

        redirections.push(ExpandedRedirection {
            fd,
            kind: match &redirection.kind {
                RedirectionKind::HereDoc { body, expand } => RedirectionKind::HereDoc {
                    body: if *expand {
                        expand_heredoc_body(state.clone(), body).await?
                    } else {
                        body.clone()
                    },
                    expand: *expand,
                },
                other => other.clone(),
            },
            target: match &redirection.kind {
                RedirectionKind::HereDoc { .. } => None,
                _ => Some(target),
            },
        });
    }

    Ok((argv, redirections))
}

async fn resolve_aliases(
    state: SharedShellState,
    simple: &SimpleCommand,
) -> ShellResult<SimpleCommand> {
    let mut current = simple.clone();
    let mut seen = std::collections::HashSet::new();

    loop {
        let Some(first) = current.argv.first() else {
            return Ok(current);
        };

        let Some(name) = alias_candidate_name(first) else {
            return Ok(current);
        };

        let alias_value = {
            let guard = state.read().await;
            guard.aliases().get(name).map(str::to_owned)
        };

        let Some(alias_value) = alias_value else {
            return Ok(current);
        };

        if !seen.insert(name.to_string()) {
            return Ok(current);
        }

        let Some(alias_simple) = parse_alias_simple_command(&alias_value)? else {
            return Ok(current);
        };

        let mut argv = alias_simple.argv;
        argv.extend(current.argv.iter().skip(1).cloned());

        let mut redirections = alias_simple.redirections;
        redirections.extend(current.redirections);

        current = SimpleCommand::with_assignments(current.assignments, argv, redirections);
    }
}

fn alias_candidate_name(word: &Word) -> Option<&str> {
    word.as_unquoted_literal()
}

fn parse_alias_simple_command(alias_value: &str) -> ShellResult<Option<SimpleCommand>> {
    let parsed = Parser::default()
        .parse(alias_value)
        .map_err(|err| ShellError::message(format!("invalid alias expansion: {err}")))?;

    match parsed {
        ParsedCommand::Empty => Ok(None),
        ParsedCommand::Expr(ShellExpr::Command(CommandNode::Simple(simple))) => Ok(Some(simple)),
        _ => Ok(None),
    }
}

async fn expand_heredoc_body(state: SharedShellState, body: &str) -> ShellResult<String> {
    let mut out = String::new();
    let mut chars = body.chars().peekable();
    let substitution_executor: CommandSubstitutionExecutor = Arc::new(move |state, expr| {
        let executor = BootstrapExecutor;
        Box::pin(async move { executor.execute_command_substitution(state, expr).await })
    });

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => match chars.peek().copied() {
                Some('$') | Some('\\') => {
                    out.push(chars.next().expect("peeked character should exist"));
                }
                _ => out.push('\\'),
            },
            '$' => match chars.peek().copied() {
                Some('?') => {
                    chars.next();
                    let status = state.read().await.last_exit_status().as_u8().to_string();
                    out.push_str(&status);
                }
                Some(next) if is_var_start(next) => {
                    let mut name = String::new();
                    while let Some(next) = chars.peek().copied() {
                        if is_var_continue(next) {
                            name.push(next);
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    let value = {
                        let guard = state.read().await;
                        guard.env_var(&name).map(ToOwned::to_owned)
                    };

                    if let Some(value) = value {
                        out.push_str(&value);
                    }
                }
                Some('(') => {
                    chars.next();
                    let source = collect_command_substitution_source(&mut chars)?;
                    let expr = parse_command_substitution_source(&source)?;
                    let substituted = substitution_executor(state.clone(), *expr).await?;
                    out.push_str(&normalize_command_substitution_output(substituted));
                }
                _ => out.push('$'),
            },
            _ => out.push(ch),
        }
    }

    Ok(out)
}

fn is_var_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_var_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn collect_command_substitution_source<I>(chars: &mut std::iter::Peekable<I>) -> ShellResult<String>
where
    I: Iterator<Item = char>,
{
    let mut out = String::new();
    let mut depth = 1usize;

    while let Some(ch) = chars.next() {
        match ch {
            '\'' => {
                out.push(ch);
                collect_raw_single_quoted(chars, &mut out)?;
            }
            '"' => {
                out.push(ch);
                collect_raw_double_quoted(chars, &mut out)?;
            }
            '\\' => {
                out.push(ch);
                match chars.next() {
                    Some(next) => out.push(next),
                    None => {
                        return Err(ShellError::message("unterminated command substitution"));
                    }
                }
            }
            '$' if chars.peek() == Some(&'(') => {
                out.push('$');
                out.push('(');
                chars.next();
                depth += 1;
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(out);
                }
                out.push(')');
            }
            other => out.push(other),
        }
    }

    Err(ShellError::message("unterminated command substitution"))
}

fn collect_raw_single_quoted<I>(
    chars: &mut std::iter::Peekable<I>,
    out: &mut String,
) -> ShellResult<()>
where
    I: Iterator<Item = char>,
{
    loop {
        match chars.next() {
            Some('\'') => {
                out.push('\'');
                return Ok(());
            }
            Some(c) => out.push(c),
            None => return Err(ShellError::message("unterminated single-quoted string")),
        }
    }
}

fn collect_raw_double_quoted<I>(
    chars: &mut std::iter::Peekable<I>,
    out: &mut String,
) -> ShellResult<()>
where
    I: Iterator<Item = char>,
{
    loop {
        match chars.next() {
            Some('"') => {
                out.push('"');
                return Ok(());
            }
            Some('\\') => {
                out.push('\\');
                match chars.next() {
                    Some(next) => out.push(next),
                    None => {
                        return Err(ShellError::message(
                            "unterminated escape in double-quoted string",
                        ));
                    }
                }
            }
            Some(c) => out.push(c),
            None => return Err(ShellError::message("unterminated double-quoted string")),
        }
    }
}

fn parse_command_substitution_source(source: &str) -> ShellResult<Box<ShellExpr>> {
    match Parser::default()
        .parse(source)
        .map_err(|err| ShellError::message(err.to_string()))?
    {
        ParsedCommand::Expr(expr) => Ok(Box::new(expr)),
        ParsedCommand::Empty => Ok(Box::new(ShellExpr::Command(CommandNode::Simple(
            SimpleCommand::new(Vec::new()),
        )))),
    }
}

fn normalize_command_substitution_output(mut output: String) -> String {
    while output.ends_with('\n') {
        output.pop();
        if output.ends_with('\r') {
            output.pop();
        }
    }

    output
}
