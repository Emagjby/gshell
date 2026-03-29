use crate::shell::{ShellError, ShellResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Word(String),
    Pipe,
    AndIf,
    OrIf,
    Semicolon,
    RedirectIn,
    RedirectOut,
    RedirectAppend,
    LParen,
    RParen,
    IoNumber(u8),
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

            match ch {
                '|' => {
                    chars.next();
                    if chars.peek() == Some(&'|') {
                        chars.next();
                        tokens.push(Token::OrIf);
                    } else {
                        tokens.push(Token::Pipe);
                    }
                }
                '&' => {
                    chars.next();
                    if chars.peek() == Some(&'&') {
                        chars.next();
                        tokens.push(Token::AndIf);
                    } else {
                        return Err(ShellError::message("unsupported operator '&'"));
                    }
                }
                ';' => {
                    chars.next();
                    tokens.push(Token::Semicolon);
                }
                '(' => {
                    chars.next();
                    tokens.push(Token::LParen);
                }
                ')' => {
                    chars.next();
                    tokens.push(Token::RParen);
                }
                '>' => {
                    chars.next();
                    if chars.peek() == Some(&'>') {
                        chars.next();
                        tokens.push(Token::RedirectAppend);
                    } else {
                        tokens.push(Token::RedirectOut);
                    }
                }
                '<' => {
                    chars.next();
                    tokens.push(Token::RedirectIn);
                }
                c if c.is_ascii_digit() => {
                    let mut digits = String::new();

                    while let Some(next) = chars.peek().copied() {
                        if next.is_ascii_digit() {
                            digits.push(next);
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    match chars.peek().copied() {
                        Some('>') | Some('<') => {
                            let fd = digits.parse::<u8>().map_err(|_| {
                                ShellError::message("invalid file descriptor number")
                            })?;
                            tokens.push(Token::IoNumber(fd));
                        }
                        _ => {
                            tokens.push(Token::Word(digits));
                        }
                    }
                }
                _ => {
                    let word = self.read_word(&mut chars)?;
                    if !word.is_empty() {
                        tokens.push(Token::Word(word));
                    }
                }
            }
        }

        Ok(tokens)
    }

    fn read_word<I>(&self, chars: &mut std::iter::Peekable<I>) -> ShellResult<String>
    where
        I: Iterator<Item = char>,
    {
        let mut word = String::new();

        while let Some(ch) = chars.peek().copied() {
            match ch {
                c if c.is_whitespace() => break,
                '|' | '&' | ';' | '>' | '<' | '(' | ')' => break,
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

        Ok(word)
    }
}
