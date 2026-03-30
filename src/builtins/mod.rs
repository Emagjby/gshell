mod cd;
mod clear;
mod echo;
mod exit;
mod history;
mod pwd;
mod r#type;

pub use cd::CdBuiltin;
pub use clear::ClearBuiltin;
pub use echo::EchoBuiltin;
pub use exit::ExitBuiltin;
pub use history::HistoryBuiltin;
pub use pwd::PwdBuiltin;
pub use r#type::TypeBuiltin;

use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, OnceLock},
};

use crate::shell::{SharedShellState, ShellAction, ShellResult};

pub type BuiltinFuture<'a> = Pin<Box<dyn Future<Output = ShellResult<ShellAction>> + Send + 'a>>;

pub trait Builtin: Send + Sync {
    fn name(&self) -> &'static str;
    fn execute<'a>(&'a self, state: SharedShellState, args: &'a [String]) -> BuiltinFuture<'a>;
}

#[derive(Default)]
pub struct BuiltinRegistry {
    builtins: HashMap<String, Arc<dyn Builtin>>,
}

impl BuiltinRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn defaults() -> &'static Self {
        static DEFAULTS: OnceLock<BuiltinRegistry> = OnceLock::new();
        DEFAULTS.get_or_init(Self::with_defaults)
    }

    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(HistoryBuiltin));
        registry.register(Arc::new(CdBuiltin));
        registry.register(Arc::new(PwdBuiltin));
        registry.register(Arc::new(EchoBuiltin));
        registry.register(Arc::new(ClearBuiltin));
        registry.register(Arc::new(TypeBuiltin));
        registry.register(Arc::new(ExitBuiltin));
        registry
    }

    pub fn register(&mut self, builtin: Arc<dyn Builtin>) -> Option<Arc<dyn Builtin>> {
        self.builtins.insert(builtin.name().to_string(), builtin)
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Builtin>> {
        self.builtins.get(name).cloned()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.builtins.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.builtins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.builtins.is_empty()
    }
}
