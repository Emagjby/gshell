use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::shell::{ShellError, ShellResult};

#[derive(Debug, Clone)]
pub struct HistoryConfig {
    path: PathBuf,
}

impl HistoryConfig {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn resolve_default() -> ShellResult<Self> {
        let path = if let Ok(xdg_data_home) = env::var("XDG_DATA_HOME") {
            PathBuf::from(xdg_data_home)
                .join("gshell")
                .join("history.txt")
        } else {
            let home = env::var("HOME").map(PathBuf::from).map_err(|_| {
                ShellError::message("HOME is not set and XDG_DATA_HOME is unavailable")
            })?;

            home.join(".local")
                .join("share")
                .join("gshell")
                .join("history.txt")
        };

        Ok(Self::new(path))
    }

    pub fn ensure_parent_dir(&self) -> ShellResult<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        Ok(())
    }
}

pub fn should_record_history_entry(input: &str) -> bool {
    !input.trim().is_empty()
}
