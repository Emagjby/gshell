use anyhow::Result;
use gshell::{runtime::BootstrapExecutor, shell::ShellState, ui::Repl};
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<()> {
    if let Some(code) = handle_cli_flag(std::env::args().skip(1)) {
        std::process::exit(code);
    }

    init_tracing();

    let state = ShellState::shared().await?;
    gshell::runtime::load_startup_file(state.clone()).await?;
    let executor = BootstrapExecutor;
    let mut repl = Repl::new(executor, state.clone()).await;

    repl.run(state).await?;

    Ok(())
}

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("gshell=info"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

fn handle_cli_flag(args: impl Iterator<Item = String>) -> Option<i32> {
    let args = args.collect::<Vec<_>>();

    match args.as_slice() {
        [] => None,
        [flag] if flag == "--help" || flag == "-h" => {
            print_help();
            Some(0)
        }
        [flag] if flag == "--version" || flag == "-V" => {
            println!("gshell {}", env!("CARGO_PKG_VERSION"));
            Some(0)
        }
        _ => {
            eprintln!(
                "gshell: unsupported arguments: {}\n\nRun `gshell --help` for usage.",
                args.join(" ")
            );
            Some(2)
        }
    }
}

fn print_help() {
    println!(
        "gshell {}\n\nUsage:\n  gshell\n  gshell --help\n  gshell --version\n\nStartup:\n  Reads ~/.gshrc when present.\n\nDocs:\n  Installation: https://github.com/emagjby/gshell/blob/main/docs/install.md\n  Configuration: https://github.com/emagjby/gshell/blob/main/docs/configuration.md",
        env!("CARGO_PKG_VERSION")
    );
}
