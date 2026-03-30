use std::pin::Pin;
use std::sync::Arc;

use crate::{
    ast::{BoolOp, CommandNode, RedirectionKind, ShellExpr},
    shell::{ExitCode, SharedShellState, ShellResult, ShellState},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    pub segments: Vec<WordSegment>,
}

impl Word {
    pub fn new(segments: Vec<WordSegment>) -> Self {
        Self { segments }
    }

    pub fn literal(text: impl Into<String>) -> Self {
        Self {
            segments: vec![WordSegment::Literal {
                text: text.into(),
                quote: QuoteKind::Unquoted,
            }],
        }
    }

    pub fn expand(&self, state: &ShellState) -> String {
        expand_word_sync(state, self)
    }

    pub fn split_assignment(&self) -> Option<(String, Word)> {
        let WordSegment::Literal {
            text,
            quote: QuoteKind::Unquoted,
        } = self.segments.first()?
        else {
            return None;
        };

        let equals = text.find('=')?;
        let name = &text[..equals];
        if !is_valid_assignment_name(name) {
            return None;
        }

        let mut value_segments = Vec::with_capacity(self.segments.len());
        let suffix = &text[equals + 1..];
        if !suffix.is_empty() {
            value_segments.push(WordSegment::Literal {
                text: suffix.to_string(),
                quote: QuoteKind::Unquoted,
            });
        }
        value_segments.extend(self.segments.iter().skip(1).cloned());

        Some((name.to_string(), Word::new(value_segments)))
    }

    pub fn quote_removed_text(&self) -> String {
        let mut out = String::new();

        for segment in &self.segments {
            match segment {
                WordSegment::Literal { text, .. } => out.push_str(text),
                WordSegment::Variable { name, .. } => {
                    out.push('$');
                    out.push_str(name);
                }
                WordSegment::LastStatus { .. } => out.push_str("$?"),
                WordSegment::CommandSubstitution { expr, .. } => {
                    out.push_str("$(");
                    out.push_str(&render_shell_expr(expr));
                    out.push(')');
                }
            }
        }

        out
    }

    pub fn is_quoted(&self) -> bool {
        self.segments
            .iter()
            .any(|segment| !matches!(segment.quote_kind(), QuoteKind::Unquoted))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordSegment {
    Literal {
        text: String,
        quote: QuoteKind,
    },
    Variable {
        name: String,
        quote: QuoteKind,
    },
    LastStatus {
        quote: QuoteKind,
    },
    CommandSubstitution {
        expr: Box<ShellExpr>,
        quote: QuoteKind,
    },
}

impl WordSegment {
    pub fn quote_kind(&self) -> QuoteKind {
        match self {
            WordSegment::Literal { quote, .. }
            | WordSegment::Variable { quote, .. }
            | WordSegment::LastStatus { quote }
            | WordSegment::CommandSubstitution { quote, .. } => *quote,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteKind {
    Unquoted,
    SingleQuoted,
    DoubleQuoted,
}

pub fn expand_words(state: &ShellState, words: &[Word]) -> Vec<String> {
    words
        .iter()
        .map(|word| expand_word_sync(state, word))
        .collect()
}

pub fn exit_code_to_string(code: ExitCode) -> String {
    code.as_u8().to_string()
}

fn expand_word_sync(state: &ShellState, word: &Word) -> String {
    let mut out = String::new();

    for segment in &word.segments {
        match segment {
            WordSegment::Literal { text, .. } => out.push_str(text),
            WordSegment::Variable { name, quote } => {
                if matches!(quote, QuoteKind::SingleQuoted) {
                    out.push('$');
                    out.push_str(name);
                } else if let Some(value) = state.env_var(name) {
                    out.push_str(value);
                }
            }
            WordSegment::LastStatus { quote } => {
                if matches!(quote, QuoteKind::SingleQuoted) {
                    out.push_str("$?");
                } else {
                    out.push_str(&u8::from(state.last_exit_status()).to_string());
                }
            }
            WordSegment::CommandSubstitution { .. } => {}
        }
    }

    out
}

pub type CommandSubstitutionFuture =
    Pin<Box<dyn Future<Output = ShellResult<String>> + Send + 'static>>;

pub type CommandSubstitutionExecutor =
    Arc<dyn Fn(SharedShellState, ShellExpr) -> CommandSubstitutionFuture + Send + Sync>;

pub async fn expand_word_with_state(
    state: SharedShellState,
    word: &Word,
    substitution_executor: &CommandSubstitutionExecutor,
) -> ShellResult<String> {
    let mut out = String::new();

    for segment in &word.segments {
        match segment {
            WordSegment::Literal { text, .. } => out.push_str(text),
            WordSegment::Variable { name, quote } => {
                if matches!(quote, QuoteKind::SingleQuoted) {
                    out.push('$');
                    out.push_str(name);
                } else {
                    let val = {
                        let guard = state.read().await;
                        guard.env_var(name).map(ToOwned::to_owned)
                    };

                    if let Some(val) = val {
                        out.push_str(&val);
                    }
                }
            }
            WordSegment::LastStatus { quote } => {
                if matches!(quote, QuoteKind::SingleQuoted) {
                    out.push_str("$?");
                } else {
                    let code = {
                        let guard = state.read().await;
                        guard.last_exit_status()
                    };

                    out.push_str(&exit_code_to_string(code));
                }
            }
            WordSegment::CommandSubstitution { expr, .. } => {
                let substituted = substitution_executor(state.clone(), (**expr).clone()).await?;
                out.push_str(&normalize_command_substitution_output(substituted));
            }
        }
    }

    Ok(out)
}

pub async fn expand_words_with_state(
    state: SharedShellState,
    words: &[Word],
    substitution_executor: &CommandSubstitutionExecutor,
) -> ShellResult<Vec<String>> {
    let mut out = Vec::with_capacity(words.len());

    for word in words {
        out.push(expand_word_with_state(state.clone(), word, substitution_executor).await?);
    }

    Ok(out)
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

fn is_valid_assignment_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }

    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn render_shell_expr(expr: &ShellExpr) -> String {
    match expr {
        ShellExpr::Command(node) => render_command_node(node),
        ShellExpr::Pipeline(nodes) => nodes
            .iter()
            .map(render_command_node)
            .collect::<Vec<_>>()
            .join(" | "),
        ShellExpr::BooleanChain { first, rest } => {
            let mut out = render_shell_expr(first);
            for (op, expr) in rest {
                let op = match op {
                    BoolOp::And => "&&",
                    BoolOp::Or => "||",
                };
                out.push(' ');
                out.push_str(op);
                out.push(' ');
                out.push_str(&render_shell_expr(expr));
            }
            out
        }
        ShellExpr::Sequence(exprs) => exprs
            .iter()
            .map(render_shell_expr)
            .collect::<Vec<_>>()
            .join(" ; "),
    }
}

fn render_command_node(node: &CommandNode) -> String {
    match node {
        CommandNode::Simple(simple) => {
            let mut parts = simple
                .argv
                .iter()
                .map(Word::quote_removed_text)
                .collect::<Vec<_>>();

            parts.extend(simple.redirections.iter().map(|redirection| {
                let fd = redirection.fd.map(|fd| fd.to_string()).unwrap_or_default();
                let op = match &redirection.kind {
                    RedirectionKind::Input => "<",
                    RedirectionKind::OutputTruncate => ">",
                    RedirectionKind::OutputAppend => ">>",
                    RedirectionKind::HereDoc { .. } => "<<",
                };

                format!("{fd}{op}{}", redirection.target.quote_removed_text())
            }));

            parts.join(" ")
        }
        CommandNode::Subshell(expr) => format!("({})", render_shell_expr(expr)),
    }
}
