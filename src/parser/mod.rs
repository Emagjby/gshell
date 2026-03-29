use crate::{
    ast::SimpleCommand,
    lexer::{Lexer, Token},
    shell::{ShellError, ShellResult},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedCommand {
    Empty,
    Simple(SimpleCommand),
}

#[derive(Debug, Default)]
pub struct Parser {
    lexer: Lexer,
}

impl Parser {
    pub fn parse(&self, input: &str) -> ShellResult<ParsedCommand> {
        if input.contains('\0') {
            return Err(ShellError::message("input contains null byte"));
        }

        let tokens = self.lexer.tokenize(input)?;

        if tokens.is_empty() {
            return Ok(ParsedCommand::Empty);
        }

        let mut argv = Vec::new();

        for token in tokens {
            match token {
                Token::Word(word) => argv.push(word),
            }
        }

        if argv.is_empty() {
            return Ok(ParsedCommand::Empty);
        }

        Ok(ParsedCommand::Simple(SimpleCommand::new(argv)))
    }
}
