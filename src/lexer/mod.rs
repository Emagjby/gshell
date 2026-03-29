use crate::shell::{ShellError, ShellResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Word(String),
}

#[derive(Debug, Default)]
pub struct Lexer;

impl Lexer {
    pub fn tokenize(&self, input: &str) -> ShellResult<Vec<Token>> {
        let mut chars = input.chars().peekable();
        let mut tokens = Vec::new();

        while let Some(ch) = chars.peek().copied() {
            if ch.is_whitespace() {
                chars.next();
                continue;
            }

            let mut word = String::new();

            while let Some(ch) = chars.peek().copied() {
                match ch {
                    c if c.is_whitespace() => break,
                    '\'' => {
                        chars.next();
                        loop {
                            match chars.next() {
                                Some('\'') => break,
                                Some(c) => word.push(c),
                                None => {
                                    return Err(ShellError::message(
                                        "unterminated single-quoted string",
                                    ));
                                }
                            }
                        }
                    }
                    '"' => {
                        chars.next();
                        loop {
                            match chars.next() {
                                Some('"') => break,
                                Some('\\') => match chars.next() {
                                    Some('"') => word.push('"'),
                                    Some('\\') => word.push('\\'),
                                    Some('n') => word.push('n'),
                                    Some(other) => {
                                        word.push('\\');
                                        word.push(other);
                                    }
                                    None => {
                                        return Err(ShellError::message(
                                            "unterminated escape in double-quoted string",
                                        ));
                                    }
                                },
                                Some(c) => word.push(c),
                                None => {
                                    return Err(ShellError::message(
                                        "unterminated double-quoted string",
                                    ));
                                }
                            }
                        }
                    }
                    '\\' => {
                        chars.next();
                        match chars.next() {
                            Some(c) => word.push(c),
                            None => {
                                return Err(ShellError::message("unterminated escape sequence"));
                            }
                        }
                    }
                    other => {
                        chars.next();
                        word.push(other);
                    }
                }
            }

            if !word.is_empty() {
                tokens.push(Token::Word(word));
            }
        }

        Ok(tokens)
    }
}
