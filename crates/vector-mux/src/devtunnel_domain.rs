//! `DevTunnelDomain` — Phase 8 leaves this as a stub by design.
//!
//! Per WIN-04 (Phase 4 D-38): `vector-mux` must not depend on the dev-tunnels crate.
//! Phase 8 Dev Tunnel sessions are installed by callers that use the dev-tunnels
//! crate's `domain::connect_tunnel` helper, which produces a
//! `Box<dyn PtyTransport>` handed to `Mux::create_tab_async_with_transport`.
//! The `Domain::spawn` method below remains `unimplemented!()` and is
//! unreachable in v1 — the picker actor (Plan 08-06) NEVER routes through
//! `DevTunnelDomain::spawn`.

use anyhow::Result;
use async_trait::async_trait;

use crate::domain::{Domain, SpawnCommand};
use crate::transport::PtyTransport;

#[derive(Debug, Default)]
pub struct DevTunnelDomain {
    _private: (),
}

impl DevTunnelDomain {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Domain for DevTunnelDomain {
    async fn spawn(&self, _cmd: SpawnCommand) -> Result<Box<dyn PtyTransport>> {
        unimplemented!(
            "Use the dev-tunnels crate's connect_tunnel + Mux::create_tab_async_with_transport"
        )
    }
    fn label(&self) -> String {
        "dev_tunnel".into()
    }
    fn is_alive(&self) -> bool {
        false
    }
    async fn reconnect_one_shot(
        &self,
        _rows: u16,
        _cols: u16,
    ) -> Result<Option<Box<dyn PtyTransport>>> {
        unimplemented!(
            "Phase 9 Plan 02: ReconnectableDevTunnelDomain in crates/vector-tunnels/src/domain.rs"
        )
    }
}
