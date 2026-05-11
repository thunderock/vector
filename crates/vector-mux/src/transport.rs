use anyhow::Result;
use tokio::sync::mpsc;

/// Best-effort transport kind for diagnostics and (later) tab tint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    Local,
    Codespace,
    DevTunnel,
}

/// Byte-stream transport for a single shell session. Locked in Phase 2 (D-38).
///
/// Reads are pushed via a caller-owned mpsc channel so the parser task can drive
/// on `recv().await` without pinned-async-reader gymnastics. Writes are async at
/// the trait level; impls route to an internal writer task.
#[async_trait::async_trait]
pub trait PtyTransport: Send + 'static {
    /// Resize the transport. For local PTY this issues TIOCSWINSZ — kernel
    /// delivers SIGWINCH to the foreground pgrp (CORE-04).
    fn resize(&mut self, rows: u16, cols: u16, px_w: u16, px_h: u16) -> Result<()>;

    /// Write bytes toward the shell. Buffered internally.
    async fn write(&mut self, bytes: &[u8]) -> Result<()>;

    /// Take the receiving end of the output channel. Called once at startup;
    /// subsequent calls return `None`.
    fn take_reader(&mut self) -> Option<mpsc::Receiver<Vec<u8>>>;

    /// Best-effort transport kind for diagnostics and tab tint.
    fn kind(&self) -> TransportKind;

    /// Wait for the underlying shell/channel to exit. Returns the exit status
    /// when available, or `None` if the transport has no notion of exit.
    async fn wait(&mut self) -> Result<Option<i32>>;
}
