use crate::{
    ast::{BoolOp, CommandNode, Redirection, RedirectionKind, ShellExpr, SimpleCommand},
    lexer::{Lexer, Token},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedCommand {
    Empty,
    Expr(ShellExpr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    Invalid,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub message: String,
}

pub type ParseResult<T> = Result<T, ParseError>;

impl ParseError {
    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            kind: ParseErrorKind::Invalid,
            message: message.into(),
        }
    }

    pub fn incomplete(message: impl Into<String>) -> Self {
        Self {
            kind: ParseErrorKind::Incomplete,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Default)]
pub struct Parser {
    lexer: Lexer,
}

impl Parser {
    pub fn parse(&self, input: &str) -> ParseResult<ParsedCommand> {
        if input.contains('\0') {
            return Err(ParseError::invalid("input contains a null byte"));
        }

        if input.contains("<<")
            && let Some(parsed) = self.parse_with_heredocs(input)?
        {
            return Ok(parsed);
        }

        self.parse_complete_source(input)
    }

    fn parse_complete_source(&self, input: &str) -> ParseResult<ParsedCommand> {
        let tokens = self.tokenize(input)?;

        parse_tokens(tokens)
    }

    fn tokenize(&self, input: &str) -> ParseResult<Vec<Token>> {
        self.lexer.tokenize(input).map_err(|err| {
            let msg = err.to_string();

            if matches_incomplete_lex_error(&msg) {
                ParseError::incomplete(msg)
            } else {
                ParseError::invalid(msg)
            }
        })
    }

    fn parse_with_heredocs(&self, input: &str) -> ParseResult<Option<ParsedCommand>> {
        let mut command_source = String::new();
        let mut consumed = 0usize;

        for line in input.split_inclusive('\n') {
            command_source.push_str(line);
            consumed += line.len();

            let mut parsed = match self.parse_complete_source(&command_source) {
                Ok(parsed) => parsed,
                Err(err) if err.kind == ParseErrorKind::Incomplete => continue,
                Err(err) => return Err(err),
            };

            if heredoc_count(&parsed) == 0 {
                continue;
            }

            collect_heredoc_bodies(&mut parsed, &input[consumed..])?;
            return Ok(Some(parsed));
        }

        if !command_source.is_empty() {
            let mut parsed = self.parse_complete_source(&command_source)?;

            if heredoc_count(&parsed) > 0 {
                collect_heredoc_bodies(&mut parsed, "")?;
                return Ok(Some(parsed));
            }
        }

        Ok(None)
    }
}

fn parse_tokens(tokens: Vec<Token>) -> ParseResult<ParsedCommand> {
    if tokens.is_empty() {
        return Ok(ParsedCommand::Empty);
    }

    let mut cursor = TokenCursor::new(tokens);
    let expr = parse_sequence(&mut cursor)?;

    if !cursor.is_eof() {
        return Err(ParseError::invalid(format!(
            "unexpected trailing token: {:?}",
            cursor.peek()
        )));
    }

    Ok(ParsedCommand::Expr(expr))
}

fn matches_incomplete_lex_error(msg: &str) -> bool {
    matches!(
        msg,
        "unterminated single-quoted string"
            | "unterminated double-quoted string"
            | "unterminated escape in double-quoted string"
            | "unterminated escape sequence"
            | "unterminated command substitution"
    )
}

#[derive(Debug, Clone)]
struct TokenCursor {
    tokens: Vec<Token>,
    pos: usize,
}

impl TokenCursor {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }
}

fn parse_sequence(cursor: &mut TokenCursor) -> ParseResult<ShellExpr> {
    let mut exprs = Vec::new();
    exprs.push(parse_boolean_chain(cursor)?);

    while matches!(cursor.peek(), Some(Token::Semicolon)) {
        cursor.next();

        if cursor.is_eof() {
            return Err(ParseError::incomplete(
                "trailing ';' requires another command",
            ));
        }

        exprs.push(parse_boolean_chain(cursor)?);
    }

    if exprs.len() == 1 {
        Ok(exprs.remove(0))
    } else {
        Ok(ShellExpr::Sequence(exprs))
    }
}

fn parse_boolean_chain(cursor: &mut TokenCursor) -> ParseResult<ShellExpr> {
    let first = parse_pipeline(cursor)?;
    let mut rest = Vec::new();

    loop {
        let op = match cursor.peek() {
            Some(Token::AndIf) => BoolOp::And,
            Some(Token::OrIf) => BoolOp::Or,
            _ => break,
        };

        cursor.next();

        if cursor.is_eof() {
            return Err(ParseError::incomplete(
                "boolean operator requires another command",
            ));
        }

        let rhs = parse_pipeline(cursor)?;
        rest.push((op, rhs));
    }

    if rest.is_empty() {
        Ok(first)
    } else {
        Ok(ShellExpr::BooleanChain {
            first: Box::new(first),
            rest,
        })
    }
}

fn parse_pipeline(cursor: &mut TokenCursor) -> ParseResult<ShellExpr> {
    let mut commands = Vec::new();
    commands.push(parse_command(cursor)?);

    while matches!(cursor.peek(), Some(Token::Pipe)) {
        cursor.next();

        if cursor.is_eof() {
            return Err(ParseError::incomplete(
                "trailing '|' requires another command",
            ));
        }

        commands.push(parse_command(cursor)?);
    }

    if commands.len() == 1 {
        Ok(ShellExpr::Command(commands.remove(0)))
    } else {
        Ok(ShellExpr::Pipeline(commands))
    }
}

fn parse_command(cursor: &mut TokenCursor) -> ParseResult<CommandNode> {
    match cursor.peek() {
        Some(Token::LParen) => parse_subshell(cursor),
        Some(Token::Word(_))
        | Some(Token::IoNumber(_))
        | Some(Token::RedirectIn)
        | Some(Token::RedirectHeredoc)
        | Some(Token::RedirectOut)
        | Some(Token::RedirectAppend) => parse_simple_command(cursor),
        Some(Token::RParen) => Err(ParseError::invalid("unexpected ')'")),
        Some(Token::Pipe | Token::AndIf | Token::OrIf | Token::Semicolon) => {
            Err(ParseError::invalid("expected command"))
        }
        None => Err(ParseError::incomplete("expected command")),
    }
}

fn parse_subshell(cursor: &mut TokenCursor) -> ParseResult<CommandNode> {
    match cursor.next() {
        Some(Token::LParen) => {}
        _ => return Err(ParseError::invalid("expected '('")),
    }

    if cursor.is_eof() {
        return Err(ParseError::incomplete("unclosed subshell"));
    }

    let expr = parse_sequence(cursor)?;

    match cursor.next() {
        Some(Token::RParen) => Ok(CommandNode::Subshell(Box::new(expr))),
        None => Err(ParseError::incomplete("unclosed subshell")),
        other => Err(ParseError::invalid(format!(
            "expected ')' but found {:?}",
            other
        ))),
    }
}

fn parse_simple_command(cursor: &mut TokenCursor) -> ParseResult<CommandNode> {
    let mut assignments = Vec::new();
    let mut argv = Vec::new();
    let mut redirections = Vec::new();

    loop {
        match cursor.peek() {
            Some(Token::Word(word)) => {
                if argv.is_empty() {
                    if let Some((name, value)) = word.split_assignment() {
                        assignments.push((name, value));
                    } else {
                        argv.push(word.clone());
                    }
                } else {
                    argv.push(word.clone());
                }

                cursor.next();
            }
            Some(Token::IoNumber(_))
            | Some(Token::RedirectIn)
            | Some(Token::RedirectHeredoc)
            | Some(Token::RedirectOut)
            | Some(Token::RedirectAppend) => {
                redirections.push(parse_redirection(cursor)?);
            }
            _ => break,
        }
    }

    if assignments.is_empty() && argv.is_empty() && redirections.is_empty() {
        return Err(ParseError::invalid("expected simple command"));
    }

    Ok(CommandNode::Simple(SimpleCommand::with_assignments(
        assignments,
        argv,
        redirections,
    )))
}

fn parse_redirection(cursor: &mut TokenCursor) -> ParseResult<Redirection> {
    let mut fd = None;

    if let Some(Token::IoNumber(n)) = cursor.peek() {
        fd = Some(u32::from(*n));
        cursor.next();
    }

    let kind = match cursor.next() {
        Some(Token::RedirectIn) => RedirectionKind::Input,
        Some(Token::RedirectHeredoc) => RedirectionKind::HereDoc {
            body: String::new(),
            expand: true,
        },
        Some(Token::RedirectOut) => RedirectionKind::OutputTruncate,
        Some(Token::RedirectAppend) => RedirectionKind::OutputAppend,
        other => {
            return Err(ParseError::invalid(format!(
                "expected redirection operator, found {:?}",
                other
            )));
        }
    };

    let target = match cursor.next() {
        Some(Token::Word(word)) => word,
        None => return Err(ParseError::incomplete("redirection missing target")),
        other => {
            return Err(ParseError::invalid(format!(
                "redirection target must be a word, found {:?}",
                other
            )));
        }
    };

    Ok(Redirection { fd, kind, target })
}

fn heredoc_count(command: &ParsedCommand) -> usize {
    match command {
        ParsedCommand::Empty => 0,
        ParsedCommand::Expr(expr) => heredoc_count_expr(expr),
    }
}

fn heredoc_count_expr(expr: &ShellExpr) -> usize {
    match expr {
        ShellExpr::Command(node) => heredoc_count_node(node),
        ShellExpr::Pipeline(nodes) => nodes.iter().map(heredoc_count_node).sum(),
        ShellExpr::BooleanChain { first, rest } => {
            heredoc_count_expr(first)
                + rest
                    .iter()
                    .map(|(_, expr)| heredoc_count_expr(expr))
                    .sum::<usize>()
        }
        ShellExpr::Sequence(exprs) => exprs.iter().map(heredoc_count_expr).sum(),
    }
}

fn heredoc_count_node(node: &CommandNode) -> usize {
    match node {
        CommandNode::Simple(simple) => simple
            .redirections
            .iter()
            .filter(|redir| matches!(redir.kind, RedirectionKind::HereDoc { .. }))
            .count(),
        CommandNode::Subshell(expr) => heredoc_count_expr(expr),
    }
}

fn collect_heredoc_bodies(command: &mut ParsedCommand, remainder: &str) -> ParseResult<()> {
    let ParsedCommand::Expr(expr) = command else {
        return Ok(());
    };

    let mut cursor = HeredocBodyCursor::new(remainder);
    fill_expr_heredocs(expr, &mut cursor)?;

    if cursor.has_remaining_content() {
        return Err(ParseError::invalid(
            "unexpected trailing content after heredoc terminator",
        ));
    }

    Ok(())
}

fn fill_expr_heredocs(expr: &mut ShellExpr, cursor: &mut HeredocBodyCursor<'_>) -> ParseResult<()> {
    match expr {
        ShellExpr::Command(node) => fill_node_heredocs(node, cursor),
        ShellExpr::Pipeline(nodes) => {
            for node in nodes {
                fill_node_heredocs(node, cursor)?;
            }

            Ok(())
        }
        ShellExpr::BooleanChain { first, rest } => {
            fill_expr_heredocs(first, cursor)?;
            for (_, expr) in rest {
                fill_expr_heredocs(expr, cursor)?;
            }

            Ok(())
        }
        ShellExpr::Sequence(exprs) => {
            for expr in exprs {
                fill_expr_heredocs(expr, cursor)?;
            }

            Ok(())
        }
    }
}

fn fill_node_heredocs(
    node: &mut CommandNode,
    cursor: &mut HeredocBodyCursor<'_>,
) -> ParseResult<()> {
    match node {
        CommandNode::Simple(simple) => {
            for redirection in &mut simple.redirections {
                let RedirectionKind::HereDoc { body, expand } = &mut redirection.kind else {
                    continue;
                };

                *expand = !redirection.target.is_quoted();
                *body = cursor.collect_body(redirection.target.quote_removed_text())?;
            }

            Ok(())
        }
        CommandNode::Subshell(expr) => fill_expr_heredocs(expr, cursor),
    }
}

struct HeredocBodyCursor<'a> {
    lines: Vec<&'a str>,
    pos: usize,
}

impl<'a> HeredocBodyCursor<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            lines: source.split_inclusive('\n').collect(),
            pos: 0,
        }
    }

    fn collect_body(&mut self, delimiter: String) -> ParseResult<String> {
        let mut body = String::new();

        while let Some(line) = self.lines.get(self.pos).copied() {
            self.pos += 1;

            if normalize_heredoc_line(line) == delimiter {
                return Ok(body);
            }

            body.push_str(line);
        }

        Err(ParseError::incomplete(format!(
            "heredoc missing terminator: {delimiter}"
        )))
    }

    fn has_remaining_content(&self) -> bool {
        self.lines[self.pos..]
            .iter()
            .any(|line| !normalize_heredoc_line(line).is_empty())
    }
}

fn normalize_heredoc_line(line: &str) -> String {
    line.trim_end_matches(['\r', '\n']).to_string()
}
