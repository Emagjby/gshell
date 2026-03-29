use crate::shell::{ShellError, ShellResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedCommand {
    Empty,
    Exit,
    Raw(String),
}

#[derive(Debug, Default)]
pub struct Parser;

impl Parser {
    pub fn parse(&self, input: &str) -> ShellResult<ParsedCommand> {
        let trimmed = input.trim();

        match trimmed {
            "" => Ok(ParsedCommand::Empty),
            "exit" => Ok(ParsedCommand::Exit),
            s if s.contains('\0') => Err(ShellError::message("input contains a null byte")),
            s => Ok(ParsedCommand::Raw(s.to_string())),
        }
    }
}
