use std::path::PathBuf;

use anyhow::Result;

use crate::transport::PtyTransport;

/// Inputs to `Domain::spawn`. Unified across all domains so Phase 4 mux can
/// route a single SpawnCommand through any Domain trait object.
#[derive(Debug, Clone, Default)]
pub struct SpawnCommand {
    /// Argv. None means "use the user's login shell".
    pub argv: Option<Vec<String>>,
    /// Working directory. None means "inherit".
    pub cwd: Option<PathBuf>,
    /// Initial PTY rows / cols.
    pub rows: u16,
    pub cols: u16,
    /// Extra env vars; TERM=xterm-256color is added by the transport itself.
    pub env: Vec<(String, String)>,
}

/// A `Domain` knows how to spawn a `PtyTransport`. Locked in Phase 2 (D-38).
///
/// Phase 2 ships `LocalDomain` fully; `CodespaceDomain` (Phase 7) and
/// `DevTunnelDomain` (Phase 8) ship as compile-time stubs with `unimplemented!()`
/// bodies — trait shape is final, only impls fill in later.
#[async_trait::async_trait]
pub trait Domain: Send + Sync {
    /// Open a new shell session. Returns a transport that the caller wires
    /// to a `vector_term::Term`.
    async fn spawn(&self, cmd: SpawnCommand) -> Result<Box<dyn PtyTransport>>;

    /// Human-readable label for logs and (later) tab UI.
    fn label(&self) -> String;

    /// True if the underlying connection is live. LocalDomain always returns true;
    /// remote domains track session liveness.
    fn is_alive(&self) -> bool;

    /// Re-establish the underlying transport. LocalDomain is a no-op
    /// (a fresh `spawn` is sufficient); remote domains implement this in Phase 9.
    async fn reconnect(&self) -> Result<()>;
}
