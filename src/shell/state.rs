use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::sync::RwLock;

use crate::{history::HistoryConfig, shell::ExitCode};

pub type SharedShellState = Arc<RwLock<ShellState>>;

#[derive(Debug, Clone, Default)]
pub struct HistoryState {
    path: PathBuf,
    entries: Vec<String>,
}

impl HistoryState {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            entries: Vec::new(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    pub fn set_entries(&mut self, entries: Vec<String>) {
        self.entries = entries;
    }

    pub fn push(&mut self, entry: impl Into<String>) {
        self.entries.push(entry.into());
    }
}

#[derive(Debug, Clone, Default)]
pub struct AliasStore {
    aliases: HashMap<String, String>,
}

impl AliasStore {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.aliases.get(name).map(String::as_str)
    }

    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.aliases.insert(name.into(), value.into());
    }
}

#[derive(Debug, Clone, Default)]
pub struct FunctionStore {
    functions: HashMap<String, String>,
}

impl FunctionStore {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.functions.get(name).map(String::as_str)
    }

    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.functions.insert(name.into(), value.into());
    }
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeServices;

#[derive(Debug, Clone)]
pub struct ShellState {
    env: HashMap<String, String>,
    cwd: PathBuf,
    last_exit_status: ExitCode,
    history: HistoryState,
    aliases: AliasStore,
    functions: FunctionStore,
    runtime_services: RuntimeServices,
}

impl ShellState {
    pub fn new() -> std::io::Result<Self> {
        let history_path = HistoryConfig::resolve_default()
            .map_err(|err| std::io::Error::other(err.to_string()))?
            .path()
            .to_path_buf();

        Ok(Self {
            env: std::env::vars().collect(),
            cwd: std::env::current_dir()?,
            last_exit_status: ExitCode::SUCCESS,
            history: HistoryState::new(history_path),
            aliases: AliasStore::default(),
            functions: FunctionStore::default(),
            runtime_services: RuntimeServices,
        })
    }

    pub async fn shared() -> std::io::Result<SharedShellState> {
        Ok(Arc::new(RwLock::new(Self::new()?)))
    }

    pub fn env(&self) -> &HashMap<String, String> {
        &self.env
    }

    pub fn env_var(&self, key: &str) -> Option<&str> {
        self.env.get(key).map(String::as_str)
    }

    pub fn set_env_var(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.env.insert(key.into(), value.into());
    }

    pub fn remove_env_var(&mut self, key: &str) -> Option<String> {
        self.env.remove(key)
    }

    pub fn cwd(&self) -> &PathBuf {
        &self.cwd
    }

    pub fn set_cwd(&mut self, cwd: PathBuf) {
        self.cwd = cwd;
    }

    pub fn last_exit_status(&self) -> ExitCode {
        self.last_exit_status
    }

    pub fn set_last_exit_status(&mut self, exit_code: ExitCode) {
        self.last_exit_status = exit_code;
    }

    pub fn history(&self) -> &HistoryState {
        &self.history
    }

    pub fn history_mut(&mut self) -> &mut HistoryState {
        &mut self.history
    }

    pub fn aliases(&self) -> &AliasStore {
        &self.aliases
    }

    pub fn aliases_mut(&mut self) -> &mut AliasStore {
        &mut self.aliases
    }

    pub fn functions(&self) -> &FunctionStore {
        &self.functions
    }

    pub fn functions_mut(&mut self) -> &mut FunctionStore {
        &mut self.functions
    }

    pub fn runtime_services(&self) -> &RuntimeServices {
        &self.runtime_services
    }
}
