use gshell::{
    ast::SimpleCommand,
    lexer::{Lexer, Token},
    parser::{ParsedCommand, Parser},
};

#[test]
fn plain_word_tokenization() {
    let lexer = Lexer;
    let tokens = lexer
        .tokenize("echo hello world")
        .expect("tokenization should succeed");

    assert_eq!(
        tokens,
        vec![
            Token::Word("echo".to_string()),
            Token::Word("hello".to_string()),
            Token::Word("world".to_string()),
        ]
    );
}

#[test]
fn single_quoted_parsing() {
    let parser = Parser::default();
    let parsed = parser
        .parse("echo 'hello world'")
        .expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Simple(SimpleCommand::new(vec![
            "echo".to_string(),
            "hello world".to_string(),
        ]))
    );
}

#[test]
fn double_quoted_parsing() {
    let parser = Parser::default();
    let parsed = parser
        .parse("echo \"hello world\"")
        .expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Simple(SimpleCommand::new(vec![
            "echo".to_string(),
            "hello world".to_string(),
        ]))
    );
}

#[test]
fn escaped_characters_parsing() {
    let parser = Parser::default();
    let parsed = parser
        .parse(r#"echo hello\ world \"quoted\""#)
        .expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Simple(SimpleCommand::new(vec![
            "echo".to_string(),
            "hello world".to_string(),
            "\"quoted\"".to_string(),
        ]))
    );
}

#[test]
fn unterminated_single_quote_errors() {
    let parser = Parser::default();
    let err = parser.parse("echo 'hello").expect_err("parse should fail");

    assert!(
        err.to_string()
            .contains("unterminated single-quoted string")
    );
}

#[test]
fn unterminated_double_quote_errors() {
    let parser = Parser::default();
    let err = parser.parse("echo \"hello").expect_err("parse should fail");

    assert!(
        err.to_string()
            .contains("unterminated double-quoted string")
    );
}
