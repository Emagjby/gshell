use crate::shell::{ExitCode, SharedShellState, ShellResult, ShellState};

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
        let mut out = String::new();

        for segment in &self.segments {
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
                WordSegment::CommandSubstitution { source, quote } => {
                    if matches!(quote, QuoteKind::SingleQuoted) {
                        out.push_str(&format!("$({source})"));
                    } else {
                        // To be added in P3-02
                    }
                }
            }
        }

        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordSegment {
    Literal { text: String, quote: QuoteKind },
    Variable { name: String, quote: QuoteKind },
    LastStatus { quote: QuoteKind },
    CommandSubstitution { source: String, quote: QuoteKind },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteKind {
    Unquoted,
    SingleQuoted,
    DoubleQuoted,
}

pub fn expand_words(state: &ShellState, words: &[Word]) -> Vec<String> {
    words.iter().map(|word| word.expand(state)).collect()
}

pub async fn expand_words_with_state(
    state: SharedShellState,
    words: &[Word],
) -> ShellResult<Vec<String>> {
    let guard = state.read().await;
    Ok(expand_words(&guard, words))
}

pub fn exit_code_to_string(code: ExitCode) -> String {
    code.as_u8().to_string()
}
