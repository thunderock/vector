---
phase: 09-persistence-reconnect-tmux-auto-attach
plan: 02
subsystem: tunnels
tags: [vector-mux, vector-tunnels, devtunnels, reconnect, persist, async-trait, wiremock, tokio-duplex]

requires:
  - phase: 09-persistence-reconnect-tmux-auto-attach/01
    provides: "Domain::reconnect_one_shot(rows, cols) -> Result<Option<Box<dyn PtyTransport>>> trait seam + LocalDomain Ok(None) + Wave-0 #[ignore]'d tunnels stubs"
  - phase: 08-vs-code-remote-tunnels-connect
    provides: "DevTunnelsApi, AuthProvider, TunnelRecord, connect_tunnel helper, DevTunnelTransport with `shell: None` OpenPty handshake"
provides:
  - "ReconnectableDevTunnelDomain — concrete `vector_mux::Domain` impl whose `reconnect_one_shot` re-runs `connect_tunnel` for a fresh `Box<dyn PtyTransport>`"
  - "`auth_factory: Arc<dyn Fn() -> AuthProvider + Send + Sync>` shape — picker actor wraps its `MicrosoftTokenStore` here so token refresh works between reconnect attempts"
  - "PERSIST-03 wire-format regression test: client always sends `OpenPty { shell: None, .. }` to the agent"
  - "Reconnect-path test: `reconnect_one_shot` reaches `connect_tunnel` and invokes `auth_factory` per attempt"
affects:
  - "09-03 (per-pane Reconnecting actor): consumes `Arc<dyn Domain>` constructed from this type"
  - "09-05 (picker-actor wire-up): constructs `ReconnectableDevTunnelDomain` after initial `connect_tunnel`, attaches via `Pane.domain`"

tech-stack:
  added: []
  patterns:
    - "Per-attempt auth refresh via `Arc<dyn Fn() -> AuthProvider + Send + Sync>` — keeps `vector-tunnels` free of `MicrosoftTokenStore` and lets the picker actor own the source of truth"
    - "Concrete-domain-lives-with-transport pattern: `vector-mux` declares the trait, `vector-tunnels` provides the reconnectable impl. Preserves WIN-04 (mux stays russh-free / tunnel-free)"
    - "Integration tests use `tokio::io::duplex` as the wire (already established in 08-04 `transport_protocol.rs`) — no real relay needed"

key-files:
  created:
    - "(none — both test files existed as Wave-0 stubs)"
  modified:
    - "crates/vector-tunnels/src/domain.rs — added `ReconnectableDevTunnelDomain` struct + `MuxDomain` impl beneath existing `connect_tunnel`"
    - "crates/vector-tunnels/src/lib.rs — re-export `ReconnectableDevTunnelDomain`"
    - "crates/vector-tunnels/tests/reconnect_one_shot.rs — real wiremock-backed tests replacing Wave-0 stub"
    - "crates/vector-tunnels/tests/open_pty_no_shell_override.rs — real `tokio::io::duplex` wire-format regression replacing Wave-0 stub"

key-decisions:
  - "Auth factory closure (not stored AuthProvider): rotation-safe. Each `reconnect_one_shot` call invokes the factory; the picker actor's `MicrosoftTokenStore` decides whether to silently refresh or return the cached token."
  - "Test B uses empty-endpoints `TunnelRecord` to short-circuit `DevTunnelTransport::connect` at the existing 'no endpoints' guard (transport.rs:205-208). Proves we reached `connect_tunnel` without needing a real Dev Tunnels relay."
  - "Manual `Debug` impl on `ReconnectableDevTunnelDomain` (Pitfall-14 discipline) — excludes the `Arc<dyn Fn>` (not Debug) and ensures only `tunnel_id` + `label` surface."

patterns-established:
  - "Reconnectable domain lives next to its transport, implements the mux trait from the other side of the seam"
  - "Wire-format regression tests use `tokio::io::duplex` + frame inspection, matching the established 08-04 protocol-test style"

requirements-completed: [PERSIST-02, PERSIST-03]

duration: 12min
completed: 2026-05-22
---

# Phase 9 Plan 02: ReconnectableDevTunnelDomain + PERSIST-03 wire-format regression Summary

**`ReconnectableDevTunnelDomain` lands in `vector-tunnels` as the concrete `vector_mux::Domain` whose `reconnect_one_shot(rows, cols)` re-runs `connect_tunnel` with a freshly-minted `AuthProvider` from a caller-supplied factory closure; PERSIST-03 is locked at the wire format via a `tokio::io::duplex` frame-inspection test asserting `OpenPty { shell: None, .. }`.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-05-22T19:55:00Z
- **Completed:** 2026-05-22T20:07:00Z
- **Tasks:** 2
- **Files modified:** 4 (domain.rs, lib.rs, 2 integration tests)

## Accomplishments

- `ReconnectableDevTunnelDomain` struct + `vector_mux::Domain` impl beneath the existing `connect_tunnel` helper in `crates/vector-tunnels/src/domain.rs`.
- `spawn` deliberately bails ("use reconnect_one_shot — picker actor owns initial connect") — the picker actor (Plan 09-05) calls `connect_tunnel` directly for the first transport; this domain only services reconnect.
- `reconnect_one_shot(rows, cols)` invokes `auth_factory` → `connect_tunnel(api, auth, tunnel, rows, cols)` → `Ok(Some(transport))`, propagating any underlying error transparently.
- Re-exported as `vector_tunnels::ReconnectableDevTunnelDomain` so Plan 09-05 can name the type at construction time.
- PERSIST-03 regression test (`open_pty_sends_no_shell_override`): drives `DevTunnelTransport::new_with_stream` against a `tokio::io::duplex` wire, captures the first frame, asserts `OpenPty { shell: None, rows: 30, cols: 100 }`. Any future patch that wraps the remote shell in tmux from the client side fails this test.
- Reconnect-path tests in `reconnect_one_shot.rs` (3 tests): one proves `connect_tunnel` is reached and the `auth_factory` fires; one proves the factory is invoked on every attempt (2 calls → counter = 2); one smokes `label()` + `is_alive()`.
- All Wave-0 `#[ignore]` annotations removed.

## Task Commits

1. **Task 1: Implement ReconnectableDevTunnelDomain** — `77c6978` (feat)
   - `crates/vector-tunnels/src/domain.rs` + `crates/vector-tunnels/src/lib.rs`
2. **Task 2: Fill in tunnels-side tests** — `5f2845a` (test)
   - `crates/vector-tunnels/tests/open_pty_no_shell_override.rs` + `crates/vector-tunnels/tests/reconnect_one_shot.rs`

## Files Created/Modified

- `crates/vector-tunnels/src/domain.rs` — Appended `ReconnectableDevTunnelDomain` struct (api + auth_factory + tunnel + label), `new()` ctor, manual `Debug` (no token surface), `async_trait` impl of `MuxDomain` (spawn-bail / `reconnect_one_shot` via `connect_tunnel`).
- `crates/vector-tunnels/src/lib.rs` — Added `ReconnectableDevTunnelDomain` to the `pub use domain::{...}` re-export.
- `crates/vector-tunnels/tests/open_pty_no_shell_override.rs` — Real `tokio::io::duplex` wire-driver test; captures first frame, decodes as `AgentMessage::OpenPty`, asserts `shell.is_none()` + `matches!(shell, None)` + dims.
- `crates/vector-tunnels/tests/reconnect_one_shot.rs` — wiremock-backed tests; `mock_token_server` returns a fake connect-scope token for `/api/v1/tunnels/t-fake/access`; `empty_endpoints_record` short-circuits `DevTunnelTransport::connect` at the no-endpoints guard so the test proves "we reached `connect_tunnel`" without a real relay; `AtomicUsize` in the factory closure proves per-attempt invocation.

## Decisions Made

- **Auth factory closure (not stored token):** Each `reconnect_one_shot` invocation re-queries the factory. This lets the picker actor's `MicrosoftTokenStore` silently refresh the upstream MS access token without `vector-tunnels` knowing about Keychain or refresh tokens. Matches the 08-CONTEXT pattern of keeping `vector-tunnels` free of platform-specific identity stores.
- **Empty-endpoints test fixture:** Reusing the existing `Protocol("tunnel has no endpoints")` guard in `transport.rs:205-208` gives a deterministic, fast assertion point that proves `connect_tunnel` was reached without standing up a real Dev Tunnels relay or a complex SDK mock. The 08-04 `connect_rejects_tunnel_with_no_endpoints` test already established this seam.
- **Manual `Debug` on the domain struct:** Pitfall-14 discipline. The `Arc<dyn Fn>` isn't `Debug`, and even if it were, we never want the closure to format under tracing. The manual impl prints only `tunnel_id` + `label`.

## Deviations from Plan

None — plan executed exactly as written. Plan called out two acceptable shapes for Test B (wiremock vs cfg-test trait-object seam) and recommended wiremock first; that's what landed. Plan's Test 3 ("passes dims to OpenPty via fake relay") is already covered by Test A in `open_pty_no_shell_override.rs` (rows=30, cols=100 round-trip through `new_with_stream`), so a separate dims-only test wasn't added — the wire-format test subsumes it.

## Issues Encountered

- Initial draft of `reconnect_one_shot_reaches_connect_tunnel` used `res.expect_err(...)` which requires `Ok: Debug`. `Box<dyn PtyTransport>` is intentionally not `Debug`. Replaced with an explicit `match` — trivial fix, no design impact.

## Verification

- `cargo build -p vector-tunnels` — OK (1.82s incremental).
- `cargo build -p vector-app` — OK (re-export doesn't break the picker actor, which is unchanged this plan).
- `cargo test -p vector-tunnels --lib` — 11 passed / 0 failed / 0 ignored.
- `cargo test -p vector-tunnels --test open_pty_no_shell_override` — 1 passed / 0 failed / 0 ignored.
- `cargo test -p vector-tunnels --test reconnect_one_shot` — 3 passed / 0 failed / 0 ignored.
- `grep -RIn "tmux new -A" crates/vector-mux crates/vector-app crates/vector-tunnels` — 0 matches (PERSIST-03 absence assertion complementing the wire-format test).
- Acceptance greps for `pub struct ReconnectableDevTunnelDomain`, `impl MuxDomain for ReconnectableDevTunnelDomain`, and the `lib.rs` re-export all return matches.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 09-03 (per-pane Reconnecting actor): has the `Arc<dyn Domain>` shape it needs. Construct via `Arc::new(ReconnectableDevTunnelDomain::new(api, auth_factory, tunnel, label))` and pass into the actor spawn signature.
- Plan 09-05 (picker-actor wire-up): construction site is the only one — wrap the existing `MicrosoftTokenStore` in an `auth_factory` closure (`Arc::new(move || AuthProvider::Microsoft(store.get_access_token_blocking()))` or equivalent async-friendly shape if the store exposes it).
- PERSIST-03 is now defended at two levels: (a) `transport.rs:88` hard-codes `shell: None` (production), (b) the wire-format test fails any patch that changes that line.

## Self-Check

- File `/Users/ashutosh/personal/vector/crates/vector-tunnels/src/domain.rs`: FOUND
- File `/Users/ashutosh/personal/vector/crates/vector-tunnels/tests/reconnect_one_shot.rs`: FOUND
- File `/Users/ashutosh/personal/vector/crates/vector-tunnels/tests/open_pty_no_shell_override.rs`: FOUND
- Commit `77c6978`: FOUND
- Commit `5f2845a`: FOUND

## Self-Check: PASSED

---
*Phase: 09-persistence-reconnect-tmux-auto-attach*
*Plan: 02*
*Completed: 2026-05-22*
