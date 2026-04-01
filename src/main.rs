use anyhow::Result;
use gshell::{
    parser::{ParsedCommand, Parser},
    runtime::{BootstrapExecutor, Executor},
    shell::{ShellAction, ShellState},
    ui::Repl,
};
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    if let Some(code) = handle_cli_flag(std::env::args().skip(1)).await? {
        std::process::exit(code);
    }

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

async fn handle_cli_flag(args: impl Iterator<Item = String>) -> Result<Option<i32>> {
    let args = args.collect::<Vec<_>>();

    match args.as_slice() {
        [] => Ok(None),
        [flag] if flag == "--help" || flag == "-h" => {
            print_help();
            Ok(Some(0))
        }
        [flag] if flag == "--version" || flag == "-V" => {
            println!("gshell {}", env!("CARGO_PKG_VERSION"));
            Ok(Some(0))
        }
        [flag] if flag == "-c" => {
            eprintln!("gshell: missing command for -c\n\nRun `gshell --help` for usage.");
            Ok(Some(2))
        }
        [flag, rest @ ..] if flag == "-c" => Ok(Some(run_command(&rest.join(" ")).await?)),
        _ => {
            eprintln!(
                "gshell: unsupported arguments: {}\n\nRun `gshell --help` for usage.",
                args.join(" ")
            );
            Ok(Some(2))
        }
    }
}

async fn run_command(command: &str) -> Result<i32> {
    let state = ShellState::shared().await?;
    gshell::runtime::load_startup_file(state.clone()).await?;

    let parsed = Parser::default().parse(command)?;

    if matches!(parsed, ParsedCommand::Empty) {
        return Ok(0);
    }

    let action = BootstrapExecutor.execute(state.clone(), &parsed).await?;

    let exit_code = match action {
        ShellAction::Continue(output) => {
            if !output.stdout.is_empty() {
                print!("{}", output.stdout);
            }

            if !output.stderr.is_empty() {
                eprint!("{}", output.stderr);
            }

            output.exit_code.as_u8()
        }
        ShellAction::Exit(code) => code.as_u8(),
    };

    Ok(i32::from(exit_code))
}

fn print_help() {
    println!(
        "gshell {}\n\nUsage:\n  gshell\n  gshell -c <command>\n  gshell --help\n  gshell --version\n\nStartup:\n  Reads ~/.gshrc when present.\n\nDocs:\n  Installation: https://github.com/emagjby/gshell/blob/main/docs/install.md\n  Configuration: https://github.com/emagjby/gshell/blob/main/docs/configuration.md",
        env!("CARGO_PKG_VERSION")
    );
}
