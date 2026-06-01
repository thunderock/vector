//! Wave-2 Plan 08-04: list_tunnels REST integration tests (wiremock).

use vector_tunnels::api::{ApiError, DevTunnelsApi};
use vector_tunnels::model::AuthProvider;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn load_fixture() -> String {
    std::fs::read_to_string("tests/fixtures/dev_tunnels_list.json").expect("fixture present")
}

#[tokio::test]
async fn list_tunnels_filters_to_vector_agent_label() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/tunnels"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(load_fixture(), "application/json"))
        .mount(&server)
        .await;

    let api = DevTunnelsApi::with_base_url(server.uri());
    let tunnels = api
        .list_tunnels(&AuthProvider::GitHub("gho_test".into()))
        .await
        .expect("list ok");

    // Fixture has 5 tunnels; only 2 carry `vector-agent: true`.
    assert_eq!(tunnels.len(), 2);
    let display_names: Vec<String> = tunnels
        .iter()
        .map(vector_tunnels::TunnelRecord::display_name)
        .collect();
    assert!(display_names.iter().any(|n| n == "corp-box"));
    assert!(display_names.iter().any(|n| n == "home-server"));
}

#[tokio::test]
async fn list_tunnels_strips_vector_prefix() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/tunnels"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(load_fixture(), "application/json"))
        .mount(&server)
        .await;

    let api = DevTunnelsApi::with_base_url(server.uri());
    let tunnels = api
        .list_tunnels(&AuthProvider::GitHub("gho_test".into()))
        .await
        .unwrap();

    for t in &tunnels {
        assert!(
            !t.display_name().starts_with("vector-"),
            "display_name leaked prefix: {}",
            t.display_name()
        );
    }
}

#[tokio::test]
async fn list_tunnels_decodes_bare_array_envelope() {
    // Regression: the live relay returns a top-level JSON array, not {"value":[...]}.
    // An account with zero tunnels returns `[]` — must decode to an empty list, not error.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/tunnels"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("[]", "application/json"))
        .mount(&server)
        .await;

    let api = DevTunnelsApi::with_base_url(server.uri());
    let tunnels = api
        .list_tunnels(&AuthProvider::GitHub("gho_test".into()))
        .await
        .expect("empty bare array must decode to empty list");
    assert!(tunnels.is_empty());
}

#[tokio::test]
async fn list_tunnels_handles_401() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/tunnels"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let api = DevTunnelsApi::with_base_url(server.uri());
    let err = api
        .list_tunnels(&AuthProvider::Microsoft("expired-jwt".into()))
        .await
        .expect_err("must surface 401");
    assert!(matches!(err, ApiError::Unauthorized));
}

#[tokio::test]
async fn list_tunnels_sends_provider_auth_header_github() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/tunnels"))
        .and(header("Authorization", "github gho_xyz"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(load_fixture(), "application/json"))
        .mount(&server)
        .await;
    let api = DevTunnelsApi::with_base_url(server.uri());
    let res = api
        .list_tunnels(&AuthProvider::GitHub("gho_xyz".into()))
        .await;
    assert!(res.is_ok(), "header match: {res:?}");
}

#[tokio::test]
async fn list_tunnels_sends_provider_auth_header_microsoft() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/tunnels"))
        .and(header("Authorization", "Bearer ms-jwt"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(load_fixture(), "application/json"))
        .mount(&server)
        .await;
    let api = DevTunnelsApi::with_base_url(server.uri());
    let res = api
        .list_tunnels(&AuthProvider::Microsoft("ms-jwt".into()))
        .await;
    assert!(res.is_ok(), "header match: {res:?}");
}

#[tokio::test]
async fn get_access_token_returns_connect_token() {
    // Verified live: connect tokens are returned inline on the tunnel object via
    // GET /api/v1/tunnels/{id}?tokenScopes=connect → {"accessTokens":{"connect":..}}.
    // There is no POST .../access endpoint (it 401s).
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/tunnels/tunnel-vec-1"))
        .and(query_param("tokenScopes", "connect"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"tunnelId":"tunnel-vec-1","accessTokens":{"connect":"connect-scope-abc"}}"#,
            "application/json",
        ))
        .mount(&server)
        .await;
    let api = DevTunnelsApi::with_base_url(server.uri());
    let tok = api
        .get_access_token(&AuthProvider::GitHub("gho_test".into()), "tunnel-vec-1")
        .await
        .expect("token");
    assert_eq!(tok, "connect-scope-abc");
}

#[tokio::test]
async fn get_access_token_handles_401() {
    // The actor refreshes upstream auth on Unauthorized — keep that signal intact.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/tunnels/tunnel-vec-1"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let api = DevTunnelsApi::with_base_url(server.uri());
    let err = api
        .get_access_token(&AuthProvider::GitHub("gho_test".into()), "tunnel-vec-1")
        .await
        .expect_err("must surface 401");
    assert!(matches!(err, ApiError::Unauthorized));
}
