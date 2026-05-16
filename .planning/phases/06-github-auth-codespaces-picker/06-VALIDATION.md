---
phase: 6
slug: github-auth-codespaces-picker
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-14
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust unit + integration) |
| **Config file** | `Cargo.toml` workspace — `[dev-dependencies]` per crate |
| **Quick run command** | `cargo test -p vector-codespaces 2>&1 \| tail -5` |
| **Full suite command** | `cargo test --workspace 2>&1 \| tail -20` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p vector-codespaces 2>&1 | tail -5`
- **After every plan wave:** Run `cargo test --workspace 2>&1 | tail -20`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 6-01-01 | 01 | 0 | AUTH-01 | unit | `cargo test -p vector-codespaces token_store` | ❌ W0 | ⬜ pending |
| 6-01-02 | 01 | 0 | AUTH-01 | unit | `cargo test -p vector-codespaces device_flow` | ❌ W0 | ⬜ pending |
| 6-01-03 | 01 | 0 | AUTH-03 | unit | `cargo test -p vector-codespaces token_refresh` | ❌ W0 | ⬜ pending |
| 6-02-01 | 02 | 1 | CS-01 | unit | `cargo test -p vector-codespaces client::list` | ❌ W0 | ⬜ pending |
| 6-02-02 | 02 | 1 | CS-02 | unit | `cargo test -p vector-codespaces client::start` | ❌ W0 | ⬜ pending |
| 6-02-03 | 02 | 1 | CS-02 | unit | `cargo test -p vector-codespaces client::poll` | ❌ W0 | ⬜ pending |
| 6-03-01 | 03 | 1 | CS-03 | unit | `cargo test -p vector-config codespace_profile` | ❌ W0 | ⬜ pending |
| 6-03-02 | 03 | 1 | AUTH-01 | arch-lint | `grep -r 'gho_' . --include='*.rs' \| grep -v test` | ✅ | ⬜ pending |
| 6-04-01 | 04 | 2 | AUTH-01 | manual | See Manual-Only Verifications | n/a | ⬜ pending |
| 6-04-02 | 04 | 2 | AUTH-02 | manual | See Manual-Only Verifications | n/a | ⬜ pending |
| 6-05-01 | 05 | 2 | CS-01 | manual | See Manual-Only Verifications | n/a | ⬜ pending |
| 6-05-02 | 05 | 2 | CS-02 | manual | See Manual-Only Verifications | n/a | ⬜ pending |
| 6-05-03 | 05 | 2 | CS-03 | manual | See Manual-Only Verifications | n/a | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/vector-codespaces/src/lib.rs` — crate scaffold
- [ ] `crates/vector-codespaces/Cargo.toml` — deps: `oauth2 5.0`, `octocrab 0.50`, `reqwest 0.13` (rustls), `keyring-core 1.0`, `tokio`, `serde`, `thiserror`, `zeroize`
- [ ] `crates/vector-codespaces/src/auth/mod.rs` — `GitHubAuth` struct stub
- [ ] `crates/vector-codespaces/src/auth/token_store.rs` — `TokenStore` wrapping `vector-secrets` Keychain
- [ ] `crates/vector-codespaces/src/auth/device_flow.rs` — device-flow state machine stub
- [ ] `crates/vector-codespaces/src/client/mod.rs` — `CodespacesClient` stub
- [ ] `crates/vector-codespaces/tests/` — wiremock fixture directory
- [ ] `dev-dependencies`: `wiremock 0.6`, `tokio-test`, `serde_json`
- [ ] `crates/vector-codespaces/src/auth/mod.rs` — manual `Debug` impl (no `#[derive(Debug)]` on token structs)
- [ ] Workspace `Cargo.toml` — `vector-codespaces` member added
- [ ] `grep -r 'gho_' . --include='*.rs' | grep -v test` returns zero hits (arch-lint baseline)
- [ ] `cargo test -p vector-codespaces 2>&1 | grep "0 failed"` passes with stub tests

*Wave 0 must complete before any Wave 1/2 plan tasks execute.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| OAuth device flow shows user-code in UI modal | AUTH-01 | Requires real GitHub account + browser | Click "Sign in with GitHub", verify code displayed, visit URL, authorize; check Keychain: `security find-generic-password -s vector -a github_oauth_token` |
| Token persists across app restart | AUTH-01 | Requires app restart cycle | Quit and relaunch Vector; menu should show "Signed in as {username}" without re-auth prompt |
| 401 triggers re-auth prompt (not silent fail) | AUTH-03 | Requires token expiry simulation | Manually corrupt stored token in Keychain; next API call should trigger re-auth prompt, not a silent error |
| Codespaces list shows state/repo/branch/last-used | CS-01 | Requires real Codespaces | Sign in; open picker; verify all 4 fields visible for each codespace |
| Shutdown → start poll shows progress → Available | CS-02 | Requires a shutdown Codespace | Select shutdown codespace; confirm start; observe poll progress up to 2 min |
| Profile save survives restart | CS-03 | Requires restart cycle | Save codespace as profile; quit; relaunch; verify profile appears in picker |
| `grep -r 'gho_' . --include='*.log'` returns 0 | AUTH-01 | Token-leak security check | Run after any app session that signed in; check logs directory |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
