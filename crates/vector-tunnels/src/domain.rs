//! Convenience: end-to-end "connect to tunnel" using API + transport.
//!
//! Used by the picker actor (Plan 08-06) — `vector-mux` stays free of
//! `vector-tunnels` dep per WIN-04. The actor calls `connect_tunnel` to get a
//! `Box<dyn PtyTransport>` and hands it to `Mux::create_tab_async_with_transport`.
//!
//! Plan 09-02 adds `ReconnectableDevTunnelDomain`, the concrete
//! `vector_mux::Domain` impl whose `reconnect_one_shot` re-runs
//! `connect_tunnel` for the per-pane actor's reconnect loop.

use std::sync::Arc;

use async_trait::async_trait;
use vector_mux::{Domain as MuxDomain, PtyTransport, SpawnCommand};

use crate::api::DevTunnelsApi;
use crate::model::{AuthProvider, TunnelRecord};
use crate::transport::DevTunnelTransport;

pub async fn connect_tunnel(
    api: &DevTunnelsApi,
    auth: &AuthProvider,
    tunnel: &TunnelRecord,
    rows: u16,
    cols: u16,
) -> anyhow::Result<Box<dyn PtyTransport>> {
    let token = api.get_access_token(auth, &tunnel.tunnel_id).await?;
    let t = DevTunnelTransport::connect(tunnel.clone(), token, rows, cols).await?;
    Ok(Box::new(t))
}

/// `Domain` impl that powers Phase 9 `reconnect_one_shot`. Lives in
/// `vector-tunnels` (not `vector-mux`) per WIN-04. Constructed by the picker
/// actor (Plan 09-05) at the same moment as the initial `connect_tunnel`
/// call.
///
/// `auth_factory` is invoked on every reconnect attempt so the picker actor's
/// `MicrosoftTokenStore` can mint a fresh `AuthProvider` (the upstream MS
/// access token may have rotated between attempts).
pub struct ReconnectableDevTunnelDomain {
    api: Arc<DevTunnelsApi>,
    auth_factory: Arc<dyn Fn() -> AuthProvider + Send + Sync>,
    tunnel: TunnelRecord,
    label: String,
}

impl std::fmt::Debug for ReconnectableDevTunnelDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReconnectableDevTunnelDomain")
            .field("tunnel_id", &self.tunnel.tunnel_id)
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

impl ReconnectableDevTunnelDomain {
    pub fn new(
        api: Arc<DevTunnelsApi>,
        auth_factory: Arc<dyn Fn() -> AuthProvider + Send + Sync>,
        tunnel: TunnelRecord,
        label: String,
    ) -> Self {
        Self {
            api,
            auth_factory,
            tunnel,
            label,
        }
    }
}

#[async_trait]
impl MuxDomain for ReconnectableDevTunnelDomain {
    async fn spawn(&self, _cmd: SpawnCommand) -> anyhow::Result<Box<dyn PtyTransport>> {
        // Picker actor handles initial connect via connect_tunnel().
        // ReconnectableDevTunnelDomain only services reconnect_one_shot.
        anyhow::bail!("use reconnect_one_shot — picker actor owns initial connect")
    }

    fn label(&self) -> String {
        self.label.clone()
    }

    fn is_alive(&self) -> bool {
        true
    }

    async fn reconnect_one_shot(
        &self,
        rows: u16,
        cols: u16,
    ) -> anyhow::Result<Option<Box<dyn PtyTransport>>> {
        let auth = (self.auth_factory)();
        let t = connect_tunnel(&self.api, &auth, &self.tunnel, rows, cols).await?;
        Ok(Some(t))
    }
}
