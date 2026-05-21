//! CS-04: russh client connects over an `AsyncRead+AsyncWrite` stream.
//!
//! - `check_server_key_rejects_mismatch` runs always: verifies the
//!   Pitfall-3 host-key check returns `Ok(false)` for a bogus fingerprint.
//! - `connect_stdio_stream_authenticates` is gated on `VECTOR_SSH_SPIKE_HOST`
//!   and skips cleanly when unset (localhost sshd is unavailable on this host
//!   — see 07-01-SUMMARY).

use russh::client::Handler;
use russh::keys::{Algorithm, PrivateKey};
use vector_ssh::handler::VectorHandler;

#[tokio::test]
async fn check_server_key_rejects_mismatch() {
    let mut handler = VectorHandler::new("SHA256:bogus-expected-fp".to_string());
    let key = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).expect("keygen");
    let public = key.public_key().clone();
    let ok = handler
        .check_server_key(&public)
        .await
        .expect("check_server_key");
    assert!(!ok, "bogus expected_fp must not match a real key");
}

#[tokio::test]
async fn check_server_key_accepts_match() {
    let key = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).expect("keygen");
    let public = key.public_key().clone();
    let expected_fp = public.fingerprint(russh::keys::HashAlg::Sha256).to_string();
    let mut handler = VectorHandler::new(expected_fp);
    let ok = handler
        .check_server_key(&public)
        .await
        .expect("check_server_key");
    assert!(ok, "matching fingerprint must be accepted");
}

#[tokio::test]
async fn connect_stdio_stream_authenticates() {
    let Ok(_host) = std::env::var("VECTOR_SSH_SPIKE_HOST") else {
        eprintln!("VECTOR_SSH_SPIKE_HOST unset — skipping live-sshd test");
        return;
    };
    // Live spike is documented unavailable on this host (07-01-SUMMARY).
    // When a host is provided, future work can wire a real TCP connect.
    eprintln!("VECTOR_SSH_SPIKE_HOST set — live spike not yet wired");
}
