//! Shared test helpers for the `vector-codespaces` integration tests.
//!
//! Phase 8 pulled in a second `reqwest` (0.13) via the `tunnels` git dep that
//! activates rustls' `ring` provider, while reqwest 0.12 already activated
//! `aws-lc-rs`. With both features unified into one rustls 0.23, auto-select
//! panics on first TLS use. Install `aws-lc-rs` explicitly (the pre-Phase-8
//! default) before any test touches the network.

use std::sync::Once;

static INSTALL_CRYPTO: Once = Once::new();

pub fn ensure_crypto_provider() {
    INSTALL_CRYPTO.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}
