//! AUTH-01 device flow tests. Plan 06-02 un-ignores these.

#[tokio::test]
#[ignore = "Wave-0 stub — Plan 06-02 fills in"]
async fn device_flow_request_code() {
    // Plan 06-02: spin up wiremock, assert GitHubAuth::request_device_code
    // calls POST /login/device/code and returns DeviceCodeDisplay with
    // user_code + verification_uri + expires_at set.
}

#[tokio::test]
#[ignore = "Wave-0 stub — Plan 06-02 fills in"]
async fn device_flow_poll_success() {
    // Plan 06-02: wiremock scripted authorization_pending → success;
    // assert Zeroizing<String> token returned, never plain String escapes.
}

#[tokio::test]
#[ignore = "Wave-0 stub — Plan 06-02 fills in"]
async fn device_flow_slow_down() {
    // Plan 06-02: wiremock returns slow_down; assert interval bumps per RFC 8628 §3.5.
}

#[tokio::test]
#[ignore = "Wave-0 stub — Plan 06-02 fills in"]
async fn device_flow_expired() {
    // Plan 06-02: wiremock returns expired_token; assert AuthError::Expired.
}
