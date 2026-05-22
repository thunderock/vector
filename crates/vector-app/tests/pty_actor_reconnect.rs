//! Wave 0 stubs for Plan 09-03 (PERSIST-01/02 state-machine + backoff).
//! Each test is #[ignore]d until Plan 09-03 fills in the body.

#[tokio::test]
#[ignore = "Wave 0 — implemented in Plan 09-03"]
async fn pty_actor_enters_reconnecting_on_eof() {
    unimplemented!()
}

#[tokio::test]
#[ignore = "Wave 0 — implemented in Plan 09-03"]
async fn pty_actor_exponential_backoff_schedule() {
    unimplemented!()
}

#[tokio::test]
#[ignore = "Wave 0 — implemented in Plan 09-03"]
async fn pty_actor_cancels_backoff_on_pane_close() {
    unimplemented!()
}

#[tokio::test]
#[ignore = "Wave 0 — implemented in Plan 09-03"]
async fn reconnect_emits_pane_reconnecting_event() {
    unimplemented!()
}
