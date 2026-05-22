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
/// `LocalDomain` ships fully; `DevTunnelDomain` is a compile-time stub
/// with an `unimplemented!()` body — trait shape is final, real impl lands
/// in the tunnels phase. Remote transports are installed directly via
/// `Mux::create_tab_async_with_transport` so vector-mux stays russh-free
/// (WIN-04).
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

    /// LocalDomain returns Ok(None) — local PTY death is permanent.
    /// DevTunnelDomain re-runs connect_tunnel via ReconnectableDevTunnelDomain (Plan 09-02).
    /// `rows` / `cols` are the latest known terminal dims (D-08 discretion in 09-CONTEXT.md).
    async fn reconnect_one_shot(
        &self,
        rows: u16,
        cols: u16,
    ) -> Result<Option<Box<dyn PtyTransport>>>;
}
