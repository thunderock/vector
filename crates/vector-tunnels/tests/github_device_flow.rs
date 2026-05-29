//! DT-03 GitHub Device Flow tests against wiremock-scripted GitHub responses.
//! Two GitHub-specific divergences from the generic OAuth device flow:
//!   - `Accept: application/json` on both POSTs.
//!   - `slow_down` ADDS 5s to the interval (does NOT double it).
//!
//! Plus a `device_flow_disabled`-is-fatal case the Microsoft mirror lacked.

use std::time::Duration;

use serde_json::json;
use tokio_util::sync::CancellationToken;
use vector_tunnels::auth::{GitHubAuth, GitHubAuthError, GitHubTokens};
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CLIENT_ID: &str = "Iv1.test-client-id";
const SCOPE: &str = "";

fn auth_for(server: &MockServer) -> GitHubAuth {
    GitHubAuth::with_endpoints(
        CLIENT_ID,
        format!("{}/device/code", server.uri()),
        format!("{}/access_token", server.uri()),
        SCOPE,
    )
}

async fn mock_device_code(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/device/code"))
        .and(header("accept", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_code": "DC",
            "user_code": "ABCD-1234",
            "verification_uri": "https://github.com/login/device",
            "expires_in": 900,
            "interval": 5
        })))
        .mount(server)
        .await;
}

// Test 1 — device-code POST sends Accept: application/json and parses the shape.
#[tokio::test]
async fn device_flow_start_parses_github_shape() {
    let server = MockServer::start().await;
    mock_device_code(&server).await;

    let auth = auth_for(&server);
    let start = auth.start_device_flow().await.expect("start");
    assert_eq!(start.device_code, "DC");
    assert_eq!(start.user_code, "ABCD-1234");
    assert_eq!(start.verification_uri, "https://github.com/login/device");
    assert_eq!(start.interval, Duration::from_secs(5));
    assert_eq!(start.expires_in, Duration::from_secs(900));
}

// Test 1b — verification_url fallback (GitHub sometimes uses _url, not _uri).
#[tokio::test]
async fn device_flow_start_falls_back_to_verification_url() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_code": "DC",
            "user_code": "ABCD-1234",
            "verification_url": "https://github.com/login/device",
            "expires_in": 900,
            "interval": 5
        })))
        .mount(&server)
        .await;

    let auth = auth_for(&server);
    let start = auth.start_device_flow().await.expect("start");
    assert_eq!(start.verification_uri, "https://github.com/login/device");
}

// Test 2 — polling success returns tokens with Accept: application/json.
#[tokio::test]
async fn polling_success_returns_tokens() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/access_token"))
        .and(header("accept", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "at",
            "refresh_token": "rt",
            "expires_in": 28800,
            "token_type": "bearer"
        })))
        .mount(&server)
        .await;

    let auth = auth_for(&server);
    let tokens: GitHubTokens = auth
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
        expires_in <= Duration::from_secs(28801) && expires_in >= Duration::from_secs(28700),
        "expires_at roughly +28800s, got {expires_in:?}"
    );
}

// Test 2b — absent expires_in is a far-future sentinel, NOT unwrap_or(3600).
#[tokio::test]
async fn polling_success_without_expires_in_uses_far_future_sentinel() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "at",
            "token_type": "bearer"
        })))
        .mount(&server)
        .await;

    let auth = auth_for(&server);
    let tokens = auth
        .poll_until_authorized(
            "DC",
            Duration::from_millis(10),
            Duration::from_secs(60),
            CancellationToken::new(),
        )
        .await
        .expect("poll success");
    let now = std::time::SystemTime::now();
    let expires_in = tokens
        .expires_at
        .duration_since(now)
        .expect("expires_at in future");
    // Far-future sentinel — must be well beyond an hour so a non-expiring token
    // isn't treated as stale (Pitfall 4: NOT unwrap_or(3600)).
    assert!(
        expires_in > Duration::from_secs(365 * 24 * 3600),
        "absent expires_in should be far-future, got {expires_in:?}"
    );
}

// Test 3 — slow_down ADDS 5s to the interval (explicitly NOT doubled).
#[tokio::test]
async fn polling_slow_down_adds_five_seconds() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/access_token"))
        .and(body_string_contains("device_code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "slow_down"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "after_slowdown",
            "token_type": "bearer"
        })))
        .mount(&server)
        .await;

    let auth = auth_for(&server);
    let started = std::time::Instant::now();
    let tokens = auth
        .poll_until_authorized(
            "DC",
            Duration::from_millis(10),
            Duration::from_secs(60),
            CancellationToken::new(),
        )
        .await
        .expect("slow_down then success");
    let elapsed = started.elapsed();
    assert_eq!(tokens.access_token, "after_slowdown");
    // After slow_down with a 10ms start interval: ADD 5s -> next sleep ~5010ms.
    // Doubling would give ~20ms. Assert ≥5s to prove +5s (not doubling).
    assert!(
        elapsed >= Duration::from_secs(5),
        "expected ≥5s after slow_down +5s (NOT doubled), got {elapsed:?}"
    );
    // And bound it so we know it's +5s, not some larger multiplication.
    assert!(
        elapsed < Duration::from_secs(8),
        "slow_down should add only 5s, got {elapsed:?}"
    );
}

// Test 4 — authorization_pending keeps polling at the current interval.
#[tokio::test]
async fn polling_authorization_pending_keeps_polling_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "authorization_pending"
        })))
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "finally",
            "token_type": "bearer"
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

// Test 5 — device_flow_disabled is a distinct typed FATAL error.
#[tokio::test]
async fn polling_device_flow_disabled_returns_typed_fatal_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "device_flow_disabled",
            "error_description": "Device flow is not enabled for this app."
        })))
        .mount(&server)
        .await;

    let auth = auth_for(&server);
    let err = auth
        .poll_until_authorized(
            "DC",
            Duration::from_millis(20),
            Duration::from_secs(60),
            CancellationToken::new(),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(err, GitHubAuthError::DeviceFlowDisabled),
        "expected DeviceFlowDisabled, got {err:?}"
    );
}

// Test 6 — device code expiry maps to typed error.
#[tokio::test]
async fn polling_device_code_expired_returns_typed_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "authorization_pending"
        })))
        .mount(&server)
        .await;

    let auth = auth_for(&server);
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
        matches!(err, GitHubAuthError::DeviceCodeExpired),
        "expected DeviceCodeExpired, got {err:?}"
    );
}

// Test 7 — cancellation exits within one interval.
#[tokio::test]
async fn polling_cancellation_exits_within_one_interval() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
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
        matches!(err, GitHubAuthError::Cancelled),
        "expected Cancelled, got {err:?}"
    );
    assert!(
        elapsed < Duration::from_millis(250),
        "cancel should exit within one polling interval, took {elapsed:?}"
    );
}

// Test 8 — refresh success returns new tokens.
#[tokio::test]
async fn refresh_success_returns_new_tokens() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/access_token"))
        .and(header("accept", "application/json"))
        .and(body_string_contains("grant_type=refresh_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "new_at",
            "refresh_token": "new_rt",
            "expires_in": 28800,
            "token_type": "bearer"
        })))
        .mount(&server)
        .await;

    let auth = auth_for(&server);
    let tokens = auth.refresh("old_rt").await.expect("refresh");
    assert_eq!(tokens.access_token, "new_at");
    assert_eq!(tokens.refresh_token.as_deref(), Some("new_rt"));
}

// Test 9 — bad_refresh_token maps to RefreshExpired (GitHub's error name).
#[tokio::test]
async fn refresh_bad_refresh_token_returns_refresh_expired() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "bad_refresh_token",
            "error_description": "The refresh token passed is incorrect or expired."
        })))
        .mount(&server)
        .await;

    let auth = auth_for(&server);
    let err = auth.refresh("expired_rt").await.unwrap_err();
    assert!(
        matches!(err, GitHubAuthError::RefreshExpired),
        "expected RefreshExpired, got {err:?}"
    );
}

// Test 10 — Debug never leaks token bytes.
#[test]
fn debug_format_never_leaks_token_bytes() {
    let tokens = GitHubTokens {
        access_token: "at_secret_value_xyz".into(),
        refresh_token: Some("rt_secret_value_xyz".into()),
        expires_at: std::time::SystemTime::now() + Duration::from_secs(28800),
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
    assert!(
        rendered.contains("access_token_len"),
        "Debug should include access_token_len: {rendered}"
    );
    assert!(
        rendered.contains("has_refresh"),
        "Debug should include has_refresh: {rendered}"
    );
}
