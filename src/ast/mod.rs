use crate::expand::Word;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleCommand {
    pub assignments: Vec<(String, Word)>,
    pub argv: Vec<Word>,
    pub redirections: Vec<Redirection>,
}

impl SimpleCommand {
    pub fn new(argv: Vec<Word>) -> Self {
        Self {
            assignments: Vec::new(),
            argv,
            redirections: Vec::new(),
        }
    }

    pub fn with_redirections(argv: Vec<Word>, redirections: Vec<Redirection>) -> Self {
        Self {
            assignments: Vec::new(),
            argv,
            redirections,
        }
    }

    pub fn with_assignments(
        assignments: Vec<(String, Word)>,
        argv: Vec<Word>,
        redirections: Vec<Redirection>,
    ) -> Self {
        Self {
            assignments,
            argv,
            redirections,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.assignments.is_empty() && self.argv.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandNode {
    Simple(SimpleCommand),
    FunctionDef {
        name: String,
        body: Box<ShellExpr>,
    },
    /// Placeholder for parenthesized command syntax until real subshell state isolation lands.
    Subshell(Box<ShellExpr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellExpr {
    Command(CommandNode),
    Pipeline(Vec<CommandNode>),
    BooleanChain {
        first: Box<ShellExpr>,
        rest: Vec<(BoolOp, ShellExpr)>,
    },
    Sequence(Vec<ShellExpr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoolOp {
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirection {
    pub fd: Option<u32>,
    pub kind: RedirectionKind,
    pub target: Word,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedirectionKind {
    Input,
    OutputTruncate,
    OutputAppend,
    HereDoc { body: String, expand: bool },
}

impl Redirection {
    pub fn effective_fd(&self) -> u8 {
        match (&self.fd, &self.kind) {
            (Some(fd), _) => *fd as u8,
            (None, RedirectionKind::Input) => 0,
            (
                None,
                RedirectionKind::OutputTruncate
                | RedirectionKind::OutputAppend
                | RedirectionKind::HereDoc { .. },
            ) => 1,
        }
    }
}
