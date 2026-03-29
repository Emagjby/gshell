use crate::{
    ast::{CommandNode, Redirection, RedirectionKind, ShellExpr, SimpleCommand},
    lexer::{Lexer, Token},
    shell::{ShellError, ShellResult},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedCommand {
    Empty,
    Expr(ShellExpr),
}

#[derive(Debug, Default)]
pub struct Parser {
    lexer: Lexer,
}

impl Parser {
    pub fn parse(&self, input: &str) -> ShellResult<ParsedCommand> {
        if input.contains('\0') {
            return Err(ShellError::message("input contains a null byte"));
        }

        let tokens = self.lexer.tokenize(input)?;
        if tokens.is_empty() {
            return Ok(ParsedCommand::Empty);
        }

        let expr = self.parse_sequence(&tokens)?;
        Ok(ParsedCommand::Expr(expr))
    }

    fn parse_sequence(&self, tokens: &[Token]) -> ShellResult<ShellExpr> {
        if let Some(parts) = split_top_level(tokens, |t| matches!(t, Token::Semicolon)) {
            let mut exprs = Vec::new();
            for part in parts {
                exprs.push(self.parse_boolean(part)?);
            }
            return Ok(ShellExpr::Sequence(exprs));
        }

        self.parse_boolean(tokens)
    }

    fn parse_boolean(&self, tokens: &[Token]) -> ShellResult<ShellExpr> {
        if let Some((left, op, right)) =
            split_first_top_level(tokens, |t| matches!(t, Token::AndIf | Token::OrIf))
        {
            let lhs = self.parse_boolean(left)?;
            let rhs = self.parse_pipeline(right)?;

            return match op {
                Token::AndIf => Ok(ShellExpr::And(Box::new(lhs), Box::new(rhs))),
                Token::OrIf => Ok(ShellExpr::Or(Box::new(lhs), Box::new(rhs))),
                _ => Err(ShellError::message("invalid boolean operator")),
            };
        }

        self.parse_pipeline(tokens)
    }

    fn parse_pipeline(&self, tokens: &[Token]) -> ShellResult<ShellExpr> {
        if let Some(parts) = split_top_level(tokens, |t| matches!(t, Token::Pipe)) {
            let mut commands = Vec::new();
            for part in parts {
                commands.push(self.parse_command_node(part)?);
            }
            return Ok(ShellExpr::Pipeline(commands));
        }

        Ok(ShellExpr::Command(self.parse_command_node(tokens)?))
    }

    fn parse_command_node(&self, tokens: &[Token]) -> ShellResult<CommandNode> {
        if tokens.is_empty() {
            return Err(ShellError::message("expected command"));
        }

        if matches!(tokens.first(), Some(Token::LParen))
            && matches!(tokens.last(), Some(Token::RParen))
            && is_wrapped_group(tokens)
        {
            let inner = &tokens[1..tokens.len() - 1];
            let expr = self.parse_sequence(inner)?;
            return Ok(CommandNode::Group(Box::new(expr)));
        }

        let simple = self.parse_simple_command(tokens)?;
        Ok(CommandNode::Simple(simple))
    }

    fn parse_simple_command(&self, tokens: &[Token]) -> ShellResult<SimpleCommand> {
        let mut argv = Vec::new();
        let mut redirections = Vec::new();
        let mut i = 0;

        while i < tokens.len() {
            match &tokens[i] {
                Token::Word(word) => {
                    argv.push(word.clone());
                    i += 1;
                }
                Token::IoNumber(fd) => {
                    let op = tokens
                        .get(i + 1)
                        .ok_or_else(|| ShellError::message("expected redirection operator"))?;
                    let target = tokens
                        .get(i + 2)
                        .ok_or_else(|| ShellError::message("expected redirection target"))?;

                    let kind = redirection_kind(op)?;
                    let target_word = expect_word(target)?;

                    redirections.push(Redirection {
                        fd: Some(u32::from(*fd)),
                        kind,
                        target: target_word.to_string(),
                    });

                    i += 3;
                }
                Token::RedirectIn | Token::RedirectOut | Token::RedirectAppend => {
                    let target = tokens
                        .get(i + 1)
                        .ok_or_else(|| ShellError::message("expected redirection target"))?;

                    let kind = redirection_kind(&tokens[i])?;
                    let target_word = expect_word(target)?;

                    redirections.push(Redirection {
                        fd: None,
                        kind,
                        target: target_word.to_string(),
                    });

                    i += 2;
                }
                other => {
                    return Err(ShellError::message(format!(
                        "unexpected token in simple command: {other:?}"
                    )));
                }
            }
        }

        if argv.is_empty() && redirections.is_empty() {
            return Err(ShellError::message("expected simple command"));
        }

        Ok(SimpleCommand::with_redirections(argv, redirections))
    }
}

fn expect_word(token: &Token) -> ShellResult<&str> {
    match token {
        Token::Word(word) => Ok(word),
        _ => Err(ShellError::message("expected word")),
    }
}

fn redirection_kind(token: &Token) -> ShellResult<RedirectionKind> {
    match token {
        Token::RedirectIn => Ok(RedirectionKind::Input),
        Token::RedirectOut => Ok(RedirectionKind::OutputTruncate),
        Token::RedirectAppend => Ok(RedirectionKind::OutputAppend),
        _ => Err(ShellError::message("expected redirection operator")),
    }
}

fn split_top_level<F>(tokens: &[Token], pred: F) -> Option<Vec<&[Token]>>
where
    F: Fn(&Token) -> bool,
{
    let mut depth = 0usize;
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut found = false;

    for (idx, token) in tokens.iter().enumerate() {
        match token {
            Token::LParen => depth += 1,
            Token::RParen => depth = depth.saturating_sub(1),
            _ => {}
        }

        if depth == 0 && pred(token) {
            parts.push(&tokens[start..idx]);
            start = idx + 1;
            found = true;
        }
    }

    if found {
        parts.push(&tokens[start..]);
        Some(parts)
    } else {
        None
    }
}

fn split_first_top_level<F>(tokens: &[Token], pred: F) -> Option<(&[Token], &Token, &[Token])>
where
    F: Fn(&Token) -> bool,
{
    let mut depth = 0usize;

    for (idx, token) in tokens.iter().enumerate() {
        match token {
            Token::LParen => depth += 1,
            Token::RParen => depth = depth.saturating_sub(1),
            _ => {}
        }

        if depth == 0 && pred(token) {
            return Some((&tokens[..idx], token, &tokens[idx + 1..]));
        }
    }

    None
}

fn is_wrapped_group(tokens: &[Token]) -> bool {
    let mut depth = 0usize;

    for (idx, token) in tokens.iter().enumerate() {
        match token {
            Token::LParen => depth += 1,
            Token::RParen => {
                depth -= 1;
                if depth == 0 && idx != tokens.len() - 1 {
                    return false;
                }
            }
            _ => {}
        }
    }

    true
}
