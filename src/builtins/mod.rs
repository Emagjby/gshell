use std::{collections::HashMap, pin::Pin, sync::Arc};

use crate::shell::{CommandOutput, SharedShellState, ShellResult};

pub type BuiltinFuture<'a> = Pin<Box<dyn Future<Output = ShellResult<CommandOutput>> + Send + 'a>>;

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
