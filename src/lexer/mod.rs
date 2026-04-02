use crate::{
    ast::{CommandNode, ShellExpr, SimpleCommand},
    expand::{QuoteKind, Word, WordSegment},
    parser::{ParsedCommand, Parser},
    shell::{ShellError, ShellResult},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Word(Word),
    Pipe,
    Ampersand,
    AndIf,
    OrIf,
    Semicolon,
    RedirectIn,
    RedirectHeredoc,
    RedirectOut,
    RedirectAppend,
    LBrace,
    LParen,
    RBrace,
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
                        tokens.push(Token::Ampersand);
                    }
                }
                ';' => {
                    chars.next();
                    tokens.push(Token::Semicolon);
                }
                '{' if is_standalone_brace(chars.clone(), '{') => {
                    chars.next();
                    tokens.push(Token::LBrace);
                }
                '(' => {
                    chars.next();
                    tokens.push(Token::LParen);
                }
                '}' if is_standalone_brace(chars.clone(), '}') => {
                    chars.next();
                    tokens.push(Token::RBrace);
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
                    if chars.peek() == Some(&'<') {
                        chars.next();
                        tokens.push(Token::RedirectHeredoc);
                    } else {
                        tokens.push(Token::RedirectIn);
                    }
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
                            let word = self.read_word_with_literal(&mut chars, digits)?;
                            if !word.segments.is_empty() {
                                tokens.push(Token::Word(word));
                            }
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
        self.read_word_with_literal(chars, String::new())
    }

    fn read_word_with_literal<I>(
        &self,
        chars: &mut std::iter::Peekable<I>,
        mut literal: String,
    ) -> ShellResult<Word>
    where
        I: Iterator<Item = char>,
    {
        let mut segments = Vec::new();

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
                    self.read_dollar_expression(chars, &mut segments, QuoteKind::Unquoted)?
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
                        Some('$') => {
                            flush_literal(&mut literal, segments, QuoteKind::DoubleQuoted);
                            chars.next();
                            self.read_dollar_expression(chars, segments, QuoteKind::DoubleQuoted)?;
                        }
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
                    self.read_dollar_expression(chars, segments, QuoteKind::DoubleQuoted)?;
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

    fn read_dollar_expression<I>(
        &self,
        chars: &mut std::iter::Peekable<I>,
        segments: &mut Vec<WordSegment>,
        quote: QuoteKind,
    ) -> ShellResult<()>
    where
        I: Iterator<Item = char>,
    {
        match chars.peek().copied() {
            Some('(') => {
                chars.next();
                let expr = self.read_command_substitution(chars)?;
                segments.push(WordSegment::CommandSubstitution { expr, quote });
                Ok(())
            }
            _ => self.read_variable(chars, segments, quote),
        }
    }

    fn read_command_substitution<I>(
        &self,
        chars: &mut std::iter::Peekable<I>,
    ) -> ShellResult<Box<ShellExpr>>
    where
        I: Iterator<Item = char>,
    {
        let mut out = String::new();
        let mut depth = 1usize;

        while let Some(ch) = chars.next() {
            match ch {
                '\'' => {
                    out.push(ch);
                    self.read_raw_single_quoted(chars, &mut out)?;
                }
                '"' => {
                    out.push(ch);
                    self.read_raw_double_quoted(chars, &mut out)?;
                }
                '\\' => {
                    out.push(ch);
                    match chars.next() {
                        Some(next) => out.push(next),
                        None => {
                            return Err(ShellError::message("unterminated command substitution"));
                        }
                    }
                }
                '$' if chars.peek() == Some(&'(') => {
                    out.push('$');
                    out.push('(');
                    chars.next();
                    depth += 1;
                }
                '(' => {
                    out.push(ch);
                }
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return parse_command_substitution_expr(&out);
                    }
                    out.push(ch);
                }
                other => out.push(other),
            }
        }

        Err(ShellError::message("unterminated command substitution"))
    }

    fn read_raw_single_quoted<I>(
        &self,
        chars: &mut std::iter::Peekable<I>,
        out: &mut String,
    ) -> ShellResult<()>
    where
        I: Iterator<Item = char>,
    {
        loop {
            match chars.next() {
                Some('\'') => {
                    out.push('\'');
                    return Ok(());
                }
                Some(c) => out.push(c),
                None => return Err(ShellError::message("unterminated single-quoted string")),
            }
        }
    }

    fn read_raw_double_quoted<I>(
        &self,
        chars: &mut std::iter::Peekable<I>,
        out: &mut String,
    ) -> ShellResult<()>
    where
        I: Iterator<Item = char>,
    {
        loop {
            match chars.next() {
                Some('"') => {
                    out.push('"');
                    return Ok(());
                }
                Some('\\') => {
                    out.push('\\');
                    match chars.next() {
                        Some(next) => out.push(next),
                        None => {
                            return Err(ShellError::message(
                                "unterminated escape in double-quoted string",
                            ));
                        }
                    }
                }
                Some(c) => out.push(c),
                None => return Err(ShellError::message("unterminated double-quoted string")),
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

fn is_standalone_brace<I>(mut chars: std::iter::Peekable<I>, brace: char) -> bool
where
    I: Iterator<Item = char>,
{
    match chars.next() {
        Some(current) if current == brace => {}
        _ => return false,
    }

    match chars.peek().copied() {
        None => true,
        Some(next) => next.is_whitespace() || matches!(next, '|' | '&' | ';' | '>' | '<'),
    }
}

fn parse_command_substitution_expr(source: &str) -> ShellResult<Box<ShellExpr>> {
    match Parser::default()
        .parse(source)
        .map_err(|err| ShellError::message(err.to_string()))?
    {
        ParsedCommand::Expr(expr) | ParsedCommand::Background(expr) => Ok(Box::new(expr)),
        ParsedCommand::Empty => Ok(Box::new(ShellExpr::Command(CommandNode::Simple(
            SimpleCommand::new(Vec::new()),
        )))),
    }
}
