//! CS-01 / CS-02 REST tests. Plan 06-03 un-ignores these.

#[tokio::test]
#[ignore = "Wave-0 stub — Plan 06-03 fills in"]
async fn list_codespaces_fixture() {
    // Plan 06-03: load tests/fixtures/list_codespaces.json into wiremock,
    // assert CodespacesClient::list() returns Vec<Codespace> with parsed state/repo/branch.
}

#[tokio::test]
#[ignore = "Wave-0 stub — Plan 06-03 fills in"]
async fn state_other_variant_deserializes_as_unrecognized() {
    // Plan 06-03: pass state="Hibernated" → Codespace.state == CodespaceState::Unrecognized.
}

#[tokio::test]
#[ignore = "Wave-0 stub — Plan 06-03 fills in"]
async fn start_swallows_409() {
    // Plan 06-03: wiremock returns 409; assert client.start(name) returns Ok(()).
}

#[tokio::test]
#[ignore = "Wave-0 stub — Plan 06-03 fills in"]
async fn start_swallows_200_and_202() {
    // Plan 06-03: 200 and 202 each → Ok(()).
}

#[tokio::test]
#[ignore = "Wave-0 stub — Plan 06-03 fills in"]
async fn start_fails_on_500() {
    // Plan 06-03: 500 → ClientError::StartFailed { status: 500 }.
}

#[tokio::test]
#[ignore = "Wave-0 stub — Plan 06-03 fills in"]
async fn poll_terminates_on_available() {
    // Plan 06-03: wiremock scripted starting,starting,available; assert poll returns Available.
}

#[tokio::test]
#[ignore = "Wave-0 stub — Plan 06-03 fills in"]
async fn poll_times_out_at_120s() {
    // Plan 06-03: tokio::time::pause; assert PollTimeout after 120s.
}
