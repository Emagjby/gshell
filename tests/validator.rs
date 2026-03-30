use gshell::ui::validator::ParserValidator;
use reedline::{ValidationResult, Validator};

fn assert_validation(result: ValidationResult, expected: ValidationResult) {
    assert!(
        matches!(
            (result, expected),
            (ValidationResult::Incomplete, ValidationResult::Incomplete)
                | (ValidationResult::Complete, ValidationResult::Complete)
        ),
        "validator returned a different completion state"
    );
}

#[test]
fn trailing_pipe_is_incomplete() {
    let validator = ParserValidator::default();
    assert_validation(
        validator.validate("echo hello |"),
        ValidationResult::Incomplete,
    );
}

#[test]
fn unclosed_quote_is_incomplete() {
    let validator = ParserValidator::default();
    assert_validation(
        validator.validate("echo \"hello"),
        ValidationResult::Incomplete,
    );
}

#[test]
fn unclosed_subshell_is_incomplete() {
    let validator = ParserValidator::default();
    assert_validation(
        validator.validate("(echo hello"),
        ValidationResult::Incomplete,
    );
}

#[test]
fn complete_command_is_complete() {
    let validator = ParserValidator::default();
    assert_validation(validator.validate("echo hello"), ValidationResult::Complete);
}

#[test]
fn invalid_but_complete_input_does_not_force_multiline() {
    let validator = ParserValidator::default();
    assert_validation(validator.validate("&&"), ValidationResult::Complete);
}
