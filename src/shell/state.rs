use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::{
    process::Child,
    sync::{Mutex, RwLock},
};

use crate::{
    ast::ShellExpr,
    config::{HighlighterConfig, PromptConfig},
    history::HistoryConfig,
    jobs::Jobs,
    shell::{CommandOutput, ExitCode},
};

type OutputSink = Arc<dyn Fn(&CommandOutput) + Send + Sync>;

pub type SharedShellState = Arc<RwLock<ShellState>>;

#[derive(Debug, Clone, Default)]
pub struct ChildHandleStore {
    children: Arc<Mutex<HashMap<u32, Arc<Mutex<Child>>>>>,
}

impl ChildHandleStore {
    pub async fn insert(&self, pid: u32, child: Child) {
        self.children
            .lock()
            .await
            .insert(pid, Arc::new(Mutex::new(child)));
    }

    pub async fn get(&self, pid: u32) -> Option<Arc<Mutex<Child>>> {
        self.children.lock().await.get(&pid).cloned()
    }

    pub async fn remove(&self, pid: u32) -> Option<Arc<Mutex<Child>>> {
        self.children.lock().await.remove(&pid)
    }

    pub async fn pids(&self) -> Vec<u32> {
        self.children.lock().await.keys().copied().collect()
    }
}

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

    pub fn remove(&mut self, name: &str) -> Option<String> {
        self.aliases.remove(name)
    }

    pub fn entries(&self) -> Vec<(&str, &str)> {
        let mut entries = self
            .aliases
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect::<Vec<_>>();
        entries.sort_by_key(|(name, _)| *name);
        entries
    }
}

#[derive(Debug, Clone, Default)]
pub struct FunctionStore {
    functions: HashMap<String, ShellExpr>,
}

impl FunctionStore {
    pub fn get(&self, name: &str) -> Option<&ShellExpr> {
        self.functions.get(name)
    }

    pub fn set(&mut self, name: impl Into<String>, value: ShellExpr) {
        self.functions.insert(name.into(), value);
    }

    pub fn remove(&mut self, name: &str) -> Option<ShellExpr> {
        self.functions.remove(name)
    }

    pub fn names(&self) -> Vec<String> {
        let mut names = self.functions.keys().cloned().collect::<Vec<_>>();
        names.sort();
        names
    }
}

#[derive(Clone)]
pub struct RuntimeServices {
    prompt_config: PromptConfig,
    highlighter_config: HighlighterConfig,
    output_sink: Option<OutputSink>,
}

impl std::fmt::Debug for RuntimeServices {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeServices")
            .field("prompt_config", &self.prompt_config)
            .field("highlighter_config", &self.highlighter_config)
            .field(
                "output_sink",
                &self.output_sink.as_ref().map(|_| "<installed>"),
            )
            .finish()
    }
}

impl Default for RuntimeServices {
    fn default() -> Self {
        Self {
            prompt_config: PromptConfig::from_env(),
            highlighter_config: HighlighterConfig::from_env(),
            output_sink: None,
        }
    }
}

impl RuntimeServices {
    pub fn prompt_config(&self) -> &PromptConfig {
        &self.prompt_config
    }

    pub fn set_prompt_config(&mut self, config: PromptConfig) {
        self.prompt_config = config;
    }

    pub fn highlighter_config(&self) -> &HighlighterConfig {
        &self.highlighter_config
    }

    pub fn set_highlighter_config(&mut self, config: HighlighterConfig) {
        self.highlighter_config = config;
    }

    pub fn output_sink(&self) -> Option<OutputSink> {
        self.output_sink.clone()
    }

    pub fn set_output_sink(&mut self, sink: Option<OutputSink>) {
        self.output_sink = sink;
    }
}

#[derive(Debug, Clone)]
pub struct ShellState {
    env: HashMap<String, String>,
    cwd: PathBuf,
    last_exit_status: ExitCode,
    history: HistoryState,
    aliases: AliasStore,
    functions: FunctionStore,
    jobs: Jobs,
    child_handles: ChildHandleStore,
    active_functions: Vec<String>,
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
            jobs: Jobs::default(),
            child_handles: ChildHandleStore::default(),
            active_functions: Vec::new(),
            runtime_services: RuntimeServices::default(),
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

    pub fn cwd(&self) -> &Path {
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

    pub fn jobs(&self) -> &Jobs {
        &self.jobs
    }

    pub fn jobs_mut(&mut self) -> &mut Jobs {
        &mut self.jobs
    }

    pub fn child_handles(&self) -> &ChildHandleStore {
        &self.child_handles
    }

    pub fn can_enter_function(&self, name: &str) -> bool {
        !self.active_functions.iter().any(|active| active == name)
            && self.active_functions.len() < 64
    }

    pub fn enter_function(&mut self, name: impl Into<String>) {
        self.active_functions.push(name.into());
    }

    pub fn exit_function(&mut self) {
        self.active_functions.pop();
    }

    pub fn runtime_services(&self) -> &RuntimeServices {
        &self.runtime_services
    }

    pub fn runtime_services_mut(&mut self) -> &mut RuntimeServices {
        &mut self.runtime_services
    }
}
