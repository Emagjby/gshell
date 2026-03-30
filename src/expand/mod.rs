use std::{
    fs,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};

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

pub async fn expand_word_pathnames_with_state(
    state: SharedShellState,
    word: &Word,
    substitution_executor: &CommandSubstitutionExecutor,
) -> ShellResult<Vec<String>> {
    let expanded =
        expand_word_pattern_with_state(state.clone(), word, substitution_executor).await?;

    if !expanded.has_glob_meta {
        return Ok(vec![expanded.text]);
    }

    let cwd = state.read().await.cwd().to_path_buf();
    let matches = expand_glob_pattern(&cwd, &expanded.text, &expanded.components)?;

    if matches.is_empty() {
        Ok(vec![expanded.text])
    } else {
        Ok(matches)
    }
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

pub async fn expand_words_pathnames_with_state(
    state: SharedShellState,
    words: &[Word],
    substitution_executor: &CommandSubstitutionExecutor,
) -> ShellResult<Vec<String>> {
    let mut out = Vec::new();

    for word in words {
        out.extend(
            expand_word_pathnames_with_state(state.clone(), word, substitution_executor).await?,
        );
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

#[derive(Debug)]
struct ExpandedGlobWord {
    text: String,
    components: Vec<GlobComponent>,
    has_glob_meta: bool,
}

#[derive(Debug, Clone)]
struct GlobComponent {
    tokens: Vec<GlobToken>,
}

#[derive(Debug, Clone)]
enum GlobToken {
    Literal(char),
    AnyString,
    AnyChar,
    CharClass(CharClass),
}

#[derive(Debug, Clone)]
struct CharClass {
    negated: bool,
    items: Vec<CharClassItem>,
}

#[derive(Debug, Clone)]
enum CharClassItem {
    Char(char),
    Range(char, char),
}

async fn expand_word_pattern_with_state(
    state: SharedShellState,
    word: &Word,
    substitution_executor: &CommandSubstitutionExecutor,
) -> ShellResult<ExpandedGlobWord> {
    let mut text = String::new();
    let mut tokens = Vec::new();
    let mut has_glob_meta = false;

    for segment in &word.segments {
        let quote = segment.quote_kind();
        let segment_text = match segment {
            WordSegment::Literal { text, .. } => text.clone(),
            WordSegment::Variable { name, .. } => {
                let guard = state.read().await;
                guard
                    .env_var(name)
                    .map(ToOwned::to_owned)
                    .unwrap_or_default()
            }
            WordSegment::LastStatus { .. } => {
                let guard = state.read().await;
                exit_code_to_string(guard.last_exit_status())
            }
            WordSegment::CommandSubstitution { expr, .. } => {
                let substituted = substitution_executor(state.clone(), (**expr).clone()).await?;
                normalize_command_substitution_output(substituted)
            }
        };

        text.push_str(&segment_text);

        if matches!(quote, QuoteKind::Unquoted) {
            let parsed = parse_glob_tokens(&segment_text);
            has_glob_meta |= parsed.1;
            tokens.extend(parsed.0);
        } else {
            tokens.extend(segment_text.chars().map(GlobToken::Literal));
        }
    }

    Ok(ExpandedGlobWord {
        text,
        components: split_glob_components(tokens),
        has_glob_meta,
    })
}

fn parse_glob_tokens(text: &str) -> (Vec<GlobToken>, bool) {
    let chars = text.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut idx = 0usize;
    let mut has_glob_meta = false;

    while idx < chars.len() {
        match chars[idx] {
            '*' => {
                tokens.push(GlobToken::AnyString);
                has_glob_meta = true;
                idx += 1;
            }
            '?' => {
                tokens.push(GlobToken::AnyChar);
                has_glob_meta = true;
                idx += 1;
            }
            '[' => {
                if let Some((class, consumed)) = parse_char_class(&chars[idx..]) {
                    tokens.push(GlobToken::CharClass(class));
                    has_glob_meta = true;
                    idx += consumed;
                } else {
                    tokens.push(GlobToken::Literal('['));
                    idx += 1;
                }
            }
            ch => {
                tokens.push(GlobToken::Literal(ch));
                idx += 1;
            }
        }
    }

    (tokens, has_glob_meta)
}

fn parse_char_class(chars: &[char]) -> Option<(CharClass, usize)> {
    if chars.len() < 2 || chars[0] != '[' {
        return None;
    }

    let mut idx = 1usize;
    let mut negated = false;
    if matches!(chars.get(idx), Some('!' | '^')) {
        negated = true;
        idx += 1;
    }

    let mut items = Vec::new();
    let mut closed = false;

    while idx < chars.len() {
        if chars[idx] == ']' && !items.is_empty() {
            idx += 1;
            closed = true;
            break;
        }

        let start = chars[idx];
        if idx + 2 < chars.len() && chars[idx + 1] == '-' && chars[idx + 2] != ']' {
            items.push(CharClassItem::Range(start, chars[idx + 2]));
            idx += 3;
        } else {
            items.push(CharClassItem::Char(start));
            idx += 1;
        }
    }

    closed.then_some((CharClass { negated, items }, idx))
}

fn split_glob_components(tokens: Vec<GlobToken>) -> Vec<GlobComponent> {
    let mut components = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if matches!(token, GlobToken::Literal('/')) {
            components.push(GlobComponent { tokens: current });
            current = Vec::new();
        } else {
            current.push(token);
        }
    }

    components.push(GlobComponent { tokens: current });
    components
}

fn expand_glob_pattern(
    cwd: &Path,
    text: &str,
    components: &[GlobComponent],
) -> ShellResult<Vec<String>> {
    let absolute = text.starts_with('/');
    let mut matches = Vec::new();
    let start = if absolute {
        PathBuf::from("/")
    } else {
        cwd.to_path_buf()
    };
    let prefix = if absolute {
        String::from("/")
    } else {
        String::new()
    };

    expand_glob_component(&start, prefix, components, 0, &mut matches)?;
    matches.sort();
    Ok(matches)
}

fn expand_glob_component(
    current_dir: &Path,
    display_prefix: String,
    components: &[GlobComponent],
    idx: usize,
    matches: &mut Vec<String>,
) -> ShellResult<()> {
    if idx >= components.len() {
        matches.push(if display_prefix.is_empty() {
            ".".to_string()
        } else {
            display_prefix
        });
        return Ok(());
    }

    let component = &components[idx];
    if component.tokens.is_empty() {
        let next_dir = if current_dir == Path::new("/") {
            PathBuf::from("/")
        } else {
            current_dir.to_path_buf()
        };
        return expand_glob_component(&next_dir, display_prefix, components, idx + 1, matches);
    }

    if !component_has_glob(component) {
        let name = component_literal_text(component);
        let next_dir = current_dir.join(&name);
        let next_display = join_display_path(&display_prefix, &name);
        return expand_glob_component(&next_dir, next_display, components, idx + 1, matches);
    }

    let mut entries = fs::read_dir(current_dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !component_matches_hidden_rule(component, &name) {
            continue;
        }
        if !glob_component_matches(component, &name) {
            continue;
        }

        let path = entry.path();
        if idx + 1 < components.len() && !path.is_dir() {
            continue;
        }

        let next_display = join_display_path(&display_prefix, &name);
        expand_glob_component(&path, next_display, components, idx + 1, matches)?;
    }

    Ok(())
}

fn component_has_glob(component: &GlobComponent) -> bool {
    component.tokens.iter().any(|token| {
        matches!(
            token,
            GlobToken::AnyString | GlobToken::AnyChar | GlobToken::CharClass(_)
        )
    })
}

fn component_literal_text(component: &GlobComponent) -> String {
    component
        .tokens
        .iter()
        .filter_map(|token| match token {
            GlobToken::Literal(ch) => Some(*ch),
            _ => None,
        })
        .collect()
}

fn join_display_path(prefix: &str, name: &str) -> String {
    if prefix.is_empty() || prefix == "/" {
        format!("{prefix}{name}")
    } else {
        format!("{prefix}/{name}")
    }
}

fn component_matches_hidden_rule(component: &GlobComponent, name: &str) -> bool {
    if !name.starts_with('.') {
        return true;
    }

    matches!(component.tokens.first(), Some(GlobToken::Literal('.')))
}

fn glob_component_matches(component: &GlobComponent, text: &str) -> bool {
    let chars = text.chars().collect::<Vec<_>>();
    glob_tokens_match(&component.tokens, &chars, 0, 0)
}

fn glob_tokens_match(
    tokens: &[GlobToken],
    text: &[char],
    token_idx: usize,
    text_idx: usize,
) -> bool {
    if token_idx == tokens.len() {
        return text_idx == text.len();
    }

    match &tokens[token_idx] {
        GlobToken::Literal(ch) => {
            text.get(text_idx) == Some(ch)
                && glob_tokens_match(tokens, text, token_idx + 1, text_idx + 1)
        }
        GlobToken::AnyChar => {
            text.get(text_idx).is_some()
                && glob_tokens_match(tokens, text, token_idx + 1, text_idx + 1)
        }
        GlobToken::AnyString => (text_idx..=text.len())
            .any(|next_idx| glob_tokens_match(tokens, text, token_idx + 1, next_idx)),
        GlobToken::CharClass(class) => {
            text.get(text_idx)
                .is_some_and(|ch| char_class_matches(class, *ch))
                && glob_tokens_match(tokens, text, token_idx + 1, text_idx + 1)
        }
    }
}

fn char_class_matches(class: &CharClass, ch: char) -> bool {
    let matched = class.items.iter().any(|item| match item {
        CharClassItem::Char(item) => *item == ch,
        CharClassItem::Range(start, end) => *start <= ch && ch <= *end,
    });

    if class.negated { !matched } else { matched }
}
