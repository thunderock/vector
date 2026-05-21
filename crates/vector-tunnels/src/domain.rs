//! Convenience: end-to-end "connect to tunnel" using API + transport.
//!
//! Used by the picker actor (Plan 08-06) — `vector-mux` stays free of
//! `vector-tunnels` dep per WIN-04. The actor calls `connect_tunnel` to get a
//! `Box<dyn PtyTransport>` and hands it to `Mux::create_tab_async_with_transport`.

use crate::api::DevTunnelsApi;
use crate::model::{AuthProvider, TunnelRecord};
use crate::transport::DevTunnelTransport;
use vector_mux::PtyTransport;

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
