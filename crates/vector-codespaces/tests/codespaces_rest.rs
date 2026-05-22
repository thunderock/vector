//! CS-01 / CS-02 REST tests against wiremock-scripted GitHub responses.
mod support;

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use support::ensure_crypto_provider;
use tokio_util::sync::CancellationToken;
use vector_codespaces::{ClientError, CodespaceState, CodespacesClient};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn build_test_client(server: &MockServer) -> CodespacesClient {
    ensure_crypto_provider();
    let octo = octocrab::Octocrab::builder()
        .personal_token("gho_test_token".to_string())
        .base_uri(server.uri())
        .unwrap()
        .build()
        .unwrap();
    CodespacesClient::new(Arc::new(octo))
}

#[tokio::test]
async fn list_codespaces_fixture() {
    let server = MockServer::start().await;
    let fixture = include_str!("fixtures/list_codespaces.json");
    Mock::given(method("GET"))
        .and(path("/user/codespaces"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(fixture)
                .insert_header("content-type", "application/json"),
        )
        .mount(&server)
        .await;
    let client = build_test_client(&server);
    let list = client.list().await.expect("list");
    assert_eq!(list.len(), 5);
    assert_eq!(list[0].state, CodespaceState::Available);
    assert_eq!(list[0].repository.full_name, "octocat/hello-world");
    assert_eq!(list[0].git_status.ref_name, "main");
    assert_eq!(list[1].state, CodespaceState::Starting);
}

#[tokio::test]
async fn state_other_variant_deserializes_as_unrecognized() {
    let server = MockServer::start().await;
    let fixture = include_str!("fixtures/list_codespaces.json");
    Mock::given(method("GET"))
        .and(path("/user/codespaces"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(fixture)
                .insert_header("content-type", "application/json"),
        )
        .mount(&server)
        .await;
    let client = build_test_client(&server);
    let list = client.list().await.expect("list");
    // 5th row has state="Hibernated" — no such variant; #[serde(other)] catches it.
    assert_eq!(list[4].state, CodespaceState::Unrecognized);
}

#[tokio::test]
async fn start_swallows_409() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/user/codespaces/foo/start"))
        .respond_with(ResponseTemplate::new(409))
        .mount(&server)
        .await;
    let client = build_test_client(&server);
    client
        .start("foo")
        .await
        .expect("409 should swallow as success");
}

#[tokio::test]
async fn start_swallows_200_and_202() {
    for code in [200_u16, 202] {
        let server = MockServer::start().await;
        let name = format!("cs-{code}");
        Mock::given(method("POST"))
            .and(path(format!("/user/codespaces/{name}/start")))
            .respond_with(ResponseTemplate::new(code))
            .mount(&server)
            .await;
        let client = build_test_client(&server);
        client
            .start(&name)
            .await
            .unwrap_or_else(|_| panic!("code {code} should succeed"));
    }
}

#[tokio::test]
async fn start_fails_on_500() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/user/codespaces/badcs/start"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let client = build_test_client(&server);
    let err = client.start("badcs").await.unwrap_err();
    assert!(matches!(err, ClientError::StartFailed { status: 500 }));
}

#[tokio::test]
async fn poll_terminates_on_available() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user/codespaces/cs1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "cs1",
            "state": "Starting",
            "repository": { "full_name": "o/r" },
            "git_status": { "ref": "main" },
            "last_used_at": "2026-05-13T10:00:00Z"
        })))
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/user/codespaces/cs1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "cs1",
            "state": "Available",
            "repository": { "full_name": "o/r" },
            "git_status": { "ref": "main" },
            "last_used_at": "2026-05-13T10:00:00Z"
        })))
        .mount(&server)
        .await;
    let client = build_test_client(&server);
    let state = client
        .poll_until_available(
            "cs1",
            CancellationToken::new(),
            |_| {},
            Duration::from_secs(120),
        )
        .await
        .expect("poll");
    assert_eq!(state, CodespaceState::Available);
}

#[tokio::test(start_paused = true)]
async fn poll_times_out_at_120s() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user/codespaces/cs2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "cs2",
            "state": "Starting",
            "repository": { "full_name": "o/r" },
            "git_status": { "ref": "main" },
            "last_used_at": "2026-05-13T10:00:00Z"
        })))
        .mount(&server)
        .await;
    let client = build_test_client(&server);
    let h = tokio::spawn(async move {
        client
            .poll_until_available(
                "cs2",
                CancellationToken::new(),
                |_| {},
                Duration::from_secs(120),
            )
            .await
    });
    // Advance virtual time past the deadline.
    tokio::time::advance(Duration::from_secs(125)).await;
    let res = h.await.unwrap();
    assert!(matches!(res, Err(ClientError::PollTimeout)));
}

#[tokio::test]
async fn poll_cancellation_token() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/user/codespaces/cs3"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "name": "cs3",
            "state": "Starting",
            "repository": { "full_name": "o/r" },
            "git_status": { "ref": "main" },
            "last_used_at": "2026-05-13T10:00:00Z"
        })))
        .mount(&server)
        .await;
    let client = build_test_client(&server);
    let token = CancellationToken::new();
    let token_clone = token.clone();
    let h = tokio::spawn(async move {
        client
            .poll_until_available("cs3", token_clone, |_| {}, Duration::from_secs(120))
            .await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    token.cancel();
    let res = h.await.unwrap();
    assert!(matches!(res, Err(ClientError::Cancelled)));
}
