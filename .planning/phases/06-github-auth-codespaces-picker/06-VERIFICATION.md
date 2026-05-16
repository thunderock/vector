---
phase: 06-github-auth-codespaces-picker
verified: 2026-05-14T20:30:00Z
status: human_needed
score: 6/6 must-haves wired (automated); 0/11 smoke matrix items executed (deferred — 06-07 autonomous=false)
re_verification: null
human_verification:
  - test: "Item 1 — Sign in with GitHub (AUTH-01)"
    expected: "AuthDeviceFlowModal opens 440x280 floating, 8-char user-code in large mono font, countdown ticks, pbpaste returns user-code, primary opens github.com/device in Safari, after auth the modal auto-dismisses within ~5s, toast `signed in as @{login}` appears, menu flips to `Sign out (@{login})`, pbpaste returns the PRE-modal clipboard (Pitfall 7 restore)."
    why_human: "Requires real GitHub OAuth, real Safari browser, real Keychain, real AppKit modal interaction, real clipboard, visual + temporal confirmation."
  - test: "Item 2 — Token persisted in Keychain (AUTH-02)"
    expected: "`security find-generic-password -s vector -a github_oauth_token -w` returns a `gho_*` or `ghu_*` token. `grep -r 'gho_' ~/Library/Logs/` returns 0. `grep -r 'gho_' ./target/` returns 0."
    why_human: "Requires real Keychain entry from real OAuth flow; requires user-session log directory the verifier cannot access."
  - test: "Item 3 — Token survives app restart (AUTH-02)"
    expected: "After Cmd-Q and relaunch, `Vector` menu shows `Sign out (@{login})` on launch with no re-auth prompt."
    why_human: "Requires real app process lifecycle + real Keychain read on cold start."
  - test: "Item 4 — Sign out clears Keychain"
    expected: "Click `Sign out` → toast `signed out` → menu reverts to `Sign in with GitHub`. `security find-generic-password -s vector -a github_oauth_token -w` returns `... could not be found.`"
    why_human: "Requires real Keychain delete via menu action."
  - test: "Item 5 — Codespaces picker via menu + keyboard (CS-01)"
    expected: "Cmd-Shift-G opens 640px NSPanel titled `Codespaces` with brief `loading codespaces…` then rows showing state / repo / branch / last-used (e.g. `2 hours ago`). Footer reads `{N} codespaces · last refreshed just now`. Re-open via `Vector → Codespaces…` menu identical."
    why_human: "Requires real GitHub Codespaces list, real network fetch, visual NSPanel layout confirmation."
  - test: "Item 6 — Connect button placeholder toast (CS-04 deferred to Phase 7)"
    expected: "Arrow to Available row + Enter → toast `codespace ssh transport not yet wired — phase 7`. Modal stays open."
    why_human: "Requires real codespace row + visual toast confirmation."
  - test: "Item 7 — Start a Shutdown codespace (CS-02)"
    expected: "Select Shutdown row + Enter → toast `starting codespace…` → state label flips to `Starting` within ~5s → eventually flips to `Available` within 2min."
    why_human: "Requires real Shutdown codespace + real /start API + real 1Hz poll over 30-120s."
  - test: "Item 8 — 409 swallow when codespace already starting (Pitfall 5)"
    expected: "Click `Start` while codespace is Starting → NO `could not start` toast; polling continues uninterrupted."
    why_human: "Requires real 409 response from GitHub during a real Starting transition."
  - test: "Item 9 — Save as profile (CS-03)"
    expected: "Cmd-S on selected row → toast `profile saved as \"{derived-name}\"`. `~/.config/vector/config.toml` contains `[profile.{derived-name}]` with `kind = \"codespace\"`, `codespace_name = \"...\"`, `tint = \"#7a3aaf\"`. Pre-existing blocks/comments preserved. Cmd-Shift-P picker shows the new profile not dimmed."
    why_human: "Requires real disk write + visual inspection of TOML formatting + real profile picker rerender."
  - test: "Item 10 — Saved profile survives app restart (CS-03)"
    expected: "Cmd-Q + relaunch + Cmd-Shift-P → saved profile still visible."
    why_human: "Requires real app restart cycle."
  - test: "Item 11 — 401 silent refresh / re-auth (AUTH-03)"
    expected: "Delete only the access token from Keychain. Cmd-Shift-G → either picker loads silently (refresh-token chain succeeded) OR AuthDeviceFlowModal opens automatically. NO empty list with no explanation. NO silent failure."
    why_human: "Requires real Keychain manipulation + real 401 from GitHub + real re-auth flow."
---

# Phase 6: GitHub Auth + Codespaces Picker Verification Report

**Phase Goal:** A user can sign into GitHub from inside Vector and see a list of their Codespaces with state, repo, branch, and last-used time — no SSH transport yet.

**Verified:** 2026-05-14T20:30:00Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Executive Summary

Plans 06-01 through 06-06 are complete on disk: vector-codespaces crate (auth + REST + 401-refresh) is wired, vector-config writer ships profile-save + atomic rename, vector-app menu items + AuthDeviceFlowModal + CodespacesPickerModal + tokio actors + Cmd-Shift-G keymap all wired end-to-end. Pitfall-14 arch-lint is enforcing. Token-leak audit returns 0 hits across `*.rs`. Full workspace test suite is green (363 passed / 0 failed / 5 ignored — UAT placeholders).

Plan 06-07 is intentionally deferred: it is an `autonomous: false` 11-item manual smoke matrix that must be driven by a real human against real GitHub + real Codespaces + real Safari + real Keychain. The matrix is the close-gate for all 5 ROADMAP §"Phase 6" success criteria. Verifier returns `human_needed` rather than `passed` until the matrix lands.

## Goal Achievement — Observable Truths (Success Criteria)

| #   | Truth (ROADMAP)   | Status     | Automated Evidence | Human Gap |
| --- | ----------------- | ---------- | ------------------ | --------- |
| 1   | Device Flow + user-code shown + token in Keychain; no `gho_` in disk/logs | ✓ VERIFIED (automated) / ? UNCERTAIN (UI) | 4/4 device_flow tests green (wiremock); TokenStore over `vector_secrets::Secrets`; arch-lint blocks `#[derive(Debug)]` near tokens (2 passed). `grep -rn 'gho_' --include='*.rs' \| grep -v test` = 0 hits. AuthDeviceFlowModal NSPanel matches UI-SPEC §5.1 in code. | Items 1 + 2: real OAuth + real Keychain confirmation. |
| 2   | Picker lists codespaces with state/repo/branch/last-used; refreshes on state change | ✓ VERIFIED (automated) / ? UNCERTAIN (UI) | 8/8 codespaces_rest tests green; `relative_time::humanize` 7/7 tests; fixture exercises 5 rows incl. Hibernated → Unrecognized variant; CodespacesPickerModal LoadState + rerender wired; per-row poll tasks emit CodespaceStateChanged. | Item 5: real GitHub Codespaces visible in NSPanel. |
| 3   | Shutdown → POST /start + swallow 409 + 1s/2min poll until Available | ✓ VERIFIED (automated) / ? UNCERTAIN (timing) | `CodespacesClient::start` returns Ok on 200/202/409 (Pitfall 5); `poll_until_available` uses tokio::select! with CancellationToken; 8/8 codespaces_rest tests cover poll terminates, 120s timeout, cancellation. `spawn_start_then_poll` wired in codespaces_actor + app.rs. | Items 7 + 8: real Shutdown→Available transition + real 409 swallow. |
| 4   | Saved codespace profile survives restart; Connect shows placeholder toast | ✓ VERIFIED (automated) / ? UNCERTAIN (restart) | 6/6 profile_writer tests green; `append_codespace_profile` atomic-rename verified (no `.tmp` after success); placeholder toast literal `codespace ssh transport not yet wired — phase 7` present in app.rs:472. | Items 9 + 10: real disk write + real restart cycle confirming Cmd-Shift-P picks up profile. |
| 5   | 401 silent-refresh chain (transient invisible; refresh-fail → re-auth prompt) | ✓ VERIFIED (automated) / ? UNCERTAIN (network) | 2/2 auth_refresh tests green (401→refresh→200 OK; refresh-401 → Unauthenticated). AuthRequired UserEvent wired through `spawn_fetch_codespaces` + `build_client_from_keychain`; app.rs:1625 routes AuthRequired → device-flow modal. | Item 11: real 401 from GitHub + real refresh-token chain. |

**Score:** 5/5 truths verified at the automated/code level; 5/5 require human UAT confirmation for end-to-end goal achievement.

## Required Artifacts (Levels 1-3: exists, substantive, wired)

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `crates/vector-codespaces/src/auth/device_flow.rs` | GitHubAuth driver + Tokens + DEFAULT_CLIENT_ID consts | ✓ VERIFIED | request_device_code + poll_for_token + refresh_access_token; 4/4 tests green; manual Debug per Pitfall-14. |
| `crates/vector-codespaces/src/auth/token_store.rs` | Keychain save/load/clear over `vector_secrets::Secrets` | ✓ VERIFIED | save_access / save_refresh / load_access / load_refresh / clear wired; manual Debug. |
| `crates/vector-codespaces/src/auth/error.rs` | AuthError thiserror enum | ✓ VERIFIED | OAuth/Http/Secrets/Url/Cancelled/Expired/NoRefreshToken variants. |
| `crates/vector-codespaces/src/client/mod.rs` | CodespacesClient + build_octocrab + RefreshContext | ✓ VERIFIED | list/get/start/poll/list_with_refresh; Arc<RwLock<Arc<Octocrab>>> for Pitfall 2; 10/10 tests green. |
| `crates/vector-codespaces/src/model.rs` | Codespace + CodespaceState (Unrecognized) + RepositoryRef + GitStatus | ✓ VERIFIED | #[serde(other)] Unrecognized + #[serde(flatten)] _rest (Pitfall 4). |
| `crates/vector-config/src/writer.rs` | append_codespace_profile + derive_profile_name + WriterError | ✓ VERIFIED | toml_edit round-trip + atomic rename + regex `-[a-z0-9]{4,}$` suffix strip + collision auto-suffix; 6/6 tests green. |
| `crates/vector-app/src/auth_actor.rs` | spawn_device_flow + AuthCancellation + fetch_login | ✓ VERIFIED | Drives device-flow state machine on tokio runtime; emits AuthDisplayCode / AuthCompleted / AuthFailed. |
| `crates/vector-app/src/auth_modal.rs` | AuthDeviceFlowModal NSPanel + AuthModalResponder | ✓ VERIFIED | 440x280 Titled+Closable, NSFloatingWindowLevel, 32pt JetBrains Mono semibold code, clipboard save/restore (Pitfall 7), define_class! responder pattern. |
| `crates/vector-app/src/codespaces_actor.rs` | spawn_fetch_codespaces + spawn_poll_row + spawn_start_then_poll + build_client_from_keychain | ✓ VERIFIED | 4 fns confirmed via grep; list_with_refresh call site present. |
| `crates/vector-app/src/codespaces_modal.rs` | CodespacesPickerModal NSPanel + LoadState + config_path | ✓ VERIFIED | 640x480 Titled+Closable + NSFloatingWindowLevel, LoadState enum, poll_cancel CancellationToken, is_key_window helper. |
| `crates/vector-app/src/relative_time.rs` | humanize + state_label + state_color | ✓ VERIFIED | 60 LoC pure-Rust; 7/7 tests green covering boundaries 59s/60s/3599s/3600s/year. |
| `crates/vector-app/src/menu.rs` (Auth section) | install_auth_menu_items + rebuild_auth_menu_section + AuthMenuTarget | ✓ VERIFIED | 3 items inserted at indices 0..3 of Vector menu; Sign in / Sign out (hidden) / Codespaces… with Cmd-Shift-G; rebuild toggles visibility + title. |
| `crates/vector-input/src/keymap.rs` (Cmd-Shift-G) | AppShortcut::OpenCodespacesPicker + SignInWithGitHub | ✓ VERIFIED | Cmd-Shift-G match arm + dispatch into UserEvent::OpenCodespacesPicker. |
| `crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs` | Pitfall-14 arch-lint (Debug + tracing macros) | ✓ VERIFIED | 2/2 tests passing. |

## Key Link Verification (wiring)

| From | To  | Via | Status | Details |
| ---- | --- | --- | ------ | ------- |
| Vector menu → `Sign in with GitHub` | AuthDeviceFlowModal | AuthMenuTarget → UserEvent::AuthSignInRequested → app.rs handler → auth_actor::spawn_device_flow | ✓ WIRED | install_auth_menu_items in menu.rs; app.rs handler calls spawn_device_flow with proxy + tokio_handle. |
| Cmd-Shift-G keymap | CodespacesPickerModal | AppShortcut::OpenCodespacesPicker → UserEvent::OpenCodespacesPicker → app.rs::handle_open_codespaces_picker → build_client_from_keychain → spawn_fetch_codespaces | ✓ WIRED | Keymap arm in vector-input; app.rs dispatch confirmed; CodespacesPickerModal::show called on key Cmd-Shift-G. |
| Vector menu → `Codespaces…` | Same handler | AuthMenuTarget → UserEvent::OpenCodespacesPicker | ✓ WIRED | Menu item points at the same UserEvent the keymap dispatches. |
| Auth modal primary button | Browser + UserEvent | AuthModalResponder::primaryClicked → re-copy + NSWorkspace open URL | ✓ WIRED | define_class! responder routes setAction; primary opens github.com/device. |
| 401 from CodespacesClient::list_with_refresh | Re-auth | ClientError::Unauthenticated → spawn_fetch_codespaces emits UserEvent::AuthRequired → app.rs:1625 routes to handle_auth_sign_in_requested | ✓ WIRED | AuthRequired arm present at app.rs:1625; refresh chain in client/mod.rs. |
| No token at picker open | Re-auth | TokenStore::load_access None → build_client_from_keychain None → AuthRequired UserEvent | ✓ WIRED | Lazy client construction in codespaces_actor::build_client_from_keychain. |
| ProfileSelected (Codespace kind, no token) | Re-auth (D-84) | app.rs ProfileSelected guard → AuthSignInRequested | ✓ WIRED | D-84 single-chokepoint guard in app.rs ProfileSelected handler. |
| Cmd-S in picker | TOML write | codespaces_save_selected → derive_profile_name + append_codespace_profile → toast `profile saved as "{name}"` | ✓ WIRED | app.rs:524 toast literal confirmed; vector-config writer::append_codespace_profile called. |
| Enter on Available row | Phase-7 placeholder toast | codespaces_connect_selected → ToastBanner::info(`codespace ssh transport not yet wired — phase 7`) | ✓ WIRED | Exact literal at app.rs:472. |
| Enter on Shutdown row | Start + poll | codespaces_start_selected → codespaces_actor::spawn_start_then_poll → CodespacesClient::start (409 swallow) → poll_until_available → CodespaceStateChanged events | ✓ WIRED | spawn_start_then_poll grep-confirmed; defensive 409 arm in actor + Pitfall-5 swallow in client. |

## Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| CodespacesPickerModal | LoadState::Ready(Arc<Vec<Codespace>>) | spawn_fetch_codespaces → CodespacesClient::list_with_refresh → GET /user/codespaces → octocrab body_to_string + serde_json::from_str | ✓ FLOWING (against real GitHub once token present) / wiremock-verified for fixture | ✓ FLOWING |
| AuthDeviceFlowModal | user_code, verification_uri, expires_at, interval_secs | spawn_device_flow → GitHubAuth::request_device_code → POST github.com/login/device/code → oauth2::DeviceAuthorizationResponse | ✓ FLOWING (real GitHub once OAuth App registered; falls back to gh CLI client ID 178c6fc778ccc68e1d6a today) | ✓ FLOWING |
| Toast `signed in as @{login}` | user_login | spawn_device_flow → fetch_login → vector_codespaces::build_octocrab + octocrab.current().user() | ✓ FLOWING | ✓ FLOWING |
| Toast `profile saved as "{name}"` | final_name | append_codespace_profile returns String (collision-resolved name) | ✓ FLOWING (toml_edit round-trip + atomic rename) | ✓ FLOWING |

No HOLLOW/STATIC/DISCONNECTED artifacts detected.

## Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| Workspace tests pass | `cargo test --workspace --tests` | 363 passed / 0 failed / 5 ignored | ✓ PASS |
| Token-leak audit (gho_) | `grep -rn 'gho_' . --include='*.rs' \| grep -v test \| grep -v gho_test` | 0 hits | ✓ PASS |
| Token-leak audit (ghu_) | `grep -rn 'ghu_' . --include='*.rs' \| grep -v test` | 0 hits | ✓ PASS |
| Token-leak audit (ghp_) | `grep -rn 'ghp_' . --include='*.rs' \| grep -v test` | 0 hits | ✓ PASS |
| Pitfall-14 arch-lint | `cargo test -p vector-arch-tests --test no_token_in_debug_or_log` | 2 passed / 0 failed | ✓ PASS |
| Device-flow tests | `cargo test -p vector-codespaces --test device_flow` | 4 passed / 0 failed | ✓ PASS |
| REST + start/poll tests | `cargo test -p vector-codespaces --test codespaces_rest` | 8 passed / 0 failed | ✓ PASS |
| 401-refresh chain tests | `cargo test -p vector-codespaces --test auth_refresh` | 2 passed / 0 failed | ✓ PASS |
| Profile writer tests | `cargo test -p vector-config --test profile_writer` | 6 passed / 0 failed | ✓ PASS |
| relative_time tests | `cargo test -p vector-app --test relative_time` | 7 passed / 0 failed | ✓ PASS |
| AuthModal contract | `cargo test -p vector-app --test auth_modal_state` | 2 passed / 0 failed | ✓ PASS |
| Live OAuth Device Flow against real GitHub | Item 1 of smoke matrix | not run (autonomous=false) | ? SKIP |
| Real Keychain entry post sign-in | Item 2 of smoke matrix | not run | ? SKIP |
| Real Shutdown→Available transition | Item 7 of smoke matrix | not run | ? SKIP |
| Real 409 swallow on already-Starting codespace | Item 8 of smoke matrix | not run | ? SKIP |
| Real disk write + TOML preservation | Item 9 of smoke matrix | not run | ? SKIP |
| Real restart-cycle profile persistence | Item 10 of smoke matrix | not run | ? SKIP |
| Real 401 silent-refresh / re-auth | Item 11 of smoke matrix | not run | ? SKIP |

## Requirements Coverage

| Requirement | Source Plan(s) | Description (REQUIREMENTS.md) | Status | Evidence |
| ----------- | -------------- | ----------------------------- | ------ | -------- |
| AUTH-01 | 06-01, 06-02, 06-05, 06-07 | OAuth Device Flow (RFC 8628) sign-in from inside the app | ✓ SATISFIED (automated) / ? NEEDS HUMAN | device_flow.rs + AuthDeviceFlowModal wired; 4/4 device-flow tests green; arch-lint clean. Real-GitHub UAT = Items 1, 11. |
| AUTH-02 | 06-01, 06-02, 06-07 | Tokens stored in macOS Keychain via keyring 4.0; never on disk; never logged | ✓ SATISFIED (automated) / ? NEEDS HUMAN | TokenStore over `vector_secrets::Secrets`; 1 ignored keychain_roundtrip UAT; Pitfall-14 arch-lint passing; `grep -rn 'gho_' --include='*.rs'` = 0. Real-Keychain UAT = Items 2, 3, 4. |
| AUTH-03 | 06-01, 06-03, 06-05, 06-07 | Silent refresh on 401; expired tokens trigger re-auth prompt, not silent failure | ✓ SATISFIED (automated) / ? NEEDS HUMAN | 2/2 auth_refresh tests (401→refresh→200, refresh-401 → Unauthenticated); AuthRequired → device-flow modal wired in app.rs:1625. Real-network UAT = Item 11. |
| CS-01 | 06-01, 06-03, 06-06, 06-07 | Codespaces picker lists state/repo/branch/last-used; refreshes on state change | ✓ SATISFIED (automated) / ? NEEDS HUMAN | 8/8 codespaces_rest + 7/7 relative_time tests; CodespacesPickerModal LoadState + per-row poll tasks. Real-list UAT = Item 5. |
| CS-02 | 06-01, 06-03, 06-06, 06-07 | Shutdown → POST /start, swallow 409, poll 1s/120s until Available | ✓ SATISFIED (automated) / ? NEEDS HUMAN | Pitfall-5 swallow in client.rs; spawn_start_then_poll + defensive 409 arm; poll cancellation test green. Real-start UAT = Items 7, 8. |
| CS-03 | 06-01, 06-04, 06-06, 06-07 | Codespace saved as profile that survives restart; Connect shows Phase-7 placeholder | ✓ SATISFIED (automated) / ? NEEDS HUMAN | 6/6 profile_writer tests; atomic-rename verified; placeholder literal `codespace ssh transport not yet wired — phase 7` at app.rs:472. Real-disk + restart UAT = Items 9, 10. |

Every requirement ID declared in any plan frontmatter is accounted for in REQUIREMENTS.md (lines 56-61 v1 checklist; lines 183-188 traceability table). REQUIREMENTS.md already flips these to `[x] Complete` — pre-emptive of the smoke matrix landing. Confirm 06-07 smoke matrix verdict before treating Complete as load-bearing.

No orphaned requirements detected.

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| (none) | — | — | — | Pitfall-14 arch-lint passing; `gho_/ghu_/ghp_` audits return 0; no TODO/FIXME/placeholder strings in Phase-6 source paths beyond the explicit `codespace ssh transport not yet wired — phase 7` toast (which IS the spec — UI-SPEC §6.1 Phase-7 placeholder). |

## Human Verification Required (11-item smoke matrix from 06-07-PLAN.md)

The 11 items in the `human_verification` frontmatter block above mirror 06-07-PLAN.md verbatim. Run them in order after wiping any existing Keychain entries:

```bash
security delete-generic-password -s vector -a github_oauth_token 2>/dev/null
security delete-generic-password -s vector -a github_refresh_token 2>/dev/null
cargo build -p vector-app --release && ./target/release/vector-app
```

Capture verdicts per the 06-07-PLAN.md verdict block:

```
1: PASS|FAIL|SKIP — notes
2: PASS|FAIL|SKIP — notes
3: PASS|FAIL|SKIP — notes
4: PASS|FAIL|SKIP — notes
5: PASS|FAIL|SKIP — notes
6: PASS|FAIL|SKIP — notes
7: PASS|FAIL|SKIP — notes
8: PASS|FAIL|SKIP — notes
9: PASS|FAIL|SKIP — notes
10: PASS|FAIL|SKIP — notes
11: PASS|FAIL|SKIP — notes
Overall: N/11 PASS — approved | gaps_found
```

A `gaps_found` outcome routes to `/gsd:plan-phase 6 --gaps`; an `approved` outcome lets Phase 6 close and unblocks Phase 7.

## Gaps Summary

No automated gaps. Phase-6 implementation is goal-complete at the code/wiring level: every artifact exists, every key link is wired, every data source flows real data, and every requirement has automated evidence (unit + integration tests covering happy path + error path + edge case). Pitfall-14 arch-lint enforces token discipline; token-leak audit confirms zero hits.

The only outstanding work is **Plan 06-07's 11-item human smoke matrix** — explicitly `autonomous: false` and deferred by the user to be driven via `/gsd:verify-work 6`. Five of the phase's success criteria (per ROADMAP) cannot be confirmed without real GitHub OAuth + real Codespaces + real macOS Keychain + real Safari + real NSPanel interaction. Until those 11 items return PASS verdicts, status stays `human_needed` rather than `passed`.

---

_Verified: 2026-05-14T20:30:00Z_
_Verifier: Claude (gsd-verifier)_
