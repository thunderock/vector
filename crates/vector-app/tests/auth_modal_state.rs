//! Phase 6 / AUTH-01 — pure-Rust contract test that the UserEvent variants
//! the auth_actor + AuthDeviceFlowModal hand around are declared, and
//! AuthCancellation is Send + Sync (required for moving across the tokio task
//! boundary).

use vector_app::auth_actor::AuthCancellation;
use vector_app::UserEvent;

#[test]
fn user_event_phase6_variants_exist() {
    // Compile-time witness that the variants are declared with the expected
    // shape. The destructure-via-construct dance pins the field names so a
    // rename in lib.rs surfaces here before it surfaces in 06-06.
    let _ = UserEvent::AuthSignInRequested;
    let _ = UserEvent::AuthRequired;
    let _ = UserEvent::SignOut;
    let _ = UserEvent::OpenCodespacesPicker;
    let _ = UserEvent::AuthFailed { reason: "x".into() };
    let _ = UserEvent::AuthCompleted {
        user_login: "octocat".into(),
    };
    let _ = UserEvent::AuthDisplayCode {
        user_code: "WDJB-MJHT".into(),
        verification_uri: "https://github.com/login/device".into(),
        expires_at: std::time::SystemTime::now(),
        interval_secs: 5,
    };
}

#[test]
fn auth_cancellation_is_send_sync() {
    fn _check_send_sync<T: Send + Sync>() {}
    _check_send_sync::<AuthCancellation>();
}
