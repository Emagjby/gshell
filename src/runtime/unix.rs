#[cfg(unix)]
mod imp {
    use std::{fs::File, io, sync::OnceLock};

    use nix::{
        sys::signal::{SigSet, SigmaskHow, Signal, pthread_sigmask},
        sys::signal::{kill, killpg},
        sys::wait::{WaitPidFlag, WaitStatus, waitpid},
        unistd::{Pid, getpgrp, getpid, setpgid, tcgetpgrp, tcsetpgrp},
    };
    use tokio::task;

    use crate::shell::{ShellError, ShellResult};

    #[derive(Debug)]
    struct JobControl {
        tty: File,
        shell_pgid: Pid,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum ForegroundWaitStatus {
        Exited(i32),
        Signaled(i32),
        Stopped(i32),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum PolledWaitStatus {
        Exited { pid: u32, code: i32 },
        Signaled { pid: u32, signal: i32 },
        Stopped { pid: u32, signal: i32 },
        Continued { pid: u32 },
    }

    static JOB_CONTROL: OnceLock<Option<JobControl>> = OnceLock::new();

    pub(crate) async fn initialize_interactive_shell() -> ShellResult<()> {
        let _ = JOB_CONTROL.get_or_init(setup_job_control);
        Ok(())
    }

    pub(crate) fn configure_process_group(
        command: &mut tokio::process::Command,
        pgid: Option<u32>,
    ) {
        let Ok(group) = i32::try_from(pgid.unwrap_or(0)) else {
            return;
        };

        command.process_group(group);
    }

    pub(crate) fn hand_terminal_to_foreground_job(pgid: u32) -> ShellResult<bool> {
        let Some(control) = JOB_CONTROL.get_or_init(setup_job_control).as_ref() else {
            return Ok(false);
        };

        let Some(pgid) = pid_from_u32(pgid) else {
            return Ok(false);
        };

        with_blocked_job_control_signals(|| tcsetpgrp(&control.tty, pgid)).map_err(nix_err)?;
        Ok(true)
    }

    pub(crate) fn reclaim_terminal_for_shell() -> ShellResult<()> {
        let Some(control) = JOB_CONTROL.get_or_init(setup_job_control).as_ref() else {
            return Ok(());
        };

        with_blocked_job_control_signals(|| tcsetpgrp(&control.tty, control.shell_pgid))
            .map_err(nix_err)
    }

    pub(crate) fn continue_process_group(pgid: u32) -> ShellResult<()> {
        let Some(pgid) = pid_from_u32(pgid) else {
            return Ok(());
        };

        killpg(pgid, Signal::SIGCONT).map_err(nix_err)
    }

    pub(crate) fn signal_process_group(pgid: u32, signal: i32) -> ShellResult<()> {
        let Some(pgid) = pid_from_u32(pgid) else {
            return Ok(());
        };
        let signal = Signal::try_from(signal)
            .map_err(|_| ShellError::message(format!("unsupported signal: {signal}")))?;

        killpg(pgid, signal).map_err(nix_err)
    }

    pub(crate) fn signal_process(pid: u32, signal: i32) -> ShellResult<()> {
        let Some(pid) = pid_from_u32(pid) else {
            return Ok(());
        };
        let signal = Signal::try_from(signal)
            .map_err(|_| ShellError::message(format!("unsupported signal: {signal}")))?;

        kill(pid, signal).map_err(nix_err)
    }

    pub(crate) async fn wait_for_foreground_process(pid: u32) -> ShellResult<ForegroundWaitStatus> {
        let Some(pid) = pid_from_u32(pid) else {
            return Ok(ForegroundWaitStatus::Exited(1));
        };

        task::spawn_blocking(move || {
            loop {
                match waitpid(pid, Some(WaitPidFlag::WUNTRACED)) {
                    Ok(WaitStatus::Exited(_, code)) => {
                        return Ok(ForegroundWaitStatus::Exited(code));
                    }
                    Ok(WaitStatus::Signaled(_, signal, _)) => {
                        return Ok(ForegroundWaitStatus::Signaled(signal as i32));
                    }
                    Ok(WaitStatus::Stopped(_, signal)) => {
                        return Ok(ForegroundWaitStatus::Stopped(signal as i32));
                    }
                    Ok(WaitStatus::StillAlive | WaitStatus::Continued(_)) => continue,
                    Ok(_) => continue,
                    Err(err) => return Err(err),
                }
            }
        })
        .await
        .map_err(|err| ShellError::message(format!("failed to join wait task: {err}")))?
        .map_err(nix_err)
    }

    pub(crate) async fn poll_child_statuses() -> ShellResult<Vec<PolledWaitStatus>> {
        task::spawn_blocking(move || {
            let mut statuses = Vec::new();

            loop {
                match waitpid(
                    Pid::from_raw(-1),
                    Some(WaitPidFlag::WNOHANG | WaitPidFlag::WUNTRACED | WaitPidFlag::WCONTINUED),
                ) {
                    Ok(WaitStatus::StillAlive) => return Ok(statuses),
                    Ok(WaitStatus::Exited(pid, code)) => {
                        if let Ok(pid) = u32::try_from(pid.as_raw()) {
                            statuses.push(PolledWaitStatus::Exited { pid, code });
                        }
                    }
                    Ok(WaitStatus::Signaled(pid, signal, _)) => {
                        if let Ok(pid) = u32::try_from(pid.as_raw()) {
                            statuses.push(PolledWaitStatus::Signaled {
                                pid,
                                signal: signal as i32,
                            });
                        }
                    }
                    Ok(WaitStatus::Stopped(pid, signal)) => {
                        if let Ok(pid) = u32::try_from(pid.as_raw()) {
                            statuses.push(PolledWaitStatus::Stopped {
                                pid,
                                signal: signal as i32,
                            });
                        }
                    }
                    Ok(WaitStatus::Continued(pid)) => {
                        if let Ok(pid) = u32::try_from(pid.as_raw()) {
                            statuses.push(PolledWaitStatus::Continued { pid });
                        }
                    }
                    Ok(_) => continue,
                    Err(nix::errno::Errno::ECHILD) => return Ok(statuses),
                    Err(err) => return Err(err),
                }
            }
        })
        .await
        .map_err(|err| ShellError::message(format!("failed to join wait task: {err}")))?
        .map_err(nix_err)
    }

    fn setup_job_control() -> Option<JobControl> {
        let tty = File::options()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .ok()?;
        let pid = getpid();
        if getpgrp() != pid {
            let _ = setpgid(pid, pid);
        }
        let shell_pgid = getpgrp();

        if tcgetpgrp(&tty).ok()? != shell_pgid {
            with_blocked_job_control_signals(|| tcsetpgrp(&tty, shell_pgid)).ok()?;
        }

        Some(JobControl { tty, shell_pgid })
    }

    fn pid_from_u32(pid: u32) -> Option<Pid> {
        i32::try_from(pid).ok().map(Pid::from_raw)
    }

    fn with_blocked_job_control_signals<T, F>(operation: F) -> nix::Result<T>
    where
        F: FnOnce() -> nix::Result<T>,
    {
        let mut signals = SigSet::empty();
        signals.add(Signal::SIGTSTP);
        signals.add(Signal::SIGTTIN);
        signals.add(Signal::SIGTTOU);

        let mut old_mask = SigSet::empty();
        pthread_sigmask(SigmaskHow::SIG_BLOCK, Some(&signals), Some(&mut old_mask))?;
        let result = operation();
        let restore_result = pthread_sigmask(SigmaskHow::SIG_SETMASK, Some(&old_mask), None);

        match (result, restore_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(err), _) => Err(err),
            (Ok(_), Err(err)) => Err(err),
        }
    }

    fn nix_err(err: nix::errno::Errno) -> ShellError {
        ShellError::message(io::Error::from_raw_os_error(err as i32).to_string())
    }
}

#[cfg(unix)]
pub(crate) use imp::*;

#[cfg(not(unix))]
mod imp {
    use crate::shell::ShellResult;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum ForegroundWaitStatus {
        Exited(i32),
        Signaled(i32),
        Stopped(i32),
    }

    pub(crate) async fn initialize_interactive_shell() -> ShellResult<()> {
        Ok(())
    }

    pub(crate) fn configure_process_group(
        _command: &mut tokio::process::Command,
        _pgid: Option<u32>,
    ) {
    }

    pub(crate) fn hand_terminal_to_foreground_job(_pgid: u32) -> ShellResult<bool> {
        Ok(false)
    }

    pub(crate) fn reclaim_terminal_for_shell() -> ShellResult<()> {
        Ok(())
    }

    pub(crate) fn continue_process_group(_pgid: u32) -> ShellResult<()> {
        Ok(())
    }

    pub(crate) fn signal_process_group(_pgid: u32, _signal: i32) -> ShellResult<()> {
        Ok(())
    }

    pub(crate) fn signal_process(_pid: u32, _signal: i32) -> ShellResult<()> {
        Ok(())
    }

    pub(crate) async fn wait_for_foreground_process(
        _pid: u32,
    ) -> ShellResult<ForegroundWaitStatus> {
        Ok(ForegroundWaitStatus::Exited(1))
    }

    pub(crate) async fn poll_child_statuses() -> ShellResult<Vec<PolledWaitStatus>> {
        Ok(Vec::new())
    }
}

#[cfg(not(unix))]
pub(crate) use imp::*;
