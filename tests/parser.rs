use gshell::{
    ast::{CommandNode, Redirection, RedirectionKind, ShellExpr, SimpleCommand},
    lexer::{Lexer, Token},
    parser::{ParsedCommand, Parser},
};

#[test]
fn operator_tokenization_works() {
    let lexer = Lexer;
    let tokens = lexer
        .tokenize("a | b && c || d ; (e)")
        .expect("tokenization should succeed");

    assert_eq!(
        tokens,
        vec![
            Token::Word("a".into()),
            Token::Pipe,
            Token::Word("b".into()),
            Token::AndIf,
            Token::Word("c".into()),
            Token::OrIf,
            Token::Word("d".into()),
            Token::Semicolon,
            Token::LParen,
            Token::Word("e".into()),
            Token::RParen,
        ]
    );
}

#[test]
fn redirect_tokenization_works() {
    let lexer = Lexer;
    let tokens = lexer
        .tokenize("echo hi > out 2>> err < in")
        .expect("tokenization should succeed");

    assert_eq!(
        tokens,
        vec![
            Token::Word("echo".into()),
            Token::Word("hi".into()),
            Token::RedirectOut,
            Token::Word("out".into()),
            Token::IoNumber(2),
            Token::RedirectAppend,
            Token::Word("err".into()),
            Token::RedirectIn,
            Token::Word("in".into()),
        ]
    );
}

#[test]
fn parses_pipeline_ast() {
    let parser = Parser::default();
    let parsed = parser.parse("echo hi | cat").expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Expr(ShellExpr::Pipeline(vec![
            CommandNode::Simple(SimpleCommand::new(vec!["echo".into(), "hi".into()])),
            CommandNode::Simple(SimpleCommand::new(vec!["cat".into()])),
        ]))
    );
}

#[test]
fn parses_sequence_ast() {
    let parser = Parser::default();
    let parsed = parser.parse("echo hi ; pwd").expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Expr(ShellExpr::Sequence(vec![
            ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(vec![
                "echo".into(),
                "hi".into()
            ]))),
            ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(vec!["pwd".into()]))),
        ]))
    );
}

#[test]
fn parses_redirection_ast() {
    let parser = Parser::default();
    let parsed = parser
        .parse("echo hi > out.txt")
        .expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Expr(ShellExpr::Command(CommandNode::Simple(
            SimpleCommand::with_redirections(
                vec!["echo".into(), "hi".into()],
                vec![Redirection {
                    fd: None,
                    kind: RedirectionKind::OutputTruncate,
                    target: "out.txt".into(),
                }],
            )
        )))
    );
}

#[test]
fn parses_grouped_command_ast() {
    let parser = Parser::default();
    let parsed = parser.parse("(echo hi)").expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Expr(ShellExpr::Command(CommandNode::Group(Box::new(
            ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(vec![
                "echo".into(),
                "hi".into()
            ])))
        ))))
    );
}
