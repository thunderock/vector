//! RelayTunnelHost lifecycle: register a tunnel labeled `vector-agent` with
//! name `vector-{hostname}` (D-09/D-10), open the relay listener, accept
//! inbound channels, dispatch each to `session::run`.
//!
//! The exact SDK API surface used (verified against
//! microsoft/dev-tunnels@64048c1 at execution time):
//!
//!   tunnels::management::new_tunnel_management(user_agent) → TunnelClientBuilder
//!   builder.authorization_provider(StaticAuthorizationProvider(Authorization::*))
//!          .build() → TunnelManagementClient
//!   mgmt.create_tunnel(Tunnel { name, labels, .. }, NO_REQUEST_OPTIONS)
//!   tunnels::connections::RelayTunnelHost::new(locator, mgmt)
//!   host.add_port(&TunnelPort { port_number: VECTOR_PTY_PORT, ... })
//!   host.add_port_raw(port) → mpsc::UnboundedReceiver<ForwardedPortConnection>
//!   ForwardedPortConnection.into_rw() → AsyncRead + AsyncWrite
//!
//! Note: `Tunnel.labels` is `Vec<String>` in the Rust SDK (not a HashMap),
//! so we use the single string label `"vector-agent"` for D-10 filter matching.

use anyhow::{Context, Result};

use crate::auth;
use crate::session;
use crate::token_cache::{self, CachedToken, Provider};

/// Conventional port for the Vector PTY service on the tunnel. Picker
/// connects to this port; agent's `session::run` handles each accept.
pub const VECTOR_PTY_PORT: u16 = 16632;

/// Label key (D-10) — Vector clients filter the user's tunnel list by
/// matching this exact string in `Tunnel.labels`.
pub const VECTOR_AGENT_LABEL: &str = "vector-agent";

/// Build a Dev Tunnels-valid tunnel name from a hostname.
///
/// The Management API `Name` field must match `([a-z0-9][a-z0-9-]{1,47}[a-z0-9])|(^$)`
/// — lowercase alphanumeric + hyphens, 3-49 chars, starting/ending alphanumeric.
/// Raw macOS hostnames carry uppercase + a `.local` suffix (e.g.
/// `Ashutoshs-MacBook-Pro-4.local`), so lowercase, map non-`[a-z0-9]` to `-`,
/// collapse/trim hyphens, clamp to 49, and fall back when too short.
fn sanitize_tunnel_name(hostname: &str) -> String {
    let mut s: String = format!("vector-{hostname}")
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    while s.contains("--") {
        s = s.replace("--", "-");
    }
    let s: String = s.trim_matches('-').chars().take(49).collect();
    let s = s.trim_end_matches('-').to_string();
    if s.len() < 3 {
        "vector-agent".to_string()
    } else {
        s
    }
}

/// Entry point. Loads cached token (or runs device flow), registers tunnel,
/// opens relay host, accepts inbound channels, hands each to `session::run`.
pub async fn run() -> Result<()> {
    let token = ensure_token().await?;
    let hostname_str = hostname::get()
        .context("read hostname")?
        .to_string_lossy()
        .to_string();
    let tunnel_name = sanitize_tunnel_name(&hostname_str);

    tracing::info!(provider = ?token.provider, tunnel_name = %tunnel_name, "agent starting");

    let mgmt = build_mgmt_client(&token)?;
    let tunnel = register_tunnel(&mgmt, &tunnel_name).await?;
    let locator = ::tunnels::management::TunnelLocator::try_from(&tunnel)
        .map_err(|e| anyhow::anyhow!("could not derive tunnel locator: {e}"))?;

    eprintln!("vector-tunnel-agent: tunnel '{tunnel_name}' registered. Waiting for connections.");

    let host_token = extract_host_token(&tunnel)?;
    let mut host = ::tunnels::connections::RelayTunnelHost::new(locator, mgmt);
    let _handle = host
        .connect(&host_token)
        .await
        .context("relay host connect")?;

    let port = ::tunnels::contracts::TunnelPort {
        port_number: VECTOR_PTY_PORT,
        protocol: Some("auto".into()),
        ..Default::default()
    };
    let mut accepts = host
        .add_port_raw(&port)
        .await
        .context("add_port_raw vector-pty")?;

    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            biased;
            () = &mut shutdown => {
                tracing::info!("shutdown signal — closing tunnel");
                let _ = host.unregister().await;
                return Ok(());
            }
            maybe_conn = accepts.recv() => {
                if let Some(conn) = maybe_conn {
                    let rw = conn.into_rw();
                    tokio::spawn(async move {
                        if let Err(e) = session::run(rw).await {
                            tracing::warn!(error=%e, "session ended with error");
                        }
                    });
                } else {
                    tracing::info!("relay accept channel closed");
                    return Ok(());
                }
            }
        }
    }
}

// Async for symmetry with `run()` — a future enhancement may await a live
// Management API tunnel-state query here.
#[allow(clippy::unused_async)]
pub async fn status() -> Result<()> {
    let tok = token_cache::load()?;
    match tok {
        None => {
            println!("not registered (run `vector-tunnel-agent` to register)");
        }
        Some(t) => {
            println!("provider: {:?}", t.provider);
            println!("token expires_at_unix: {}", t.expires_at_unix);
        }
    }
    Ok(())
}

async fn ensure_token() -> Result<CachedToken> {
    if let Some(t) = token_cache::load()? {
        return Ok(t);
    }
    auth::run_first_run_device_flow()
        .await
        .map_err(anyhow::Error::from)
}

fn build_mgmt_client(t: &CachedToken) -> Result<::tunnels::management::TunnelManagementClient> {
    use ::tunnels::management::{new_tunnel_management, Authorization};
    let auth = match t.provider {
        Provider::GitHub => Authorization::Github(t.access_token.clone()),
        // Microsoft access tokens are AAD bearer JWTs.
        Provider::Microsoft => Authorization::AAD(t.access_token.clone()),
    };
    let mut builder = new_tunnel_management("vector-tunnel-agent");
    builder.authorization(auth);
    Ok(builder.into())
}

async fn register_tunnel(
    mgmt: &::tunnels::management::TunnelManagementClient,
    name: &str,
) -> Result<::tunnels::contracts::Tunnel> {
    use ::tunnels::contracts::Tunnel;
    use ::tunnels::management::NO_REQUEST_OPTIONS;
    let tunnel = Tunnel {
        name: Some(name.to_owned()),
        labels: vec![VECTOR_AGENT_LABEL.to_owned()],
        ..Default::default()
    };
    let created = mgmt
        .create_tunnel(tunnel, NO_REQUEST_OPTIONS)
        .await
        .context("create_tunnel")?;
    Ok(created)
}

fn extract_host_token(t: &::tunnels::contracts::Tunnel) -> Result<String> {
    // create_tunnel returns access_tokens keyed by scope; the host scope is
    // "host" per the SDK contracts. Fall back to the first available token if
    // the API ever renames the key.
    let map = t
        .access_tokens
        .as_ref()
        .context("tunnel has no access_tokens — Management API rejected scope grant?")?;
    if let Some(t) = map.get("host") {
        return Ok(t.clone());
    }
    map.values()
        .next()
        .cloned()
        .context("no host access token in tunnel response")
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let Ok(mut term) = signal(SignalKind::terminate()) else {
        let _ = tokio::signal::ctrl_c().await;
        return;
    };
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = term.recv() => {},
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::sanitize_tunnel_name;

    // Dev Tunnels Name regex: ([a-z0-9][a-z0-9-]{1,47}[a-z0-9])|(^$)
    fn is_valid(name: &str) -> bool {
        let bytes = name.as_bytes();
        (3..=49).contains(&name.len())
            && bytes
                .iter()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
            && bytes[0].is_ascii_alphanumeric()
            && bytes[name.len() - 1].is_ascii_alphanumeric()
    }

    #[test]
    fn macos_hostname_becomes_valid_tunnel_name() {
        // Regression: raw macOS hostname (uppercase + `.local`) was rejected 400.
        let name = sanitize_tunnel_name("Ashutoshs-MacBook-Pro-4.local");
        assert!(is_valid(&name), "invalid tunnel name: {name}");
        assert!(name.starts_with("vector-"));
        assert!(!name.contains('.') && name.chars().all(|c| !c.is_ascii_uppercase()));
    }

    #[test]
    fn plain_linux_hostname_passes_through() {
        let name = sanitize_tunnel_name("devbox");
        assert!(is_valid(&name));
        assert_eq!(name, "vector-devbox");
    }

    #[test]
    fn overlong_hostname_is_clamped_and_valid() {
        let name = sanitize_tunnel_name(&"a".repeat(80));
        assert!(is_valid(&name), "invalid tunnel name: {name}");
    }

    #[test]
    fn empty_or_garbage_hostname_falls_back() {
        assert!(is_valid(&sanitize_tunnel_name("")));
        assert!(is_valid(&sanitize_tunnel_name("...")));
    }
}
