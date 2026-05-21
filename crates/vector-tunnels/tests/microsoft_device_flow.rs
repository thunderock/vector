//! DT-02 Microsoft Device Flow tests against wiremock-scripted Microsoft `common` responses.

use std::time::Duration;

use serde_json::json;
use tokio_util::sync::CancellationToken;
use vector_tunnels::auth::{MicrosoftAuth, MicrosoftAuthError, MicrosoftTokens};
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CLIENT_ID: &str = "test-client-id";
const SCOPE: &str = "46da2f7e-b5ef-422a-9a4e-fb5e1cb7da14/.default";

fn auth_for(server: &MockServer) -> MicrosoftAuth {
    MicrosoftAuth::with_endpoints(
        CLIENT_ID,
        format!("{}/devicecode", server.uri()),
        format!("{}/token", server.uri()),
        SCOPE,
    )
}

async fn mock_device_code(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/devicecode"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_code": "DC",
            "user_code": "ABCD-1234",
            "verification_uri": "https://microsoft.com/devicelogin",
            "expires_in": 900,
            "interval": 5
        })))
        .mount(server)
        .await;
}

// Test 1
#[tokio::test]
async fn device_flow_start_parses_microsoft_shape() {
    let server = MockServer::start().await;
    mock_device_code(&server).await;

    let auth = auth_for(&server);
    let start = auth.start_device_flow().await.expect("start");
    assert_eq!(start.device_code, "DC");
    assert_eq!(start.user_code, "ABCD-1234");
    assert_eq!(start.verification_uri, "https://microsoft.com/devicelogin");
    assert_eq!(start.interval, Duration::from_secs(5));
    assert_eq!(start.expires_in, Duration::from_secs(900));
}

// Test 2
#[tokio::test]
async fn polling_success_returns_tokens() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "at",
            "refresh_token": "rt",
            "expires_in": 3600,
            "token_type": "Bearer",
            "scope": SCOPE
        })))
        .mount(&server)
        .await;

    let auth = auth_for(&server);
    let tokens: MicrosoftTokens = auth
        .poll_until_authorized(
            "DC",
            Duration::from_millis(10),
            Duration::from_secs(60),
            CancellationToken::new(),
        )
        .await
        .expect("poll success");
    assert_eq!(tokens.access_token, "at");
    assert_eq!(tokens.refresh_token.as_deref(), Some("rt"));
    let now = std::time::SystemTime::now();
    let expires_in = tokens
        .expires_at
        .duration_since(now)
        .expect("expires_at in future");
    assert!(
        expires_in <= Duration::from_secs(3601) && expires_in >= Duration::from_secs(3500),
        "expires_at roughly +3600s, got {expires_in:?}"
    );
}

// Test 3
#[tokio::test]
async fn polling_slow_down_doubles_interval() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("device_code"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "slow_down"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "after_slowdown",
            "token_type": "Bearer"
        })))
        .mount(&server)
        .await;

    let auth = auth_for(&server);
    let started = std::time::Instant::now();
    let tokens = auth
        .poll_until_authorized(
            "DC",
            Duration::from_millis(50),
            Duration::from_secs(60),
            CancellationToken::new(),
        )
        .await
        .expect("slow_down then success");
    let elapsed = started.elapsed();
    assert_eq!(tokens.access_token, "after_slowdown");
    // After slow_down, the next sleep is the doubled interval (~100ms),
    // so total elapsed should be ≥ 100ms.
    assert!(
        elapsed >= Duration::from_millis(90),
        "expected ≥90ms after slow_down doubling, got {elapsed:?}"
    );
}

// Test 4
#[tokio::test]
async fn polling_authorization_pending_keeps_polling_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "authorization_pending"
        })))
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "finally",
            "token_type": "Bearer"
        })))
        .mount(&server)
        .await;

    let auth = auth_for(&server);
    let tokens = auth
        .poll_until_authorized(
            "DC",
            Duration::from_millis(20),
            Duration::from_secs(10),
            CancellationToken::new(),
        )
        .await
        .expect("pending then success");
    assert_eq!(tokens.access_token, "finally");
}

// Test 5
#[tokio::test]
async fn polling_device_code_expired_returns_typed_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "authorization_pending"
        })))
        .mount(&server)
        .await;

    let auth = auth_for(&server);
    // expires_in shorter than the polling interval → first poll, then time runs out.
    let err = auth
        .poll_until_authorized(
            "DC",
            Duration::from_millis(50),
            Duration::from_millis(120),
            CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(err, MicrosoftAuthError::DeviceCodeExpired),
        "expected DeviceCodeExpired, got {err:?}"
    );
}

// Test 6
#[tokio::test]
async fn polling_cancellation_exits_within_one_interval() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "authorization_pending"
        })))
        .mount(&server)
        .await;

    let auth = auth_for(&server);
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(80)).await;
        cancel_clone.cancel();
    });
    let started = std::time::Instant::now();
    let err = auth
        .poll_until_authorized(
            "DC",
            Duration::from_millis(50),
            Duration::from_secs(60),
            cancel,
        )
        .await
        .unwrap_err();
    let elapsed = started.elapsed();
    assert!(
        matches!(err, MicrosoftAuthError::Cancelled),
        "expected Cancelled, got {err:?}"
    );
    assert!(
        elapsed < Duration::from_millis(250),
        "cancel should exit within one polling interval, took {elapsed:?}"
    );
}

// Test 7
#[tokio::test]
async fn refresh_success_returns_new_tokens() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "new_at",
            "refresh_token": "new_rt",
            "expires_in": 3600,
            "token_type": "Bearer"
        })))
        .mount(&server)
        .await;

    let auth = auth_for(&server);
    let tokens = auth.refresh("old_rt").await.expect("refresh");
    assert_eq!(tokens.access_token, "new_at");
    assert_eq!(tokens.refresh_token.as_deref(), Some("new_rt"));
}

// Test 8
#[tokio::test]
async fn refresh_invalid_grant_returns_refresh_expired() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "invalid_grant",
            "error_description": "AADSTS70008: The provided authorization code or refresh token has expired."
        })))
        .mount(&server)
        .await;

    let auth = auth_for(&server);
    let err = auth.refresh("expired_rt").await.unwrap_err();
    assert!(
        matches!(err, MicrosoftAuthError::RefreshExpired),
        "expected RefreshExpired, got {err:?}"
    );
}

// Test 9 — Debug never leaks tokens
#[test]
fn debug_format_never_leaks_token_bytes() {
    let tokens = MicrosoftTokens {
        access_token: "at_secret_value_xyz".into(),
        refresh_token: Some("rt_secret_value_xyz".into()),
        expires_at: std::time::SystemTime::now() + Duration::from_secs(3600),
    };
    let rendered = format!("{tokens:?}");
    assert!(
        !rendered.contains("at_secret_value_xyz"),
        "access_token leaked into Debug: {rendered}"
    );
    assert!(
        !rendered.contains("rt_secret_value_xyz"),
        "refresh_token leaked into Debug: {rendered}"
    );
    // Sanity: the metadata fields ARE present.
    assert!(
        rendered.contains("access_token_len"),
        "Debug should include access_token_len: {rendered}"
    );
    assert!(
        rendered.contains("has_refresh"),
        "Debug should include has_refresh: {rendered}"
    );
}
