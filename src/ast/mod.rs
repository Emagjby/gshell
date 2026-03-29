#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleCommand {
    pub argv: Vec<String>,
    pub redirections: Vec<Redirection>,
}

impl SimpleCommand {
    pub fn new(argv: Vec<String>) -> Self {
        Self {
            argv,
            redirections: Vec::new(),
        }
    }

    pub fn with_redirections(argv: Vec<String>, redirections: Vec<Redirection>) -> Self {
        Self { argv, redirections }
    }

    pub fn is_empty(&self) -> bool {
        self.argv.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandNode {
    Simple(SimpleCommand),
    Group(Box<ShellExpr>),
    Subshell(Box<ShellExpr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellExpr {
    Command(CommandNode),
    Pipeline(Vec<CommandNode>),
    And(Box<ShellExpr>, Box<ShellExpr>),
    Or(Box<ShellExpr>, Box<ShellExpr>),
    Sequence(Vec<ShellExpr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirection {
    pub fd: Option<u32>,
    pub kind: RedirectionKind,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectionKind {
    Input,
    OutputTruncate,
    OutputAppend,
}
