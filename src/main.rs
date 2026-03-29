use anyhow::Result;
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    tracing::info!("starting gshell...");

    // Phase 1 includes bootstrapping only
    // REPL will come in P1-03
    println!("gshell bootstrap ready");

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
