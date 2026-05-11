use std::path::PathBuf;

use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::sync::mpsc;
use vector_pty::{LocalPty, SpawnCommand as PtySpawnCommand};

use crate::domain::{Domain, SpawnCommand};
use crate::transport::{PtyTransport, TransportKind};

pub struct LocalDomain {
    shell: PathBuf,
}

impl LocalDomain {
    /// $SHELL -> /etc/passwd -> /bin/zsh -> /bin/bash fallback chain.
    pub fn new() -> Result<Self> {
        let shell = resolve_shell()?;
        Ok(Self { shell })
    }

    pub fn with_shell(shell: PathBuf) -> Self {
        Self { shell }
    }
}

fn resolve_shell() -> Result<PathBuf> {
    if let Ok(s) = std::env::var("SHELL") {
        if !s.is_empty() {
            let p = PathBuf::from(s);
            if p.exists() {
                return Ok(p);
            }
        }
    }
    // /etc/passwd parse, keyed by current uid's name. Best-effort.
    if let Ok(uid) = std::process::Command::new("id").arg("-un").output() {
        let user = String::from_utf8_lossy(&uid.stdout).trim().to_string();
        if let Ok(passwd) = std::fs::read_to_string("/etc/passwd") {
            for line in passwd.lines() {
                let cols: Vec<&str> = line.split(':').collect();
                if cols.len() >= 7 && cols[0] == user {
                    let p = PathBuf::from(cols[6]);
                    if p.exists() {
                        return Ok(p);
                    }
                }
            }
        }
    }
    let zsh = PathBuf::from("/bin/zsh");
    if zsh.exists() {
        return Ok(zsh);
    }
    let bash = PathBuf::from("/bin/bash");
    if bash.exists() {
        return Ok(bash);
    }
    anyhow::bail!(
        "no shell found: $SHELL unset, /etc/passwd no match, /bin/zsh + /bin/bash absent"
    );
}

#[async_trait]
impl Domain for LocalDomain {
    async fn spawn(&self, cmd: SpawnCommand) -> Result<Box<dyn PtyTransport>> {
        let pty_cmd = PtySpawnCommand {
            argv: cmd.argv,
            cwd: cmd.cwd,
            rows: cmd.rows,
            cols: cmd.cols,
            env: cmd.env,
        };
        let pty = LocalPty::spawn(&self.shell, pty_cmd).context("LocalPty::spawn")?;
        Ok(Box::new(LocalTransport(pty)))
    }
    fn label(&self) -> String {
        "local".into()
    }
    fn is_alive(&self) -> bool {
        true
    }
    async fn reconnect(&self) -> Result<()> {
        Ok(())
    }
}

/// Newtype wrapper: impl PtyTransport without touching vector-pty.
/// Lives here (not in vector-pty) to avoid a vector-pty -> vector-mux dep cycle.
pub struct LocalTransport(LocalPty);

#[async_trait]
impl PtyTransport for LocalTransport {
    fn resize(&mut self, rows: u16, cols: u16, px_w: u16, px_h: u16) -> Result<()> {
        self.0.resize(rows, cols, px_w, px_h).map_err(Into::into)
    }
    async fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.0.write(bytes).await.map_err(Into::into)
    }
    fn take_reader(&mut self) -> Option<mpsc::Receiver<Vec<u8>>> {
        self.0.take_reader()
    }
    fn kind(&self) -> TransportKind {
        TransportKind::Local
    }
    async fn wait(&mut self) -> Result<Option<i32>> {
        self.0.wait().await.map_err(Into::into)
    }
}
