//! AUTH-03 refresh chain tests. Plan 06-03 un-ignores these.

#[tokio::test]
#[ignore = "Wave-0 stub — Plan 06-03 fills in"]
async fn auth_401_refresh_retry_succeeds() {
    // Plan 06-03: first GET returns 401, refresh succeeds, second GET returns 200.
}

#[tokio::test]
#[ignore = "Wave-0 stub — Plan 06-03 fills in"]
async fn auth_refresh_fails_emits_unauthenticated() {
    // Plan 06-03: 401 then refresh fails → ClientError::Unauthenticated.
}
