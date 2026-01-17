mod cli;
mod connection;
mod executor;
mod formatter;
mod repl;
mod error;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    let cli = Cli::parse();
    
    cli.execute().await
}
