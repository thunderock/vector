//! Phase 8 / DT-02..04 — tokio actor for the Dev Tunnels picker.
//!
//! Mirrors Phase 6 `codespaces_actor` shape. Owns the `DevTunnelsApi` +
//! `MicrosoftAuth` + `MicrosoftTokenStore` handles; emits `UserEvent` variants
//! across the `EventLoopProxy` boundary. The picker UI never speaks REST.
//!
//! Pitfall 14: no derived Debug on the actor or any token-bearing fields.

use std::sync::Arc;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use winit::event_loop::EventLoopProxy;

use vector_mux::{Mux, PtyTransport, WindowId as MuxWindowId};
use vector_tunnels::{
    auth::{MicrosoftAuth, MicrosoftAuthError, MicrosoftTokenStore, MicrosoftTokens},
    domain::connect_tunnel,
    AuthProvider, DevTunnelsApi, TunnelRecord,
};

use crate::UserEvent;

/// UI-facing view of a `TunnelRecord` — no token-bearing fields.
#[derive(Debug, Clone)]
pub struct TunnelView {
    pub tunnel_id: String,
    pub display_name: String,
    pub host: String,
    /// Seconds since `last_updated_at`, clamped to >=0. `None` if API omitted it.
    pub last_seen_secs_ago: Option<u64>,
}

impl From<&TunnelRecord> for TunnelView {
    fn from(t: &TunnelRecord) -> Self {
        let host = t
            .endpoints
            .first()
            .map(|e| e.host_id.clone())
            .unwrap_or_default();
        let last_seen_secs_ago = t.last_updated().map(|when| {
            let now = chrono::Utc::now();
            let secs = (now - when).num_seconds().max(0);
            u64::try_from(secs).unwrap_or(0)
        });
        Self {
            tunnel_id: t.tunnel_id.clone(),
            display_name: t.display_name(),
            host,
            last_seen_secs_ago,
        }
    }
}

/// Commands accepted by the actor task.
pub enum Command {
    /// Refresh the picker list (REST → DevTunnelsLoaded / DevTunnelsAuthRequired / DevTunnelsLoadFailed).
    Load,
    /// Picker requested connect; actor performs handshake + Mux install.
    Connect {
        tunnel_id: String,
        rows: u16,
        cols: u16,
        window_id: MuxWindowId,
    },
    /// Menu "Sign in with Microsoft".
    StartMicrosoftSignIn,
    /// Menu "Sign out of Microsoft".
    SignOutMicrosoft,
}

/// Actor state. Note: derived Debug forbidden (Pitfall 14).
pub struct DevTunnelsActor {
    api: DevTunnelsApi,
    microsoft_auth: MicrosoftAuth,
    token_store: MicrosoftTokenStore,
    proxy: EventLoopProxy<UserEvent>,
    mux: Arc<Mux>,
}

// Manual Debug — never reach into MicrosoftAuth or token_store contents.
impl std::fmt::Debug for DevTunnelsActor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DevTunnelsActor")
            .field("api", &self.api)
            .finish_non_exhaustive()
    }
}

impl DevTunnelsActor {
    pub fn new(
        api: DevTunnelsApi,
        mux: Arc<Mux>,
        microsoft_auth: MicrosoftAuth,
        token_store: MicrosoftTokenStore,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Self {
        Self {
            api,
            microsoft_auth,
            token_store,
            proxy,
            mux,
        }
    }

    /// Spawn the actor on `handle` and return the command sender.
    pub fn spawn(self, handle: &tokio::runtime::Handle) -> mpsc::Sender<Command> {
        let (tx, mut rx) = mpsc::channel::<Command>(16);
        let mut actor = self;
        handle.spawn(async move {
            while let Some(cmd) = rx.recv().await {
                match cmd {
                    Command::Load => actor.handle_load().await,
                    Command::Connect {
                        tunnel_id,
                        rows,
                        cols,
                        window_id,
                    } => {
                        actor
                            .handle_connect(&tunnel_id, rows, cols, window_id)
                            .await;
                    }
                    Command::StartMicrosoftSignIn => actor.handle_start_microsoft_signin().await,
                    Command::SignOutMicrosoft => actor.handle_sign_out(),
                }
            }
        });
        tx
    }

    /// Read tokens from Keychain; pack into AuthProvider::Microsoft.
    fn auth_provider(&self) -> Option<AuthProvider> {
        let tokens = self.token_store.load().ok().flatten()?;
        Some(AuthProvider::Microsoft(tokens.access_token))
    }

    async fn handle_load(&mut self) {
        let Some(auth) = self.auth_provider() else {
            let _ = self.proxy.send_event(UserEvent::DevTunnelsAuthRequired);
            return;
        };
        match self.api.list_tunnels(&auth).await {
            Ok(records) => {
                let views: Vec<TunnelView> = records.iter().map(TunnelView::from).collect();
                let _ = self.proxy.send_event(UserEvent::DevTunnelsLoaded(views));
            }
            Err(vector_tunnels::ApiError::Unauthorized) => {
                if self.try_refresh().await.is_ok() {
                    if let Some(auth2) = self.auth_provider() {
                        match self.api.list_tunnels(&auth2).await {
                            Ok(records) => {
                                let views: Vec<TunnelView> =
                                    records.iter().map(TunnelView::from).collect();
                                let _ = self.proxy.send_event(UserEvent::DevTunnelsLoaded(views));
                                return;
                            }
                            Err(vector_tunnels::ApiError::Unauthorized) => {}
                            Err(e) => {
                                let _ = self
                                    .proxy
                                    .send_event(UserEvent::DevTunnelsLoadFailed(e.to_string()));
                                return;
                            }
                        }
                    }
                }
                let _ = self.proxy.send_event(UserEvent::DevTunnelsAuthRequired);
            }
            Err(e) => {
                let _ = self
                    .proxy
                    .send_event(UserEvent::DevTunnelsLoadFailed(e.to_string()));
            }
        }
    }

    /// Single refresh attempt. Returns Ok on success (tokens saved) or err.
    async fn try_refresh(&mut self) -> Result<(), MicrosoftAuthError> {
        let Some(tokens) = self.token_store.load()? else {
            return Err(MicrosoftAuthError::RefreshExpired);
        };
        let Some(rt) = tokens.refresh_token.as_deref() else {
            return Err(MicrosoftAuthError::RefreshExpired);
        };
        let new_tokens = self.microsoft_auth.refresh(rt).await?;
        self.token_store.save(&new_tokens)?;
        Ok(())
    }

    async fn handle_connect(
        &mut self,
        tunnel_id: &str,
        rows: u16,
        cols: u16,
        window_id: MuxWindowId,
    ) {
        let _ = self
            .proxy
            .send_event(UserEvent::DevTunnelConnectStarted(tunnel_id.to_string()));
        let Some(auth) = self.auth_provider() else {
            let _ = self.proxy.send_event(UserEvent::DevTunnelsAuthRequired);
            return;
        };
        // Re-list to locate the TunnelRecord — picker only passes id across the proxy boundary.
        let records = match self.api.list_tunnels(&auth).await {
            Ok(r) => r,
            Err(e) => {
                let _ = self.proxy.send_event(UserEvent::DevTunnelConnectFailed {
                    tunnel_id: tunnel_id.to_string(),
                    reason: e.to_string(),
                });
                return;
            }
        };
        let Some(tunnel) = records.iter().find(|r| r.tunnel_id == tunnel_id) else {
            let _ = self.proxy.send_event(UserEvent::DevTunnelConnectFailed {
                tunnel_id: tunnel_id.to_string(),
                reason: "tunnel not found (renamed or deleted)".into(),
            });
            return;
        };
        let transport: Box<dyn PtyTransport> =
            match connect_tunnel(&self.api, &auth, tunnel, rows, cols).await {
                Ok(t) => t,
                Err(e) => {
                    let _ = self.proxy.send_event(UserEvent::DevTunnelConnectFailed {
                        tunnel_id: tunnel_id.to_string(),
                        reason: e.to_string(),
                    });
                    return;
                }
            };
        match self
            .mux
            .create_tab_async_with_transport(window_id, transport, rows, cols)
            .await
        {
            Ok((tab_id, pane_id)) => {
                let _ = self.proxy.send_event(UserEvent::DevTunnelPaneReady {
                    window_id,
                    tab_id,
                    pane_id,
                });
            }
            Err(e) => {
                let _ = self.proxy.send_event(UserEvent::DevTunnelConnectFailed {
                    tunnel_id: tunnel_id.to_string(),
                    reason: e.to_string(),
                });
            }
        }
    }

    async fn handle_start_microsoft_signin(&mut self) {
        let dc = match self.microsoft_auth.start_device_flow().await {
            Ok(d) => d,
            Err(e) => {
                let _ = self
                    .proxy
                    .send_event(UserEvent::MicrosoftSignInFailed(e.to_string()));
                return;
            }
        };
        let cancel = CancellationToken::new();
        let _ = self
            .proxy
            .send_event(UserEvent::MicrosoftDeviceFlowStarted {
                user_code: dc.user_code.clone(),
                verification_uri: dc.verification_uri.clone(),
                expires_in: dc.expires_in,
                cancel: cancel.clone(),
            });
        // Poll inline (we're already on the actor task; one signin at a time).
        match self
            .microsoft_auth
            .poll_until_authorized(&dc.device_code, dc.interval, dc.expires_in, cancel)
            .await
        {
            Ok(tokens) => {
                let _ = save_then_announce(&self.token_store, &tokens, &self.proxy);
            }
            Err(MicrosoftAuthError::Cancelled) => {
                let _ = self.proxy.send_event(UserEvent::MicrosoftSignInCancelled);
            }
            Err(e) => {
                let _ = self
                    .proxy
                    .send_event(UserEvent::MicrosoftSignInFailed(e.to_string()));
            }
        }
    }

    fn handle_sign_out(&mut self) {
        let _ = self.token_store.clear();
    }
}

fn save_then_announce(
    store: &MicrosoftTokenStore,
    tokens: &MicrosoftTokens,
    proxy: &EventLoopProxy<UserEvent>,
) -> Result<(), MicrosoftAuthError> {
    store.save(tokens)?;
    let _ = proxy.send_event(UserEvent::MicrosoftSignedIn);
    Ok(())
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use vector_tunnels::{TunnelEndpoint, TunnelRecord};

    fn make_record(id: &str, name: &str) -> TunnelRecord {
        TunnelRecord {
            tunnel_id: id.into(),
            name: Some(format!("vector-{name}")),
            labels: vec!["vector-agent: true".into()],
            endpoints: vec![TunnelEndpoint {
                host_id: format!("{name}.host"),
                client_relay_uri: "wss://example".into(),
                host_public_keys: vec![],
            }],
            last_updated_at: Some("2026-05-21T12:00:00Z".into()),
        }
    }

    #[test]
    fn tunnel_view_from_record_strips_vector_prefix() {
        let r = make_record("tid1", "alpha");
        let v: TunnelView = (&r).into();
        assert_eq!(v.tunnel_id, "tid1");
        assert_eq!(v.display_name, "alpha");
        assert_eq!(v.host, "alpha.host");
        assert!(v.last_seen_secs_ago.is_some());
    }

    #[test]
    fn tunnel_view_handles_missing_last_updated() {
        let mut r = make_record("tid2", "beta");
        r.last_updated_at = None;
        let v: TunnelView = (&r).into();
        assert!(v.last_seen_secs_ago.is_none());
    }

    #[test]
    fn tunnel_view_handles_missing_endpoint() {
        let mut r = make_record("tid3", "gamma");
        r.endpoints.clear();
        let v: TunnelView = (&r).into();
        assert_eq!(v.host, "");
    }
}
