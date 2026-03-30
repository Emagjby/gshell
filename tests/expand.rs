use gshell::{
    expand::{QuoteKind, Word, WordSegment},
    shell::{ExitCode, ShellState},
};

#[tokio::test]
async fn expands_environment_variable() {
    let state = ShellState::shared()
        .await
        .expect("Failed to create shell state");
    {
        let mut guard = state.write().await;
        guard.set_env_var("NAME", "gencho");
    }

    let word = Word::new(vec![WordSegment::Variable {
        name: "NAME".into(),
        quote: QuoteKind::Unquoted,
    }]);

    let guard = state.read().await;
    assert_eq!(word.expand(&guard), "gencho");
}

#[tokio::test]
async fn expands_last_exit_status() {
    let state = ShellState::shared()
        .await
        .expect("Failed to create shell state");
    {
        let mut guard = state.write().await;
        guard.set_last_exit_status(ExitCode::new(42));
    }

    let word = Word::new(vec![WordSegment::LastStatus {
        quote: QuoteKind::Unquoted,
    }]);

    let guard = state.read().await;
    assert_eq!(word.expand(&guard), "42");
}

#[tokio::test]
async fn single_quoted_variable_does_not_expand() {
    let state = ShellState::shared()
        .await
        .expect("Failed to create shell state");
    {
        let mut guard = state.write().await;
        guard.set_env_var("NAME", "gencho");
    }

    let word = Word::new(vec![WordSegment::Variable {
        name: "NAME".into(),
        quote: QuoteKind::SingleQuoted,
    }]);

    let guard = state.read().await;
    assert_eq!(word.expand(&guard), "$NAME");
}

#[tokio::test]
async fn double_quoted_variable_does_expand() {
    let state = ShellState::shared()
        .await
        .expect("Failed to create shell state");
    {
        let mut guard = state.write().await;
        guard.set_env_var("NAME", "gencho");
    }

    let word = Word::new(vec![WordSegment::Variable {
        name: "NAME".into(),
        quote: QuoteKind::DoubleQuoted,
    }]);

    let guard = state.read().await;
    assert_eq!(word.expand(&guard), "gencho");
}

#[test]
fn assignment_split_supports_quoted_suffix() {
    let word = Word::new(vec![
        WordSegment::Literal {
            text: "NAME=".into(),
            quote: QuoteKind::Unquoted,
        },
        WordSegment::Literal {
            text: "hello world".into(),
            quote: QuoteKind::DoubleQuoted,
        },
    ]);

    assert_eq!(
        word.split_assignment(),
        Some((
            "NAME".into(),
            Word::new(vec![WordSegment::Literal {
                text: "hello world".into(),
                quote: QuoteKind::DoubleQuoted,
            }])
        ))
    );
}
