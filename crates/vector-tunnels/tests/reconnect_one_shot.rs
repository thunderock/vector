//! Plan 09-02 Task 2 / Test B — `ReconnectableDevTunnelDomain::reconnect_one_shot`.
//!
//! Strategy: wiremock the Dev Tunnels access-token endpoint so we can exercise
//! `connect_tunnel(..)` without a real relay. The downstream
//! `DevTunnelTransport::connect` is allowed to fail at the "tunnel has no
//! endpoints" guard (see `crates/vector-tunnels/src/transport.rs:205-208`) —
//! reaching that error proves:
//!   1. `auth_factory` was invoked (otherwise we wouldn't have a token to send),
//!   2. `get_access_token` succeeded (wiremock returned 200),
//!   3. `connect_tunnel` made it to the transport layer.
//!
//! Plus a second test that calls `reconnect_one_shot` twice and asserts the
//! `auth_factory` counter reads 2 — proves the factory is queried per attempt
//! (token-refresh story).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use vector_mux::Domain;
use vector_tunnels::{AuthProvider, DevTunnelsApi, ReconnectableDevTunnelDomain, TunnelRecord};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Spin up a wiremock that serves a connect-scope token for any tunnel id.
async fn mock_token_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/tunnels/t-fake/access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token": "fake-connect-scope-token"
        })))
        .mount(&server)
        .await;
    server
}

/// `TunnelRecord` with NO endpoints. `DevTunnelTransport::connect` short-
/// circuits with `Protocol("tunnel has no endpoints")` before touching the
/// SDK — perfect for asserting "we reached the connect step".
fn empty_endpoints_record() -> TunnelRecord {
    serde_json::from_str(
        r#"{"tunnelId":"t-fake","name":"vector-test","labels":["vector-agent: true"]}"#,
    )
    .expect("decode TunnelRecord")
}

#[tokio::test]
async fn reconnect_one_shot_reaches_connect_tunnel() {
    let server = mock_token_server().await;
    let api = Arc::new(DevTunnelsApi::with_base_url(server.uri()));
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_for_factory = counter.clone();

    let auth_factory: Arc<dyn Fn() -> AuthProvider + Send + Sync> = Arc::new(move || {
        counter_for_factory.fetch_add(1, Ordering::SeqCst);
        AuthProvider::Microsoft("fake-ms-token".into())
    });

    let domain = ReconnectableDevTunnelDomain::new(
        api,
        auth_factory,
        empty_endpoints_record(),
        "vector-test".into(),
    );

    let res = domain.reconnect_one_shot(24, 80).await;

    // The factory must have been invoked.
    assert_eq!(
        counter.load(Ordering::SeqCst),
        1,
        "auth_factory invoked exactly once per reconnect attempt"
    );

    // We expect an error that originates from the transport layer (no
    // endpoints), NOT from the auth/token layer. The Ok branch can't be
    // formatted (PtyTransport: !Debug) so destructure manually.
    let err = match res {
        Ok(_) => panic!("expected transport-layer error from empty endpoints"),
        Err(e) => e,
    };
    let chain = format!("{err:?} | {err}");
    assert!(
        chain.contains("no endpoints"),
        "expected 'tunnel has no endpoints' error from transport layer, got: {chain}"
    );
}

#[tokio::test]
async fn reconnect_one_shot_calls_auth_factory_each_time() {
    let server = mock_token_server().await;
    let api = Arc::new(DevTunnelsApi::with_base_url(server.uri()));
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_for_factory = counter.clone();

    let auth_factory: Arc<dyn Fn() -> AuthProvider + Send + Sync> = Arc::new(move || {
        counter_for_factory.fetch_add(1, Ordering::SeqCst);
        AuthProvider::Microsoft("fake-ms-token".into())
    });

    let domain = ReconnectableDevTunnelDomain::new(
        api,
        auth_factory,
        empty_endpoints_record(),
        "vector-test".into(),
    );

    // Two attempts — both will fail at the no-endpoints guard, which is fine.
    let _ = domain.reconnect_one_shot(24, 80).await;
    let _ = domain.reconnect_one_shot(24, 80).await;

    assert_eq!(
        counter.load(Ordering::SeqCst),
        2,
        "auth_factory must be invoked on every reconnect_one_shot call — proves token refresh path"
    );
}

#[tokio::test]
async fn label_returns_configured_string() {
    let server = mock_token_server().await;
    let api = Arc::new(DevTunnelsApi::with_base_url(server.uri()));
    let auth_factory: Arc<dyn Fn() -> AuthProvider + Send + Sync> =
        Arc::new(|| AuthProvider::Microsoft("tok".into()));

    let domain = ReconnectableDevTunnelDomain::new(
        api,
        auth_factory,
        empty_endpoints_record(),
        "vector-test-label".into(),
    );

    assert_eq!(domain.label(), "vector-test-label");
    assert!(domain.is_alive());
}
