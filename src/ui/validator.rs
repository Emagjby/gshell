use reedline::{ValidationResult, Validator};

use crate::parser::{ParseErrorKind, Parser};

#[derive(Debug, Default)]
pub struct ParserValidator {
    parser: Parser,
}

impl Validator for ParserValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        match self.parser.parse(line) {
            Ok(_) => ValidationResult::Complete,
            Err(err) => match err.kind {
                ParseErrorKind::Incomplete => ValidationResult::Incomplete,
                ParseErrorKind::Invalid => ValidationResult::Complete,
            },
        }
    }
}
