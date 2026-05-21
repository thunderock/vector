use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use xshell::Shell;

mod agent_dist;
mod dmg;
mod icon;
mod release;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Build the unsigned (Universal) DMG.
    Dmg {
        #[arg(long)]
        universal: bool,
        #[arg(long)]
        arm64: Option<PathBuf>,
        #[arg(long = "x86_64")]
        x86_64: Option<PathBuf>,
    },
    /// Bump CalVer + run git-cliff + commit + tag (no push).
    Release,
    /// Build the vector-tunnel-agent .deb for the host arch (Linux only).
    AgentDist,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let sh = Shell::new()?;
    sh.change_dir(workspace_root()?);
    match cli.cmd {
        Cmd::Dmg {
            universal: true,
            arm64,
            x86_64,
        } => dmg::dmg_universal(&sh, arm64, x86_64),
        Cmd::Dmg {
            universal: false, ..
        } => dmg::dmg_local(&sh),
        Cmd::Release => release::release(&sh),
        Cmd::AgentDist => agent_dist::run(),
    }
}

fn workspace_root() -> Result<PathBuf> {
    let xtask_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    Ok(xtask_dir
        .parent()
        .ok_or_else(|| anyhow::anyhow!("xtask must be a child of the workspace root"))?
        .to_path_buf())
}
