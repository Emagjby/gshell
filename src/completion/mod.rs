use std::{
    collections::BTreeSet,
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use nu_ansi_term::Style;
use reedline::{Completer, Hinter, SearchQuery, Span, Suggestion};

use crate::{
    builtins::BuiltinRegistry,
    shell::{SharedShellState, ShellState},
};

#[derive(Clone)]
pub struct ShellCompleter {
    state: SharedShellState,
}

impl ShellCompleter {
    pub fn new(state: SharedShellState) -> Self {
        Self { state }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionKind {
    Command,
    Path,
    EnvVar,
}

#[derive(Debug, Clone)]
struct CompletionContext {
    kind: CompletionKind,
    token: String,
    span: Span,
}

impl Completer for ShellCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let context = completion_context(line, pos);

        let values = match context.kind {
            CompletionKind::Command => self.complete_commands(&context.token),
            CompletionKind::Path => self.complete_paths(&context.token),
            CompletionKind::EnvVar => self.complete_env_vars(&context.token),
        };

        values
            .into_iter()
            .map(|value| Suggestion {
                value,
                display_override: None,
                description: None,
                style: None,
                extra: None,
                span: context.span,
                append_whitespace: context.kind != CompletionKind::Path,
                match_indices: None,
            })
            .collect()
    }
}

impl ShellCompleter {
    fn complete_commands(&self, prefix: &str) -> Vec<String> {
        let mut out = BTreeSet::new();

        for builtin in BuiltinRegistry::defaults().names() {
            if builtin.starts_with(prefix) {
                out.insert(builtin);
            }
        }

        let path_var = self.current_path_var();

        for dir in env::split_paths(&OsString::from(path_var)) {
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };

            for entry in entries.flatten() {
                let path = entry.path();

                if !is_executable_file(&path) {
                    continue;
                }

                let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };

                if name.starts_with(prefix) {
                    out.insert(name.to_string());
                }
            }
        }

        out.into_iter().collect()
    }

    fn complete_paths(&self, prefix: &str) -> Vec<String> {
        let cwd = self.read_state(|state| state.cwd().to_path_buf());

        let expanded = expand_tilde(prefix);
        let typed_path = PathBuf::from(&expanded);

        let (dir, needle, replace_base) = if prefix.is_empty() {
            (cwd.clone(), String::new(), String::new())
        } else if prefix.ends_with('/') {
            let dir = absolutize_path(&cwd, &typed_path);
            (dir, String::new(), prefix.to_string())
        } else {
            let parent = typed_path.parent().unwrap_or_else(|| Path::new(""));
            let dir = if parent.as_os_str().is_empty() {
                cwd.clone()
            } else {
                absolutize_path(&cwd, &PathBuf::from(parent))
            };

            let needle = typed_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("")
                .to_string();

            let replace_base = prefix
                .rsplit_once('/')
                .map(|(base, _)| format!("{base}/"))
                .unwrap_or_default();

            (dir, needle, replace_base)
        };

        let Ok(entries) = fs::read_dir(&dir) else {
            return Vec::new();
        };

        let mut out = BTreeSet::new();

        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            if !name.starts_with(&needle) {
                continue;
            }

            let mut value = format!("{replace_base}{name}");
            if path.is_dir() {
                value.push('/');
            }

            out.insert(value);
        }

        out.into_iter().collect()
    }

    fn complete_env_vars(&self, prefix: &str) -> Vec<String> {
        let needle = prefix.strip_prefix('$').unwrap_or(prefix);

        let mut out = BTreeSet::new();

        let env_keys = self.read_state(|state| state.env().keys().cloned().collect::<Vec<_>>());

        for key in env_keys {
            if key.starts_with(needle) {
                out.insert(format!("${key}"));
            }
        }

        out.into_iter().collect()
    }

    fn current_path_var(&self) -> String {
        self.read_state(|state| {
            state
                .env_var("PATH")
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| env::var("PATH").unwrap_or_default())
        })
    }

    fn read_state<T, F>(&self, selector: F) -> T
    where
        T: Send + 'static,
        F: FnOnce(&ShellState) -> T + Send + 'static,
    {
        if let Ok(guard) = self.state.try_read() {
            return selector(&guard);
        }

        let state = self.state.clone();
        std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("temporary runtime should initialize")
                .block_on(async move {
                    let guard = state.read().await;
                    selector(&guard)
                })
        })
        .join()
        .expect("state reader thread should not panic")
    }
}

#[derive(Debug, Clone)]
pub struct ShellHinter {
    style: Style,
    current_hint: String,
}

impl Default for ShellHinter {
    fn default() -> Self {
        Self {
            style: Style::new(),
            current_hint: String::new(),
        }
    }
}

impl ShellHinter {
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Hinter for ShellHinter {
    fn handle(
        &mut self,
        line: &str,
        pos: usize,
        history: &dyn reedline::History,
        use_ansi: bool,
        _cwd: &str,
    ) -> String {
        if pos != line.len() || line.trim().is_empty() {
            self.current_hint.clear();
            return String::new();
        }

        let search = line.to_string();
        let hint = history
            .search(SearchQuery::last_with_prefix(
                search.clone(),
                history.session(),
            ))
            .ok()
            .and_then(|entries| entries.into_iter().next())
            .and_then(|entry| {
                if entry.command_line == search {
                    None
                } else {
                    entry
                        .command_line
                        .get(search.len()..)
                        .map(ToOwned::to_owned)
                }
            })
            .unwrap_or_default();

        self.current_hint = hint.clone();
        if use_ansi && !hint.is_empty() {
            self.style.paint(hint).to_string()
        } else {
            hint
        }
    }

    fn complete_hint(&self) -> String {
        self.current_hint.clone()
    }

    fn next_hint_token(&self) -> String {
        self.current_hint
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string()
    }
}

fn completion_context(line: &str, pos: usize) -> CompletionContext {
    let safe_pos = pos.min(line.len());
    let before = &line[..safe_pos];

    let token_start = before
        .char_indices()
        .rev()
        .find(|(_, ch)| is_token_break(*ch))
        .map(|(idx, ch)| idx + ch.len_utf8())
        .unwrap_or(0);

    let token = line[token_start..safe_pos].to_string();
    let span = Span::new(token_start, safe_pos);

    let before_token = before[..token_start].trim_end();

    let kind = if token.starts_with('$') {
        CompletionKind::EnvVar
    } else if needs_path_completion(before_token, &token) {
        CompletionKind::Path
    } else if is_command_position(before_token) {
        CompletionKind::Command
    } else {
        CompletionKind::Path
    };

    CompletionContext { kind, token, span }
}

fn is_token_break(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '|' | ';' | '(' | ')' | '<' | '>')
}

fn is_command_position(before_token: &str) -> bool {
    before_token.is_empty()
        || before_token.ends_with('|')
        || before_token.ends_with("&&")
        || before_token.ends_with("||")
        || before_token.ends_with(';')
        || before_token.ends_with('(')
}

fn needs_path_completion(before_token: &str, token: &str) -> bool {
    token.contains('/')
        || token.starts_with('.')
        || token.starts_with('~')
        || before_token.ends_with('<')
        || before_token.ends_with('>')
        || before_token.ends_with(">>")
        || before_token.ends_with("2>")
        || before_token.ends_with("2>>")
}

fn expand_tilde(input: &str) -> String {
    if (input == "~" || input.starts_with("~/"))
        && let Ok(home) = env::var("HOME")
    {
        return format!("{home}{}", &input[1..]);
    }

    input.to_string()
}

fn absolutize_path(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };

    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        false
    }
}
