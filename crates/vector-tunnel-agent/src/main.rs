//! vector-tunnel-agent — Linux user-space binary entry point.
//! Async runtime + CLI dispatch + tracing init.

use clap::Parser;
use tracing_subscriber::EnvFilter;

use vector_tunnel_agent::{cli, host, token_cache};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tunnels=warn,russh=warn")),
        )
        .init();

    let cli = cli::Cli::parse();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        match cli.command.unwrap_or(cli::Command::Run) {
            cli::Command::Run => host::run().await,
            cli::Command::Reauth => {
                token_cache::clear()?;
                host::run().await
            }
            cli::Command::Status => host::status().await,
        }
    })
}
