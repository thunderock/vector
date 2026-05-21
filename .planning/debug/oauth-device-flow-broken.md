---
status: awaiting_human_verify
trigger: "GitHub OAuth device flow has two problems: overlay disappears too fast, polling fails with 'Failed to parse server response'"
created: 2026-05-18T00:00:00Z
updated: 2026-05-18T00:00:00Z
---

## Current Focus

hypothesis: CONFIRMED — Bug 1 = missing Accept: application/json on reqwest client; Bug 2 = handle_auth_failed unconditionally dismisses modal.
test: Apply fixes and rebuild
expecting: cargo build passes
next_action: Edit device_flow.rs and app.rs

## Symptoms

expected: Overlay shows device code and persists until user dismisses/auth completes; poll succeeds and token stored
actual: Overlay vanishes ~immediately; poll fails with "oauth: oauth2 error: Failed to parse server response" ~360ms after init
errors:
  - auth_failed reason="oauth: oauth2 error: Failed to parse server response"
reproduction: Click "Sign in to GitHub" in Vector
started: Current state on phase4 branch

## Eliminated

## Evidence

- checked: crates/vector-codespaces/src/auth/device_flow.rs reqwest::Client::builder()
  found: No default_headers set. GitHub /login/oauth/access_token returns application/x-www-form-urlencoded unless Accept: application/json is sent. oauth2 5.x BasicTokenResponse parses JSON only.
  implication: Bug 1 root cause confirmed.

- checked: crates/vector-app/src/app.rs handle_auth_failed
  found: Unconditionally takes & dismisses self.auth_modal on every failure reason. This fires when polling fails (e.g. parse error before user enters code) → modal disappears.
  implication: Bug 2 root cause confirmed. Even after Bug 1 is fixed, the modal must survive transient/init failures up to the point user has entered code.

- checked: auth_actor.rs run_flow
  found: AuthFailed is also emitted on "cancelled" and "expired" — those SHOULD dismiss the modal. Need to differentiate.
  implication: Fix must dismiss for cancelled/expired but NOT for transient oauth errors that occurred before/during polling.

## Resolution

root_cause:
  bug1: reqwest::Client used by oauth2 device flow is missing Accept: application/json default header; GitHub responds with form-urlencoded which oauth2 5.x cannot parse.
  bug2: handle_auth_failed always dismisses the modal — even when failure happens before the user could read the code (e.g. immediate parse error).
fix:
  bug1: Configure reqwest::Client with default Accept: application/json header in GitHubAuth::new_with_endpoints.
  bug2: In handle_auth_failed, only dismiss the modal on user-driven terminal states (cancelled/expired) or after the user has had a chance to act. Keep modal visible on transient oauth errors so the user can still see/copy the code; offer them to retry/cancel.
verification:
  - cargo build --release -p vector-codespaces → OK
  - cargo build --release -p vector-app → OK
  - cargo fmt --all -- --check → OK
  - cargo clippy -p vector-codespaces -p vector-app --all-targets -- -D warnings → no warnings
  - cargo test -p vector-codespaces --tests → all pass (8 unit + 4 device_flow integration)
files_changed:
  - crates/vector-codespaces/src/auth/device_flow.rs (added Accept: application/json default header)
  - crates/vector-app/src/app.rs (handle_auth_failed only dismisses modal on cancelled/expired)
