use anyhow::{bail, Context, Result};
use std::path::PathBuf;

/// `cargo xtask agent-dist` — builds the vector-tunnel-agent .deb for the host arch.
/// macOS is a no-op (cross-compile to Linux is CI-only via .github/workflows/agent-release.yml).
pub fn run() -> Result<()> {
    if cfg!(not(target_os = "linux")) {
        eprintln!("agent-dist: cross-compile to Linux is not supported locally.");
        eprintln!("agent-dist: push a v* tag to trigger .github/workflows/agent-release.yml.");
        return Ok(());
    }

    let status = std::process::Command::new("cargo")
        .args(["deb", "--version"])
        .status();
    if status.map(|s| !s.success()).unwrap_or(true) {
        bail!("cargo-deb not installed. Run `cargo install cargo-deb`.");
    }

    let st = std::process::Command::new("cargo")
        .args(["build", "--release", "-p", "vector-tunnel-agent"])
        .status()
        .context("cargo build")?;
    if !st.success() {
        bail!("cargo build failed");
    }

    let st = std::process::Command::new("cargo")
        .args(["deb", "-p", "vector-tunnel-agent", "--no-build"])
        .status()
        .context("cargo deb")?;
    if !st.success() {
        bail!("cargo deb failed");
    }

    let deb_dir = PathBuf::from("target/debian");
    eprintln!("agent-dist: .deb artifact(s) in {}", deb_dir.display());
    Ok(())
}
