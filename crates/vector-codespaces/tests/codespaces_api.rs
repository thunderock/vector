//! Regression tests for `GitHubAuth::list_codespaces_direct`.
//!
//! Covers bugs we just fixed:
//! - 403 from /user/codespaces (missing `codespace` scope) → Unauthorized
//! - x-oauth-scopes missing `codespace` header path (still 403) → Unauthorized
//! - 401 token rejected → Unauthorized
//! - total_count=0, codespaces=[] → Ok(vec![])
//! - total_count>0, codespaces=[] → error (schema drift guard)
use serde_json::json;
use vector_codespaces::{AuthError, GitHubAuth};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use zeroize::Zeroizing;

const CLIENT_ID: &str = "Iv1.test_client_id";

fn make_auth(server: &MockServer) -> GitHubAuth {
    GitHubAuth::new_with_endpoints_and_api(
        &format!("{}/login/device/code", server.uri()),
        &format!("{}/login/oauth/access_token", server.uri()),
        CLIENT_ID,
        &server.uri(),
    )
    .expect("auth init")
}

// Regression: token missing `codespace` scope → GitHub returns 403 with
// x-oauth-scopes echoing only granted scopes (e.g. "read:user"). Must surface
// as Unauthorized so the actor can prompt re-auth, NOT as an empty list.
#[tokio::test]
async fn list_codespaces_missing_codespace_scope_returns_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user/codespaces"))
        .respond_with(
            ResponseTemplate::new(403)
                .insert_header("x-oauth-scopes", "read:user")
                .set_body_json(json!({
                    "message": "Resource not accessible by personal access token",
                    "documentation_url": "https://docs.github.com/rest"
                })),
        )
        .mount(&server)
        .await;

    let auth = make_auth(&server);
    let token = Zeroizing::new("gho_no_codespace_scope".to_string());
    let err = auth.list_codespaces_direct(&token).await.unwrap_err();
    assert!(
        matches!(err, AuthError::Unauthorized),
        "expected Unauthorized, got {err:?}"
    );
}

// Regression: bare 403 from /user/codespaces (no scope header) → Unauthorized.
#[tokio::test]
async fn list_codespaces_403_returns_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user/codespaces"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "message": "Forbidden"
        })))
        .mount(&server)
        .await;

    let auth = make_auth(&server);
    let token = Zeroizing::new("gho_forbidden".to_string());
    let err = auth.list_codespaces_direct(&token).await.unwrap_err();
    assert!(
        matches!(err, AuthError::Unauthorized),
        "expected Unauthorized, got {err:?}"
    );
}

// 401 (token outright rejected) must also surface as Unauthorized so the actor
// triggers re-auth.
#[tokio::test]
async fn list_codespaces_401_returns_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user/codespaces"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "message": "Bad credentials"
        })))
        .mount(&server)
        .await;

    let auth = make_auth(&server);
    let token = Zeroizing::new("gho_rejected".to_string());
    let err = auth.list_codespaces_direct(&token).await.unwrap_err();
    assert!(
        matches!(err, AuthError::Unauthorized),
        "expected Unauthorized, got {err:?}"
    );
}

// Genuine empty result: total_count=0 + codespaces=[] → Ok(vec![]).
#[tokio::test]
async fn list_codespaces_real_empty_returns_ok_empty() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user/codespaces"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "total_count": 0,
            "codespaces": []
        })))
        .mount(&server)
        .await;

    let auth = make_auth(&server);
    let token = Zeroizing::new("gho_valid".to_string());
    let list = auth
        .list_codespaces_direct(&token)
        .await
        .expect("real empty list");
    assert!(
        list.is_empty(),
        "expected empty list, got {} items",
        list.len()
    );
}

// Regression: total_count says there are codespaces but the array is empty —
// indicates schema drift (rows failed serde). Must surface as an error, NOT
// silently return an empty list.
#[tokio::test]
async fn list_codespaces_schema_drift_returns_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user/codespaces"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "total_count": 2,
            "codespaces": []
        })))
        .mount(&server)
        .await;

    let auth = make_auth(&server);
    let token = Zeroizing::new("gho_valid".to_string());
    let err = auth.list_codespaces_direct(&token).await.unwrap_err();
    assert!(
        matches!(err, AuthError::OAuth(ref msg) if msg.contains("total_count=2")),
        "expected OAuth error mentioning total_count=2, got {err:?}"
    );
}

// Happy path: well-formed response with one codespace parses correctly.
#[tokio::test]
async fn list_codespaces_success_parses_codespace() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user/codespaces"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-oauth-scopes", "codespace, read:user")
                .set_body_json(json!({
                    "total_count": 1,
                    "codespaces": [{
                        "name": "fictional-couscous-abc123",
                        "state": "Available",
                        "repository": { "full_name": "octocat/hello-world" },
                        "git_status": { "ref": "main" },
                        "last_used_at": "2026-05-01T12:34:56Z",
                        "display_name": "my codespace"
                    }]
                })),
        )
        .mount(&server)
        .await;

    let auth = make_auth(&server);
    let token = Zeroizing::new("gho_valid".to_string());
    let list = auth
        .list_codespaces_direct(&token)
        .await
        .expect("happy path");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "fictional-couscous-abc123");
    assert_eq!(list[0].repository.full_name, "octocat/hello-world");
}
