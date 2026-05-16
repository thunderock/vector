---
phase: 06-github-auth-codespaces-picker
plan: 03
subsystem: codespaces-rest
tags: [octocrab, reqwest, wiremock, parking-lot, cs-01, cs-02, auth-03, pitfall-2, pitfall-4, pitfall-5, tdd]

requires:
  - phase: 06-github-auth-codespaces-picker
    plan: 01
    provides: "vector-codespaces crate surface (CodespacesClient over Arc<Octocrab>, ClientError, Codespace + CodespaceState with #[serde(other)] Unrecognized, manual-Debug discipline) + 9 #[ignore]-gated test stubs across codespaces_rest.rs + auth_refresh.rs + 1-row JSON fixture"
provides:
  - "CodespacesClient::{new, new_for_test, list, get, start, poll_until_available, list_with_refresh} — full CS-01 (list) + CS-02 (start/poll) + AUTH-03 (401-refresh chain) surface"
  - "build_octocrab(token, base_uri_opt) -> Arc<Octocrab> with Vector User-Agent + Accept headers; reused by app startup once Plan 06-05 wires the session"
  - "Pitfall 2 mitigation in production code: Arc<RwLock<Arc<Octocrab>>> internally — token refresh swaps the inner Arc without stranding in-flight clones"
  - "Pitfall 5 mitigation: POST /start returns Ok(()) on 200/202/409 (409 = already starting); only other 4xx/5xx surface as ClientError::StartFailed { status }"
  - "Inline RefreshContext over reqwest::Client — minimal OAuth refresh-token POST to a configurable token endpoint (intentionally decoupled from 06-02's GitHubAuth so this plan can land in parallel; production wiring lands in 06-05)"
  - "10 wiremock-scripted tests green: 8 REST (list, Unrecognized, start swallowing 200/202/409, start fails 500, poll terminates Available, poll 120s timeout, poll Cancellation), 2 refresh (401→refresh→200 success, 401→refresh-401 → Unauthenticated)"
affects: [06-05-auth-modal, 06-06-codespaces-modal]

tech-stack:
  added:
    - "parking_lot.workspace (already at workspace pin 0.12) — RwLock around inner Arc<Octocrab> for Pitfall 2"
  patterns:
    - "Octocrab raw _get/_post + body_to_string() — octocrab 0.50 has no typed Codespaces; we parse the response body via serde_json::from_str. _get/_post return http::Response<BoxBody<Bytes, octocrab::Error>>, not reqwest::Response (corrected from plan)"
    - "Inline OAuth refresh-token grant via reqwest — decoupled from GitHubAuth surface so REST plan can ship without blocking on device-flow plan; future wiring is a one-line constructor swap"
    - "tokio::select! over CancellationToken::cancelled() + tokio::time::sleep(1s) for cooperative cancellation inside poll_until_available"
    - "Tokio test-util pause: #[tokio::test(start_paused = true)] + tokio::time::advance(125s) drives the deadline test; in practice wiremock real-time I/O keeps the test in real wall-clock (~120s), but the assertion correctness is unaffected"

key-files:
  created: []
  modified:
    - "crates/vector-codespaces/src/client/mod.rs (rewrote — full client implementation + RefreshContext)"
    - "crates/vector-codespaces/src/lib.rs (added build_octocrab to client re-exports)"
    - "crates/vector-codespaces/Cargo.toml (added parking_lot.workspace dep)"
    - "crates/vector-codespaces/tests/codespaces_rest.rs (expanded from 7 ignored stubs to 8 live tests)"
    - "crates/vector-codespaces/tests/auth_refresh.rs (expanded from 2 ignored stubs to 2 live tests)"
    - "crates/vector-codespaces/tests/fixtures/list_codespaces.json (expanded from 1 row to 5 rows incl. Hibernated for Pitfall-4)"

key-decisions:
  - "RefreshContext inline (not via GitHubAuth from 06-02): plan called for `GitHubAuth::new_with_endpoints(...)` + `refresh_access_token(...)` on the test seam. Because 06-02 was executing in parallel and its surface wasn't yet observable, we implemented the refresh via reqwest::Client::post(token_url).form([(grant_type, refresh_token), ...]). This costs ~30 LoC and avoids cross-plan coupling. Plan 06-05 can replace RefreshContext with a wrapper around GitHubAuth in one constructor when the auth modal is wired."
  - "octocrab _get/_post return type is http::Response<BoxBody<Bytes, octocrab::Error>>, not reqwest::Response (plan was wrong). Use octo.body_to_string(resp) helper to get the body as String, then serde_json::from_str. status() works the same way."
  - "Pitfall 4 fixture row: anon-repo-r3a4b with state=\"Hibernated\" — GitHub has no such state in our enum. #[serde(other)] Unrecognized variant catches it; future GitHub-side additions never crash deserialize."
  - "Poll cancellation uses tokio::select! with cancel.cancelled() polled in the loop's wait branch — cancellation is observed within the 1s sleep window, not blocked on the next HTTP request."
  - "new_for_test takes base_uri as &str (not String) to satisfy clippy needless_pass_by_value; tests pass `&server.uri()`."

patterns-established:
  - "Test-seam constructor pattern: CodespacesClient::new (prod, takes Arc<Octocrab>) + new_for_test (wires inline refresh against a wiremock token URL). Future client plans should follow this split so tests don't have to mock the entire auth chain."
  - "Inline-then-replace decoupling: when a parallel-wave plan depends on a sibling plan's surface, ship an inline minimal implementation and document the constructor swap. Lower lock-step risk than waiting for the sibling."

requirements-completed: [CS-01, CS-02, AUTH-03]

duration: 8min
completed: 2026-05-14
---

# Phase 6 Plan 03: Wave 1 — CodespacesClient REST (list/get/start/poll) + 401 silent-refresh chain Summary

**Implemented `CodespacesClient` over `Arc<Octocrab>` with raw `_get`/`_post` for CS-01 (list with state/repo/branch), CS-02 (start treating 200/202/409 as success + 1s poll with 120s deadline and cooperative cancellation), and AUTH-03 (401 → silent refresh via inline reqwest POST → retry once; still-401 emits `ClientError::Unauthenticated`). 10/10 wiremock tests green; Pitfall-2/4/5/14 lints all clean.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-05-14T19:09:50Z
- **Completed:** 2026-05-14T19:18:01Z
- **Tasks:** 2 (RED + GREEN)
- **Files modified:** 6 (0 created, 6 modified — all scaffolded by Plan 06-01)

## Accomplishments

- `CodespacesClient::list` parses 5-row `total_count + codespaces[]` response into `Vec<Codespace>`; verified against an expanded fixture covering Available/Starting/Shutdown/Failed/Hibernated states.
- `CodespacesClient::get` percent-encodes the codespace name into `/user/codespaces/{name}`; 401 is surfaced as `ClientError::Unauthenticated` (drives the `list_with_refresh` retry chain).
- `CodespacesClient::start` posts to `/user/codespaces/{name}/start`; 200/202/409 all return `Ok(())` per Pitfall 5; 4xx/5xx otherwise return `ClientError::StartFailed { status }`.
- `CodespacesClient::poll_until_available` runs a `tokio::select!` loop over `CancellationToken::cancelled()` + `sleep(1s)`, calling `get(name)` each iteration and terminating on `Available`/`Failed`/`Shutdown` (or `PollTimeout` / `Cancelled`).
- `CodespacesClient::list_with_refresh` wraps `list()` in the AUTH-03 chain: on 401, drive the inline `RefreshContext::refresh()` (reqwest POST with `grant_type=refresh_token`) against the configured token URL; on success, swap the inner `Arc<Octocrab>` under `RwLock` and retry `list()` once. Refresh failure or still-401 returns `ClientError::Unauthenticated`.
- `build_octocrab(token, base_uri_opt)` re-exported from the crate root for production wiring once Plan 06-05 lands.
- `Pitfall 2` mitigated structurally: `inner: Arc<RwLock<Arc<Octocrab>>>` and `fn octo(&self) -> Arc<Octocrab> { self.inner.read().clone() }` — every call site grabs a fresh clone on entry.
- `Pitfall 14` arch-lint (`vector-arch-tests::no_token_in_debug_or_log`) still passes — `RefreshContext` has a manual `Debug` impl that omits `refresh_token`.

## Task Commits

1. **Task 06-03-01: RED — wiremock-scripted failing tests** — `d6db52f` (test)
2. **Task 06-03-02: GREEN — CodespacesClient + 401 refresh chain** — `0cbaeb7` (feat)

**Plan metadata commit:** (to follow — SUMMARY/STATE/ROADMAP/REQUIREMENTS)

## Files Modified

- `crates/vector-codespaces/src/client/mod.rs` — replaced `unimplemented!()` stubs with 220-LoC implementation (`build_octocrab`, `RefreshContext`, full `CodespacesClient` surface).
- `crates/vector-codespaces/src/lib.rs` — added `build_octocrab` to client re-exports.
- `crates/vector-codespaces/Cargo.toml` — added `parking_lot.workspace = true` to `[dependencies]`.
- `crates/vector-codespaces/tests/codespaces_rest.rs` — un-ignored 7 stubs into 8 live tests (added `poll_cancellation_token`).
- `crates/vector-codespaces/tests/auth_refresh.rs` — un-ignored both stubs into live tests.
- `crates/vector-codespaces/tests/fixtures/list_codespaces.json` — expanded from 1 row to 5 rows (added Starting/Shutdown/Failed/Hibernated).

## Decisions Made

- **Inline `RefreshContext` (deviation from plan's `GitHubAuth::new_with_endpoints` call):** Plan called for `RefreshContext { auth: GitHubAuth::new_with_endpoints(...), ... }` and `auth.refresh_access_token(&ctx.refresh_token).await`. At plan-start, 06-02 (parallel wave) had not yet exposed those constructors. To honor the parallel-execution invariant ("your files do NOT overlap with 06-02"), the REST client owns its own `reqwest::Client` and POSTs the OAuth refresh-token grant directly. Verified the GitHub OAuth refresh contract: `POST {token_url}` with `grant_type=refresh_token` + `refresh_token={token}` form body, response `{ access_token, token_type, scope }`. Cost: ~30 LoC of focused refresh code; gain: zero coupling on 06-02's still-in-flight surface. Plan 06-05 can swap the constructor when assembling the production client.
- **Octocrab `_get`/`_post` body extraction (deviation from plan's `resp.bytes()`):** Plan specified `let bytes = resp.bytes().await?;`. The actual return type in octocrab 0.50 is `http::Response<BoxBody<Bytes, octocrab::Error>>`, which has no `bytes()` method (it's not a reqwest Response). Switched to `let body = octo.body_to_string(resp).await?;` + `serde_json::from_str(&body)`. Same semantics, correct types.
- **Tokio test-util pause behavior:** `#[tokio::test(start_paused = true)]` + `tokio::time::advance(Duration::from_secs(125)).await` drives the 120s-timeout test. Because wiremock holds a real socket and its responses involve real reactor I/O, the test runs in approximate wall-clock (~120s) rather than virtual time. The assertion (`Err(ClientError::PollTimeout)`) is still correctness-verified. We accept the slow test; addressing it would require either swapping wiremock for a pure in-memory transport (not justified for one test) or rewriting `poll_until_available` to be transport-injectable (architectural change, out of scope).
- **New `poll_cancellation_token` test:** Plan listed 8 REST tests but the original stub file had only 7. The 8th test (`poll_cancellation_token`) is in the plan's `<behavior>` block — added as a fresh test, not un-ignored from an existing stub. Confirms the `CancellationToken::cancel()` path returns `Err(ClientError::Cancelled)` mid-poll.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] octocrab `_get`/`_post` return `http::Response<BoxBody<Bytes, Error>>`, not `reqwest::Response`**
- **Found during:** Task 06-03-02 first `cargo test` (10 compile errors all variants of `no method named bytes`).
- **Issue:** Plan specified `let bytes = resp.bytes().await?;`. octocrab 0.50's actual signature returns the http-crate Response type with a `BoxBody` body, which has no `bytes()`/`json()`.
- **Fix:** Used `octo.body_to_string(resp).await?` (octocrab's own helper) + `serde_json::from_str(&body)`. Status checks are identical (`resp.status().as_u16()`).
- **Verification:** All 10 tests pass.
- **Committed in:** `0cbaeb7`

**2. [Rule 3 — Blocking] `GitHubAuth::new_with_endpoints` did not exist when this plan started executing**
- **Found during:** Task 06-03-02 design.
- **Issue:** Plan called `GitHubAuth::new_with_endpoints(refresh_endpoint, refresh_endpoint, "Iv1.test_client_id")` inside `new_for_test`. 06-02 was executing in parallel; its API was not yet visible.
- **Fix:** Implemented the refresh-token POST inline using `reqwest::Client::post(token_url).form([("grant_type", "refresh_token"), ("refresh_token", t)]).send()` inside a private `RefreshContext` struct (with manual `Debug` per Pitfall 14).
- **Forward path:** Plan 06-05 will introduce a `CodespacesClient::new_with_refresh(octocrab, GitHubAuth, refresh_token, base_uri)` constructor that wraps the post-06-02 `GitHubAuth::refresh_access_token`. The test seam stays as-is.
- **Verification:** Both AUTH-03 tests green; structure honours the plan's success criteria ("401 → refresh → retry succeeds" and "refresh fails → ClientError::Unauthenticated").
- **Committed in:** `0cbaeb7`

**3. [Rule 1 — Bug] Clippy: `match` with one `Ok` arm could be `let...else`**
- **Found during:** Task 06-03-02 `cargo clippy -- -D warnings`.
- **Issue:** `let new_access = match ctx.refresh().await { Ok(t) => t, Err(_) => { ...; return Err(...); } };` tripped `clippy::single_match_else`.
- **Fix:** Rewrote as `let Ok(new_access) = ctx.refresh().await else { ...; return Err(...); };`.
- **Committed in:** `0cbaeb7`

**4. [Rule 1 — Bug] Clippy: redundant `continue` at end of loop body**
- **Found during:** Task 06-03-02 clippy.
- **Issue:** `match cs.state { ... terminal arms ... return Ok(cs.state), _ => continue }` had a useless `continue`.
- **Fix:** Rewrote as `if matches!(cs.state, Available | Failed | Shutdown) { return Ok(cs.state); }`.
- **Committed in:** `0cbaeb7`

**5. [Rule 1 — Bug] Clippy: `new_for_test` took `String` by value but only used it as `&str`**
- **Found during:** Task 06-03-02 clippy.
- **Issue:** `base_uri: String` was passed-by-value but only borrowed downstream.
- **Fix:** Changed signature to `base_uri: &str`; updated test call sites to `&server.uri()`.
- **Committed in:** `0cbaeb7`

---

**Total deviations:** 5 auto-fixed (2 Rule-3 blocking, 3 Rule-1 lint cleanups). Two were unavoidable plan-vs-reality gaps (octocrab API + parallel-plan coupling); three were clippy hygiene. No architectural deviation; no scope creep.

## Issues Encountered

The 120s poll-timeout test runs in real wall-clock (~121s) rather than the virtual-time auto-advance the `start_paused` attribute implies. Root cause: wiremock holds real sockets, so tokio's reactor cannot fully suspend time. The test still asserts the correct error variant; the cost is CI test latency. **Not blocking. Documented as a known slow test.** Mitigation deferred — see "Future improvements".

## User Setup Required

None. CodespacesClient is wholly internal; production wiring against a real GitHub bearer token happens in Plan 06-05 (auth modal) and 06-06 (codespaces picker modal).

## Future Improvements

- **Replace `RefreshContext` with `GitHubAuth` wrapper in Plan 06-05.** Now that 06-02 has landed (`Tokens`, `GitHubAuth::refresh_access_token`), the production constructor in 06-05 should call those instead of re-implementing the refresh-token POST. Existing tests stay as-is via `new_for_test`.
- **Make the slow `poll_times_out_at_120s` test fast.** Two options:
  - Pass a `clock_source: fn() -> Instant` into `poll_until_available` (test injects a virtual clock).
  - Or move `poll_until_available` to a transport-agnostic helper that takes a `Fn(&str) -> Future<Codespace>` and drop wiremock from this specific test.
  - Defer until the test starts blocking PR cycle time.

## Next Phase Readiness

- **Plan 06-05 (auth modal):** Has both `GitHubAuth` (from 06-02) and `CodespacesClient::build_octocrab` (from this plan). The full chain — device-flow → save tokens → build Octocrab → instantiate CodespacesClient — can be assembled in the modal's tokio actor.
- **Plan 06-06 (codespaces picker modal):** All four UI-facing entrypoints (`list_with_refresh`, `start`, `poll_until_available`, `get`) are live. Wire them inside the picker's `UserEvent` handlers per RESEARCH §"Pattern 3: Picker Open → Fetch → Poll Transitions".
- **Pitfall 14 arch-lint** stays green after this plan — `RefreshContext` was added with a manual `Debug` impl that omits the refresh token.

## Self-Check: PASSED

Verified each modified file + commit on disk:

- `crates/vector-codespaces/src/client/mod.rs` — FOUND (220 LoC, contains `build_octocrab`, `RefreshContext`, `CodespacesClient` with all required methods)
- `crates/vector-codespaces/src/lib.rs` — FOUND (`pub use client::{build_octocrab, ClientError, CodespacesClient};`)
- `crates/vector-codespaces/Cargo.toml` — FOUND (`parking_lot.workspace = true`)
- `crates/vector-codespaces/tests/codespaces_rest.rs` — FOUND (8 `#[tokio::test]`, 0 `#[ignore]`)
- `crates/vector-codespaces/tests/auth_refresh.rs` — FOUND (2 `#[tokio::test]`, 0 `#[ignore]`)
- `crates/vector-codespaces/tests/fixtures/list_codespaces.json` — FOUND (`total_count: 5`, includes `"state": "Hibernated"`)
- Commit `d6db52f` (RED) — FOUND in `git log`
- Commit `0cbaeb7` (GREEN) — FOUND in `git log`

Test runs verified:
- `cargo test -p vector-codespaces --test codespaces_rest --test auth_refresh` → `8 passed; 0 failed` + `2 passed; 0 failed` (10/10)
- `cargo test -p vector-arch-tests --test no_token_in_debug_or_log` → `2 passed; 0 failed`
- `cargo clippy -p vector-codespaces --all-targets -- -D warnings` → exit 0

---
*Phase: 06-github-auth-codespaces-picker*
*Completed: 2026-05-14*
