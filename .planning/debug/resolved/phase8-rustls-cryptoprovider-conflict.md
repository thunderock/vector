---
status: resolved
trigger: "Phase 8 execution introduced a cross-phase regression. Two Phase-6 tests in vector-codespaces fail with rustls CryptoProvider auto-select panic due to dual aws-lc-rs + ring providers in dep graph."
created: 2026-05-21T22:30:00Z
updated: 2026-05-22T00:30:00Z
---

## Current Focus

hypothesis: Dual russh (0.60 + Microsoft fork 0.37 via patch) pulls rustls 0.23 with both `aws-lc-rs` and `ring` provider features active, so rustls cannot auto-select a CryptoProvider at runtime. Both Phase-6 tests trigger TLS via reqwest/wiremock and panic on first TLS use.
test: Reproduce with `cargo test -p vector-codespaces --test auth_refresh`, then add `rustls::crypto::aws_lc_rs::default_provider().install_default()` once at test entry via OnceLock guard.
expecting: Both tests pass with provider installed; no other regressions because aws-lc-rs was the pre-Phase-8 default.
next_action: Reproduce failure, inspect CodespacesClient::new_for_test path, apply fix in tests/auth_refresh.rs.

## Symptoms

expected: Both auth_refresh tests pass (they were green before Phase 8 SDK added).
actual: Both panic at TLS init with "Could not automatically determine the process-level CryptoProvider from Rustls crate features."
errors: "Call CryptoProvider::install_default() before this point to select a provider manually, or make sure exactly one of the 'aws-lc-rs' and 'ring' features is enabled."
reproduction: `cargo test -p vector-codespaces --test auth_refresh`
started: After Phase 8 added `tunnels` git dep + russh patch override in workspace Cargo.toml.

## Evidence

- timestamp: 2026-05-21T22:30:00Z
  checked: workspace Cargo.toml
  found: `[patch.crates-io] russh = git microsoft/vscode-russh` — Phase 8 D-A2 unified russh under MS fork at 0.37 level. workspace reqwest uses `rustls-tls` feature (default-features=false, rustls-tls + json + http2). oauth2 uses rustls-tls.
  implication: When tunnels SDK + russh 0.37 fork's rustls deps mix with reqwest's rustls 0.23, both `aws-lc-rs` (from reqwest's default rustls-tls in 0.12) and `ring` (from russh/rustls) get activated. Cargo unifies rustls into one version, both features = ambiguous CryptoProvider.

## Eliminated
<!-- none yet -->

## Resolution

root_cause: [pending reproduction]
fix: [pending]
verification: [pending]
files_changed: []
