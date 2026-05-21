//! AUTH-01 device flow tests against wiremock-scripted GitHub responses.
use serde_json::json;
use vector_codespaces::{AuthError, GitHubAuth};
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CLIENT_ID: &str = "Iv1.test_client_id";

async fn mock_device_code(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/login/device/code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_code": "DEV-CODE-XYZ",
            "user_code": "WDJB-MJHT",
            "verification_uri": "https://github.com/login/device",
            "expires_in": 900,
            "interval": 1
        })))
        .mount(server)
        .await;
}

#[tokio::test]
async fn device_flow_request_code() {
    let server = MockServer::start().await;
    mock_device_code(&server).await;

    let auth = GitHubAuth::new_with_endpoints(
        &format!("{}/login/device/code", server.uri()),
        &format!("{}/login/oauth/access_token", server.uri()),
        CLIENT_ID,
    )
    .expect("auth init");

    let (display, _details) = auth.request_device_code().await.expect("request");
    assert_eq!(display.user_code, "WDJB-MJHT");
    assert_eq!(display.verification_uri, "https://github.com/login/device");
    assert_eq!(display.interval_secs, 1);
}

#[tokio::test]
async fn device_flow_poll_success() {
    let server = MockServer::start().await;
    mock_device_code(&server).await;
    // First poll: authorization_pending
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .and(body_string_contains("device_code"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "authorization_pending"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    // Second poll: success
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "gho_test_access_value_xyz",
            "token_type": "bearer",
            "scope": "codespace read:user"
        })))
        .mount(&server)
        .await;

    let auth = GitHubAuth::new_with_endpoints(
        &format!("{}/login/device/code", server.uri()),
        &format!("{}/login/oauth/access_token", server.uri()),
        CLIENT_ID,
    )
    .unwrap();
    let (_, details) = auth.request_device_code().await.unwrap();
    let tokens = auth.poll_for_token(details).await.expect("poll success");
    assert_eq!(tokens.access.as_str(), "gho_test_access_value_xyz");
    assert!(tokens.refresh.is_none());
}

#[tokio::test]
async fn device_flow_slow_down() {
    let server = MockServer::start().await;
    mock_device_code(&server).await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "slow_down"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "gho_after_slowdown",
            "token_type": "bearer",
            "scope": "codespace read:user"
        })))
        .mount(&server)
        .await;

    let auth = GitHubAuth::new_with_endpoints(
        &format!("{}/login/device/code", server.uri()),
        &format!("{}/login/oauth/access_token", server.uri()),
        CLIENT_ID,
    )
    .unwrap();
    let (_, details) = auth.request_device_code().await.unwrap();
    let tokens = auth
        .poll_for_token(details)
        .await
        .expect("slow_down then success");
    assert_eq!(tokens.access.as_str(), "gho_after_slowdown");
}

// Regression: GitHub returns HTTP 200 for authorization_pending (not RFC 8628's
// 400). Earlier oauth2-based impl bailed with "Failed to parse server response".
#[tokio::test]
async fn device_flow_github_200_pending_then_success() {
    let server = MockServer::start().await;
    mock_device_code(&server).await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "authorization_pending"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "gho_after_github200_pending",
            "token_type": "bearer",
            "scope": "codespace read:user"
        })))
        .mount(&server)
        .await;

    let auth = GitHubAuth::new_with_endpoints(
        &format!("{}/login/device/code", server.uri()),
        &format!("{}/login/oauth/access_token", server.uri()),
        CLIENT_ID,
    )
    .unwrap();
    let (_, details) = auth.request_device_code().await.unwrap();
    let tokens = auth
        .poll_for_token(details)
        .await
        .expect("github 200 pending");
    assert_eq!(tokens.access.as_str(), "gho_after_github200_pending");
}

// Regression: fetch_user_login uses a direct reqwest call (not octocrab) to
// avoid a tower buffer panic. Confirm GET /user returns the `login` field.
#[tokio::test]
async fn fetch_user_login_returns_login_on_200() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "login": "octocat",
            "id": 1,
            "type": "User"
        })))
        .mount(&server)
        .await;

    let auth = GitHubAuth::new_with_endpoints_and_api(
        &format!("{}/login/device/code", server.uri()),
        &format!("{}/login/oauth/access_token", server.uri()),
        CLIENT_ID,
        &server.uri(),
    )
    .unwrap();
    let token = zeroize::Zeroizing::new("gho_test_token".to_string());
    let login = auth.fetch_user_login(&token).await.expect("fetch login");
    assert_eq!(login, "octocat");
}

// Regression: non-200 responses must surface as an error, not silently return
// a bogus login.
#[tokio::test]
async fn fetch_user_login_errors_on_non_200() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "message": "Bad credentials"
        })))
        .mount(&server)
        .await;

    let auth = GitHubAuth::new_with_endpoints_and_api(
        &format!("{}/login/device/code", server.uri()),
        &format!("{}/login/oauth/access_token", server.uri()),
        CLIENT_ID,
        &server.uri(),
    )
    .unwrap();
    let token = zeroize::Zeroizing::new("gho_bad_token".to_string());
    let err = auth.fetch_user_login(&token).await.unwrap_err();
    assert!(
        matches!(err, AuthError::OAuth(_)),
        "expected OAuth error, got {err:?}"
    );
}

#[tokio::test]
async fn device_flow_expired() {
    let server = MockServer::start().await;
    mock_device_code(&server).await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "expired_token"
        })))
        .mount(&server)
        .await;

    let auth = GitHubAuth::new_with_endpoints(
        &format!("{}/login/device/code", server.uri()),
        &format!("{}/login/oauth/access_token", server.uri()),
        CLIENT_ID,
    )
    .unwrap();
    let (_, details) = auth.request_device_code().await.unwrap();
    let err = auth.poll_for_token(details).await.unwrap_err();
    assert!(
        matches!(err, AuthError::Expired),
        "expected Expired, got {err:?}"
    );
}
