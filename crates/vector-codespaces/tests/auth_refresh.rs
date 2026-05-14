//! AUTH-03 401-refresh chain tests.
use serde_json::json;
use vector_codespaces::{ClientError, CodespacesClient};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn auth_401_refresh_retry_succeeds() {
    let server = MockServer::start().await;
    // First /user/codespaces call: 401
    Mock::given(method("GET"))
        .and(path("/user/codespaces"))
        .respond_with(ResponseTemplate::new(401))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    // Refresh token endpoint: 200 with new token
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "gho_refreshed",
            "token_type": "bearer",
            "scope": "codespace read:user"
        })))
        .mount(&server)
        .await;
    // Second list call: 200
    Mock::given(method("GET"))
        .and(path("/user/codespaces"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "total_count": 0,
            "codespaces": []
        })))
        .mount(&server)
        .await;

    let client = CodespacesClient::new_for_test(
        server.uri(),
        "gho_old_access".to_string(),
        Some("ghr_refresh_token_value".to_string()),
        &format!("{}/login/oauth/access_token", server.uri()),
    )
    .unwrap();
    let list = client
        .list_with_refresh()
        .await
        .expect("401 then refresh then 200");
    assert_eq!(list.len(), 0);
}

#[tokio::test]
async fn auth_refresh_fails_emits_unauthenticated() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user/codespaces"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = CodespacesClient::new_for_test(
        server.uri(),
        "gho_old_access".to_string(),
        Some("ghr_bad_refresh".to_string()),
        &format!("{}/login/oauth/access_token", server.uri()),
    )
    .unwrap();
    let err = client.list_with_refresh().await.unwrap_err();
    assert!(matches!(err, ClientError::Unauthenticated), "got {err:?}");
}
