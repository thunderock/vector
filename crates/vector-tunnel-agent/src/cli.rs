//! Agent CLI surface — `run` (default), `reauth`, `status`, `--version`.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "vector-tunnel-agent",
    version,
    about = "Vector Tunnel Agent — Dev Tunnels host for PTY shells"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    /// Run the agent (default). Registers a tunnel and serves PTY shells.
    Run,
    /// Re-authenticate (clears stored token, prompts for fresh device flow).
    Reauth,
    /// Print current registration status (provider, expiry).
    Status,
}
