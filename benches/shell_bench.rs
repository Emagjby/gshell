use criterion::{Criterion, criterion_group, criterion_main};
use gshell::{
    parser::Parser,
    runtime::{BootstrapExecutor, Executor},
    shell::ShellState,
};

fn bench_parser_simple_command(c: &mut Criterion) {
    let parser = Parser::default();

    c.bench_function("parse simple command", |b| {
        b.iter(|| {
            let _ = parser
                .parse(r#"echo "hello world" foo\ bar"#)
                .expect("parse should succeed");
        });
    });
}

fn bench_runtime_builtin_dispatch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
    let executor = BootstrapExecutor;
    let parser = Parser::default();

    c.bench_function("dispatch builtin echo", |b| {
        b.iter(|| {
            rt.block_on(async {
                let state = ShellState::shared()
                    .await
                    .expect("shell state should initialize");

                let command = parser
                    .parse(r#"echo "hello world""#)
                    .expect("parse should succeed");

                let _ = executor
                    .execute(state, &command)
                    .await
                    .expect("execution should succeed");
            });
        });
    });
}

fn bench_shell_state_startup(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");

    c.bench_function("shell state startup", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _ = ShellState::shared()
                    .await
                    .expect("shell state should initialize");
            });
        });
    });
}

criterion_group!(
    benches,
    bench_parser_simple_command,
    bench_runtime_builtin_dispatch,
    bench_shell_state_startup
);
criterion_main!(benches);
