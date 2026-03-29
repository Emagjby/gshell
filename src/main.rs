use anyhow::Result;
use gshell::{runtime::BootstrapExecutor, shell::ShellState, ui::Repl};
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    tracing::info!("starting gshell...");

    let state = ShellState::shared()?;
    let executor = BootstrapExecutor;
    let mut repl = Repl::new(executor);

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
