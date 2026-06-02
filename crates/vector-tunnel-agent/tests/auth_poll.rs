//! Device-flow `poll_token` regression tests. Live here (not in `src/auth.rs`)
//! because D-08 arch-lint forbids `#[tokio::test]` in `src/` — the agent binary
//! owns its runtime via `main.rs`; async tests run in integration crates.

use vector_tunnel_agent::auth::{poll_token, DeviceCodeReply, GITHUB_CLIENT_ID};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

fn reply(interval_secs: u64) -> DeviceCodeReply {
    DeviceCodeReply {
        device_code: "dc-test".into(),
        user_code: "ABCD-1234".into(),
        verification_uri: "https://github.com/login/device".into(),
        interval_secs,
        expires_in_secs: 60,
    }
}

// Regression: GitHub intermittently returns `incorrect_device_code` on a poll
// issued before the code has propagated. It must be treated as transient
// (retry), not a fatal error.
#[tokio::test]
async fn poll_retries_through_incorrect_device_code() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": "incorrect_device_code",
            "error_description": "The device_code provided is not valid."
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "gho_ok",
            "expires_in": 3600
        })))
        .mount(&server)
        .await;

    let http = reqwest::Client::new();
    let (access, _refresh, expires_in) =
        poll_token(&http, &server.uri(), GITHUB_CLIENT_ID, &reply(0))
            .await
            .expect("must retry through incorrect_device_code, not bail");
    assert_eq!(access, "gho_ok");
    assert_eq!(expires_in, 3600);
}

#[tokio::test]
async fn poll_retries_through_authorization_pending() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": "authorization_pending"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "gho_ok2",
            "expires_in": 3600
        })))
        .mount(&server)
        .await;

    let http = reqwest::Client::new();
    let (access, ..) = poll_token(&http, &server.uri(), GITHUB_CLIENT_ID, &reply(0))
        .await
        .expect("pending then success");
    assert_eq!(access, "gho_ok2");
}
