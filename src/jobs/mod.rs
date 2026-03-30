use std::collections::BTreeMap;

pub type JobId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobDisposition {
    Foreground,
    Background,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Stopped,
    Completed(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Running,
    Stopped,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessRecord {
    pid: u32,
    state: ProcessState,
    summary: String,
}

impl ProcessRecord {
    pub fn new(pid: u32, summary: impl Into<String>) -> Self {
        Self {
            pid,
            state: ProcessState::Running,
            summary: summary.into(),
        }
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn state(&self) -> ProcessState {
        self.state
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobRecord {
    id: JobId,
    pgid: u32,
    summary: String,
    disposition: JobDisposition,
    state: JobState,
    processes: Vec<ProcessRecord>,
}

impl JobRecord {
    pub fn id(&self) -> JobId {
        self.id
    }

    pub fn pgid(&self) -> u32 {
        self.pgid
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }

    pub fn disposition(&self) -> JobDisposition {
        self.disposition
    }

    pub fn state(&self) -> JobState {
        self.state
    }

    pub fn processes(&self) -> &[ProcessRecord] {
        &self.processes
    }
}

#[derive(Debug, Clone, Default)]
pub struct Jobs {
    next_job_id: JobId,
    foreground_job: Option<JobId>,
    jobs: BTreeMap<JobId, JobRecord>,
}

impl Jobs {
    pub fn insert(
        &mut self,
        pgid: u32,
        summary: impl Into<String>,
        disposition: JobDisposition,
        processes: Vec<ProcessRecord>,
    ) -> JobId {
        let id = self.alloc_job_id();
        let record = JobRecord {
            id,
            pgid,
            summary: summary.into(),
            disposition,
            state: derive_job_state(&processes),
            processes,
        };

        if matches!(disposition, JobDisposition::Foreground) {
            self.foreground_job = Some(id);
        }

        self.jobs.insert(id, record);
        id
    }

    pub fn add_process(&mut self, job_id: JobId, process: ProcessRecord) -> bool {
        let Some(job) = self.jobs.get_mut(&job_id) else {
            return false;
        };

        job.processes.push(process);
        job.state = derive_job_state(&job.processes);
        true
    }

    pub fn update_process_state(&mut self, job_id: JobId, pid: u32, state: ProcessState) -> bool {
        let Some(job) = self.jobs.get_mut(&job_id) else {
            return false;
        };

        let Some(process) = job.processes.iter_mut().find(|process| process.pid == pid) else {
            return false;
        };

        process.state = state;
        job.state = derive_job_state(&job.processes);

        if matches!(job.state, JobState::Completed) && self.foreground_job == Some(job_id) {
            self.foreground_job = None;
        }

        true
    }

    pub fn set_disposition(&mut self, job_id: JobId, disposition: JobDisposition) -> bool {
        let Some(job) = self.jobs.get_mut(&job_id) else {
            return false;
        };

        job.disposition = disposition;
        match disposition {
            JobDisposition::Foreground => self.foreground_job = Some(job_id),
            JobDisposition::Background if self.foreground_job == Some(job_id) => {
                self.foreground_job = None;
            }
            JobDisposition::Background => {}
        }

        true
    }

    pub fn remove(&mut self, job_id: JobId) -> Option<JobRecord> {
        if self.foreground_job == Some(job_id) {
            self.foreground_job = None;
        }
        self.jobs.remove(&job_id)
    }

    pub fn get(&self, job_id: JobId) -> Option<&JobRecord> {
        self.jobs.get(&job_id)
    }

    pub fn foreground_job(&self) -> Option<JobId> {
        self.foreground_job
    }

    pub fn iter(&self) -> impl Iterator<Item = &JobRecord> {
        self.jobs.values()
    }

    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    fn alloc_job_id(&mut self) -> JobId {
        self.next_job_id = self.next_job_id.saturating_add(1).max(1);
        self.next_job_id
    }
}

fn derive_job_state(processes: &[ProcessRecord]) -> JobState {
    if processes.is_empty() {
        return JobState::Completed;
    }

    if processes
        .iter()
        .all(|process| matches!(process.state, ProcessState::Completed(_)))
    {
        JobState::Completed
    } else if processes
        .iter()
        .any(|process| matches!(process.state, ProcessState::Stopped))
    {
        JobState::Stopped
    } else {
        JobState::Running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_jobs_and_tracks_foreground() {
        let mut jobs = Jobs::default();
        let id = jobs.insert(
            1000,
            "sleep 1",
            JobDisposition::Foreground,
            vec![ProcessRecord::new(1000, "sleep 1")],
        );

        assert_eq!(id, 1);
        assert_eq!(jobs.foreground_job(), Some(id));
        let job = jobs.get(id).expect("job should exist");
        assert_eq!(job.pgid(), 1000);
        assert_eq!(job.summary(), "sleep 1");
        assert_eq!(job.state(), JobState::Running);
    }

    #[test]
    fn process_state_transitions_update_job_state() {
        let mut jobs = Jobs::default();
        let id = jobs.insert(
            2000,
            "pipeline",
            JobDisposition::Background,
            vec![
                ProcessRecord::new(2001, "printf a"),
                ProcessRecord::new(2002, "cat"),
            ],
        );

        assert!(jobs.update_process_state(id, 2001, ProcessState::Stopped));
        assert_eq!(
            jobs.get(id).expect("job should exist").state(),
            JobState::Stopped
        );

        assert!(jobs.update_process_state(id, 2001, ProcessState::Completed(0)));
        assert_eq!(
            jobs.get(id).expect("job should exist").state(),
            JobState::Running
        );

        assert!(jobs.update_process_state(id, 2002, ProcessState::Completed(0)));
        assert_eq!(
            jobs.get(id).expect("job should exist").state(),
            JobState::Completed
        );
    }
}
