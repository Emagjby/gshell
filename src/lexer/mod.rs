use crate::{
    expand::{QuoteKind, Word, WordSegment},
    shell::{ShellError, ShellResult},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Word(Word),
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
                            tokens.push(Token::Word(Word::literal(digits)));
                        }
                    }
                }
                _ => {
                    let word = self.read_word(&mut chars)?;
                    if !word.segments.is_empty() {
                        tokens.push(Token::Word(word));
                    }
                }
            }
        }

        Ok(tokens)
    }

    fn read_word<I>(&self, chars: &mut std::iter::Peekable<I>) -> ShellResult<Word>
    where
        I: Iterator<Item = char>,
    {
        let mut segments = Vec::new();
        let mut literal = String::new();

        while let Some(ch) = chars.peek().copied() {
            match ch {
                c if c.is_whitespace() => break,
                '|' | '&' | ';' | '>' | '<' | '(' | ')' => break,
                '\'' => {
                    flush_literal(&mut literal, &mut segments, QuoteKind::Unquoted);
                    chars.next();
                    self.read_single_quoted(chars, &mut segments)?;
                }
                '"' => {
                    flush_literal(&mut literal, &mut segments, QuoteKind::Unquoted);
                    chars.next();
                    self.read_double_quoted(chars, &mut segments)?;
                }
                '\\' => {
                    chars.next();
                    match chars.next() {
                        Some(c) => literal.push(c),
                        None => {
                            return Err(ShellError::message("unterminated escape sequence"));
                        }
                    }
                }
                '$' => {
                    flush_literal(&mut literal, &mut segments, QuoteKind::Unquoted);
                    chars.next();
                    self.read_variable(chars, &mut segments, QuoteKind::Unquoted)?;
                }
                other => {
                    chars.next();
                    literal.push(other);
                }
            }
        }

        flush_literal(&mut literal, &mut segments, QuoteKind::Unquoted);

        Ok(Word::new(segments))
    }

    fn read_single_quoted<I>(
        &self,
        chars: &mut std::iter::Peekable<I>,
        segments: &mut Vec<WordSegment>,
    ) -> ShellResult<()>
    where
        I: Iterator<Item = char>,
    {
        let mut text = String::new();

        loop {
            match chars.next() {
                Some('\'') => break,
                Some(c) => text.push(c),
                None => {
                    return Err(ShellError::message("unterminated single-quoted string"));
                }
            }
        }

        if !text.is_empty() {
            segments.push(WordSegment::Literal {
                text,
                quote: QuoteKind::SingleQuoted,
            });
        }

        Ok(())
    }

    fn read_double_quoted<I>(
        &self,
        chars: &mut std::iter::Peekable<I>,
        segments: &mut Vec<WordSegment>,
    ) -> ShellResult<()>
    where
        I: Iterator<Item = char>,
    {
        let mut literal = String::new();

        loop {
            match chars.peek().copied() {
                Some('"') => {
                    chars.next();
                    break;
                }
                Some('\\') => {
                    chars.next();
                    match chars.next() {
                        Some('"') => literal.push('"'),
                        Some('\\') => literal.push('\\'),
                        Some('$') => literal.push('$'),
                        Some(other) => {
                            literal.push('\\');
                            literal.push(other);
                        }
                        None => {
                            return Err(ShellError::message(
                                "unterminated escape in double-quoted string",
                            ));
                        }
                    }
                }
                Some('$') => {
                    flush_literal(&mut literal, segments, QuoteKind::DoubleQuoted);
                    chars.next();
                    self.read_variable(chars, segments, QuoteKind::DoubleQuoted)?;
                }
                Some(c) => {
                    chars.next();
                    literal.push(c);
                }
                None => {
                    return Err(ShellError::message("unterminated double-quoted string"));
                }
            }
        }

        flush_literal(&mut literal, segments, QuoteKind::DoubleQuoted);

        Ok(())
    }

    fn read_variable<I>(
        &self,
        chars: &mut std::iter::Peekable<I>,
        segments: &mut Vec<WordSegment>,
        quote: QuoteKind,
    ) -> ShellResult<()>
    where
        I: Iterator<Item = char>,
    {
        match chars.peek().copied() {
            Some('?') => {
                chars.next();
                segments.push(WordSegment::LastStatus { quote });
                Ok(())
            }
            Some(c) if is_var_start(c) => {
                let mut name = String::new();

                while let Some(c) = chars.peek().copied() {
                    if is_var_continue(c) {
                        name.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }

                segments.push(WordSegment::Variable { name, quote });
                Ok(())
            }
            _ => {
                segments.push(WordSegment::Literal {
                    text: "$".to_string(),
                    quote,
                });
                Ok(())
            }
        }
    }
}

fn flush_literal(literal: &mut String, segments: &mut Vec<WordSegment>, quote: QuoteKind) {
    if !literal.is_empty() {
        segments.push(WordSegment::Literal {
            text: std::mem::take(literal),
            quote,
        });
    }
}

fn is_var_start(c: char) -> bool {
    c == '_' || c.is_ascii_alphabetic()
}

fn is_var_continue(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric()
}
