use gshell::{
    ast::{BoolOp, CommandNode, Redirection, RedirectionKind, ShellExpr, SimpleCommand},
    expand::{QuoteKind, Word, WordSegment},
    lexer::{Lexer, Token},
    parser::{ParseErrorKind, ParsedCommand, Parser},
};

fn lit(text: &str) -> Word {
    Word::literal(text)
}

#[test]
fn operator_tokenization_works() {
    let lexer = Lexer;
    let tokens = lexer
        .tokenize("a | b && c || d ; (e)")
        .expect("tokenization should succeed");

    assert_eq!(
        tokens,
        vec![
            Token::Word(lit("a")),
            Token::Pipe,
            Token::Word(lit("b")),
            Token::AndIf,
            Token::Word(lit("c")),
            Token::OrIf,
            Token::Word(lit("d")),
            Token::Semicolon,
            Token::LParen,
            Token::Word(lit("e")),
            Token::RParen,
        ]
    );
}

#[test]
fn redirect_tokenization_works() {
    let lexer = Lexer;
    let tokens = lexer
        .tokenize("echo hi > out 2>> err < in <<EOF")
        .expect("tokenization should succeed");

    assert_eq!(
        tokens,
        vec![
            Token::Word(lit("echo")),
            Token::Word(lit("hi")),
            Token::RedirectOut,
            Token::Word(lit("out")),
            Token::IoNumber(2),
            Token::RedirectAppend,
            Token::Word(lit("err")),
            Token::RedirectIn,
            Token::Word(lit("in")),
            Token::RedirectHeredoc,
            Token::Word(lit("EOF")),
        ]
    );
}

#[test]
fn lexer_preserves_variable_segments() {
    let lexer = Lexer;
    let tokens = lexer
        .tokenize("echo $HOME $?")
        .expect("tokenization should succeed");

    assert_eq!(
        tokens,
        vec![
            Token::Word(Word::literal("echo")),
            Token::Word(Word::new(vec![WordSegment::Variable {
                name: "HOME".into(),
                quote: QuoteKind::Unquoted,
            }])),
            Token::Word(Word::new(vec![WordSegment::LastStatus {
                quote: QuoteKind::Unquoted,
            }])),
        ]
    );
}

#[test]
fn lexer_preserves_quote_context() {
    let lexer = Lexer;
    let tokens = lexer
        .tokenize(r#"echo '$HOME' "$HOME""#)
        .expect("tokenization should succeed");

    assert_eq!(
        tokens,
        vec![
            Token::Word(Word::literal("echo")),
            Token::Word(Word::new(vec![WordSegment::Literal {
                text: "$HOME".into(),
                quote: QuoteKind::SingleQuoted,
            }])),
            Token::Word(Word::new(vec![WordSegment::Variable {
                name: "HOME".into(),
                quote: QuoteKind::DoubleQuoted,
            }])),
        ]
    );
}

#[test]
fn function_definition_tokenization_works() {
    let lexer = Lexer;
    let tokens = lexer
        .tokenize("greet() { echo hi; }")
        .expect("tokenization should succeed");

    assert_eq!(
        tokens,
        vec![
            Token::Word(lit("greet")),
            Token::LParen,
            Token::RParen,
            Token::LBrace,
            Token::Word(lit("echo")),
            Token::Word(lit("hi")),
            Token::Semicolon,
            Token::RBrace,
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
            CommandNode::Simple(SimpleCommand::new(vec![lit("echo"), lit("hi")])),
            CommandNode::Simple(SimpleCommand::new(vec![lit("cat")])),
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
                lit("echo"),
                lit("hi")
            ]))),
            ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(vec![lit("pwd")]))),
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
                vec![lit("echo"), lit("hi")],
                vec![Redirection {
                    fd: None,
                    kind: RedirectionKind::OutputTruncate,
                    target: lit("out.txt"),
                }],
            )
        )))
    );
}

#[test]
fn parses_leading_assignments_as_command_prefix() {
    let parser = Parser::default();
    let parsed = parser
        .parse("NAME=value echo hi")
        .expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Expr(ShellExpr::Command(CommandNode::Simple(
            SimpleCommand::with_assignments(
                vec![("NAME".into(), lit("value"))],
                vec![lit("echo"), lit("hi")],
                vec![],
            )
        )))
    );
}

#[test]
fn parses_assignment_only_command() {
    let parser = Parser::default();
    let parsed = parser.parse("NAME=value").expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Expr(ShellExpr::Command(CommandNode::Simple(
            SimpleCommand::with_assignments(vec![("NAME".into(), lit("value"))], vec![], vec![])
        )))
    );
}

#[test]
fn parses_grouped_command_ast() {
    let parser = Parser::default();
    let parsed = parser.parse("(echo hi)").expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Expr(ShellExpr::Command(CommandNode::Subshell(Box::new(
            ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(vec![
                lit("echo"),
                lit("hi")
            ])))
        ))))
    );
}

#[test]
fn parses_function_definition_ast() {
    let parser = Parser::default();
    let parsed = parser
        .parse("greet() { echo hi; }")
        .expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Expr(ShellExpr::Command(CommandNode::FunctionDef {
            name: "greet".into(),
            body: Box::new(ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(
                vec![lit("echo"), lit("hi"),]
            )))),
        }))
    );
}

#[test]
fn malformed_function_definition_is_rejected() {
    let parser = Parser::default();
    let err = parser
        .parse("greet() { echo hi")
        .expect_err("parse should fail");

    assert_eq!(err.kind, ParseErrorKind::Incomplete);
    assert!(err.message.contains("unclosed function body"));
}

#[test]
fn pipeline_precedence_is_higher_than_boolean_chain() {
    let parser = Parser::default();
    let parsed = parser.parse("a | b && c").expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Expr(ShellExpr::BooleanChain {
            first: Box::new(ShellExpr::Pipeline(vec![
                CommandNode::Simple(SimpleCommand::new(vec![lit("a")])),
                CommandNode::Simple(SimpleCommand::new(vec![lit("b")])),
            ])),
            rest: vec![(
                BoolOp::And,
                ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(vec![lit("c")])))
            )],
        })
    );
}

#[test]
fn boolean_chain_parses_left_to_right() {
    let parser = Parser::default();
    let parsed = parser.parse("a && b || c").expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Expr(ShellExpr::BooleanChain {
            first: Box::new(ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(
                vec![lit("a")]
            )))),
            rest: vec![
                (
                    BoolOp::And,
                    ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(vec![lit("b")])))
                ),
                (
                    BoolOp::Or,
                    ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(vec![lit("c")])))
                ),
            ],
        })
    );
}

#[test]
fn sequence_parsing_works() {
    let parser = Parser::default();
    let parsed = parser.parse("a ; b ; c").expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Expr(ShellExpr::Sequence(vec![
            ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(vec![lit("a")]))),
            ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(vec![lit("b")]))),
            ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(vec![lit("c")]))),
        ]))
    );
}

#[test]
fn redirect_attaches_to_simple_command() {
    let parser = Parser::default();
    let parsed = parser
        .parse("echo hi 2>> err.log")
        .expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Expr(ShellExpr::Command(CommandNode::Simple(
            SimpleCommand::with_redirections(
                vec![lit("echo"), lit("hi")],
                vec![Redirection {
                    fd: Some(2),
                    kind: RedirectionKind::OutputAppend,
                    target: lit("err.log"),
                }]
            )
        )))
    );
}

#[test]
fn parses_heredoc_descriptor_and_body() {
    let parser = Parser::default();
    let parsed = parser
        .parse("cat <<EOF\nhello\nEOF\n")
        .expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Expr(ShellExpr::Command(CommandNode::Simple(
            SimpleCommand::with_redirections(
                vec![lit("cat")],
                vec![Redirection {
                    fd: None,
                    kind: RedirectionKind::HereDoc {
                        body: "hello\n".into(),
                        expand: true,
                    },
                    target: lit("EOF"),
                }],
            )
        )))
    );
}

#[test]
fn quoted_heredoc_delimiter_disables_expansion() {
    let parser = Parser::default();
    let parsed = parser
        .parse("cat <<'EOF'\nhello $HOME\nEOF\n")
        .expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Expr(ShellExpr::Command(CommandNode::Simple(
            SimpleCommand::with_redirections(
                vec![lit("cat")],
                vec![Redirection {
                    fd: None,
                    kind: RedirectionKind::HereDoc {
                        body: "hello $HOME\n".into(),
                        expand: false,
                    },
                    target: Word::new(vec![WordSegment::Literal {
                        text: "EOF".into(),
                        quote: QuoteKind::SingleQuoted,
                    }]),
                }],
            )
        )))
    );
}

#[test]
fn parser_reports_missing_heredoc_terminator_as_incomplete() {
    let parser = Parser::default();
    let err = parser
        .parse("cat <<EOF\nhello\n")
        .expect_err("parse should fail");

    assert_eq!(err.kind, ParseErrorKind::Incomplete);
    assert!(err.message.contains("heredoc missing terminator"));
}

#[test]
fn lexer_tokenizes_nested_command_substitution() {
    let lexer = Lexer;
    let tokens = lexer
        .tokenize("echo $(printf $(pwd))")
        .expect("tokenization should succeed");

    assert_eq!(
        tokens,
        vec![
            Token::Word(Word::literal("echo")),
            Token::Word(Word::new(vec![WordSegment::CommandSubstitution {
                expr: Box::new(ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(
                    vec![
                        Word::new(vec![WordSegment::Literal {
                            text: "printf".into(),
                            quote: QuoteKind::Unquoted,
                        }]),
                        Word::new(vec![WordSegment::CommandSubstitution {
                            expr: Box::new(ShellExpr::Command(CommandNode::Simple(
                                SimpleCommand::new(vec![Word::literal("pwd")])
                            ))),
                            quote: QuoteKind::Unquoted,
                        }])
                    ]
                )))),
                quote: QuoteKind::Unquoted,
            }])),
        ]
    );
}

#[test]
fn lexer_tokenizes_double_quoted_command_substitution() {
    let lexer = Lexer;
    let tokens = lexer
        .tokenize(r#"echo "$(pwd)""#)
        .expect("tokenization should succeed");

    assert_eq!(
        tokens,
        vec![
            Token::Word(Word::literal("echo")),
            Token::Word(Word::new(vec![WordSegment::CommandSubstitution {
                expr: Box::new(ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(
                    vec![Word::literal("pwd")]
                )))),
                quote: QuoteKind::DoubleQuoted,
            }])),
        ]
    );
}

#[test]
fn parser_reports_unclosed_command_substitution_as_incomplete() {
    let parser = Parser::default();
    let err = parser
        .parse("echo $(printf $(pwd)")
        .expect_err("parse should fail");

    assert_eq!(err.kind, ParseErrorKind::Incomplete);
    assert!(err.message.contains("unterminated command substitution"));
}

#[test]
fn parser_distinguishes_subshell_from_command_substitution() {
    let parser = Parser::default();
    let parsed = parser.parse("(echo hi)").expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Expr(ShellExpr::Command(CommandNode::Subshell(Box::new(
            ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(vec![
                lit("echo"),
                lit("hi"),
            ])))
        ))))
    );
}

#[test]
fn parser_keeps_command_substitution_inside_word_segments() {
    let parser = Parser::default();
    let parsed = parser
        .parse("echo prefix$(pwd)suffix")
        .expect("parse should succeed");

    assert_eq!(
        parsed,
        ParsedCommand::Expr(ShellExpr::Command(CommandNode::Simple(SimpleCommand::new(
            vec![
                Word::literal("echo"),
                Word::new(vec![
                    WordSegment::Literal {
                        text: "prefix".into(),
                        quote: QuoteKind::Unquoted,
                    },
                    WordSegment::CommandSubstitution {
                        expr: Box::new(ShellExpr::Command(CommandNode::Simple(
                            SimpleCommand::new(vec![Word::literal("pwd")])
                        ))),
                        quote: QuoteKind::Unquoted,
                    },
                    WordSegment::Literal {
                        text: "suffix".into(),
                        quote: QuoteKind::Unquoted,
                    },
                ]),
            ]
        ))))
    );
}
