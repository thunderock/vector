---
phase: 09-persistence-reconnect-tmux-auto-attach
plan: 01
subsystem: infra
tags: [reconnect, domain-trait, user-event, wave-0, scaffolding]

# Dependency graph
requires:
  - phase: 04-mux-tabs-splits
    provides: Domain trait + LocalDomain + DevTunnelDomain stub (D-38 transport surface)
  - phase: 08-vs-code-remote-tunnels-connect
    provides: DevTunnelTransport + DevTunnelDomain stub in vector-tunnels
provides:
  - "Domain::reconnect_one_shot(rows, cols) -> Result<Option<Box<dyn PtyTransport>>> trait method"
  - "LocalDomain::reconnect_one_shot returns Ok(None) (permanent — local PTY death irrecoverable)"
  - "DevTunnelDomain::reconnect_one_shot stub panic forwarding to Plan 09-02"
  - "UserEvent::PaneReconnecting { pane_id, attempt, profile_label } variant"
  - "UserEvent::PaneReconnected { pane_id } variant"
  - "7 Wave-0 test files scaffolded (all #[ignore]d, total 17 tests + 3 active trait-shape tests)"
affects: [09-02, 09-03, 09-04, 09-05, 09-06]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Domain::reconnect_one_shot trait method seam: returns Ok(None) for permanent failure, Some(transport) for hot-swap"
    - "Append-only UserEvent variants with Phase 9 marker comment (never reorder convention preserved)"
    - "Wave-0 stub pattern: #[ignore = \"Wave 0 — implemented in Plan 09-XX\"] for forward-referenced test files"

key-files:
  created:
    - crates/vector-mux/tests/reconnect_trait.rs
    - crates/vector-app/tests/pty_actor_reconnect.rs
    - crates/vector-app/tests/reconnect_byte_integrity.rs
    - crates/vector-app/tests/reconnect_pass_render.rs
    - crates/vector-tunnels/tests/reconnect_one_shot.rs
    - crates/vector-tunnels/tests/open_pty_no_shell_override.rs
    - crates/vector-tunnels/tests/live_devtunnel_smoke.rs
    - .planning/phases/09-persistence-reconnect-tmux-auto-attach/09-01-SUMMARY.md
  modified:
    - crates/vector-mux/src/domain.rs
    - crates/vector-mux/src/local_domain.rs
    - crates/vector-mux/src/devtunnel_domain.rs
    - crates/vector-app/src/lib.rs
    - crates/vector-app/src/app.rs

key-decisions:
  - "Domain::reconnect_one_shot signature returns Result<Option<Box<dyn PtyTransport>>>: Ok(None) is the LocalDomain permanent-failure sentinel; Some(transport) is the hot-swap value the per-pane I/O actor consumes in Plan 09-03"
  - "Add no-op match arm for PaneReconnecting/PaneReconnected in app.rs:user_event NOW (Plan 01) rather than waiting for Plan 09-05 — preserves exhaustive-match compile while consumers are still under construction"

patterns-established:
  - "Wave-0 test stub: #[tokio::test] (or #[test]) with #[ignore = \"Wave 0 — implemented in Plan 09-XX\"] and unimplemented!() body keeps `cargo test --workspace` green while pinning the file/test layout downstream plans must match"
  - "UserEvent Phase-9 block lives strictly AFTER OpenDevTunnelsPickerMenu; future Phase 9 plans append within this block, never re-order existing variants"

requirements-completed: [PERSIST-01, PERSIST-02]

# Metrics
duration: 5min
completed: 2026-05-22
---

# Phase 09 Plan 01: Reconnect Trait + Event Surface Foundation Summary

**Extended `Domain` trait with `reconnect_one_shot(rows, cols)`, gave `LocalDomain` a permanent `Ok(None)` no-op, parked `DevTunnelDomain` behind a Phase-9-Plan-02 forward-reference panic, appended `UserEvent::PaneReconnecting`/`PaneReconnected` variants, and scaffolded 7 Wave-0 test files (17 ignored stubs + 3 active trait-shape tests).**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-05-22T19:54:43Z
- **Completed:** 2026-05-22T19:59:28Z
- **Tasks:** 3
- **Files modified:** 5 + 7 created = 12

## Accomplishments

- `Domain::reconnect_one_shot(&self, rows: u16, cols: u16) -> Result<Option<Box<dyn PtyTransport>>>` replaces the prior `async fn reconnect() -> Result<()>` placeholder; trait remains object-safe (`Arc<dyn Domain>` compiles).
- `LocalDomain::reconnect_one_shot` returns `Ok(None)` — local PTY death is permanent by design; Plan 09-03's actor will treat `None` as the "drop the pane" signal.
- `DevTunnelDomain::reconnect_one_shot` panics with `"Phase 9 Plan 02: ReconnectableDevTunnelDomain in crates/vector-tunnels/src/domain.rs"` so anyone calling it pre-Plan-02 gets a crisp forward reference.
- `UserEvent` carries two new appended variants (`PaneReconnecting { pane_id, attempt, profile_label }`, `PaneReconnected { pane_id }`) — sites that EMIT them land in Plan 09-03; sites that CONSUME them (overlay + tab badge + input lock) land in Plan 09-05.
- 7 Wave-0 test files now exist at every path Plans 09-02..06 will modify, all tests `#[ignore]`d with implementation pointers; `cargo test --workspace --no-run` is green, 17 `#[ignore = "Wave 0` annotations across the two crates.

## Task Commits

1. **Task 1: Extend Domain trait + LocalDomain no-op + DevTunnelDomain unimplemented stub** — `51ce13d` (feat)
2. **Task 2: Append PaneReconnecting + PaneReconnected UserEvent variants** — `17b3e83` (feat)
3. **Task 3: Scaffold all Wave-0 test files (compile-only, no implementation)** — `b94e6ae` (test)

## Files Created/Modified

- `crates/vector-mux/src/domain.rs` — Replaced `reconnect()` with `reconnect_one_shot(rows, cols)`.
- `crates/vector-mux/src/local_domain.rs` — `LocalDomain::reconnect_one_shot` returns `Ok(None)`.
- `crates/vector-mux/src/devtunnel_domain.rs` — `DevTunnelDomain::reconnect_one_shot` panics with Phase-9-Plan-02 marker.
- `crates/vector-mux/tests/reconnect_trait.rs` — 3 trait-shape tests (object-safety, LocalDomain Ok(None), DevTunnelDomain should_panic).
- `crates/vector-app/src/lib.rs` — Appended `PaneReconnecting` + `PaneReconnected` variants inside Phase 9 marker block.
- `crates/vector-app/src/app.rs` — Added no-op `UserEvent::PaneReconnecting { .. } | UserEvent::PaneReconnected { .. } => {}` match arm to keep exhaustive matching green until Plan 09-05 wires real consumers.
- `crates/vector-app/tests/pty_actor_reconnect.rs` — 4 ignored stubs for Plan 09-03 state-machine tests.
- `crates/vector-app/tests/reconnect_byte_integrity.rs` — 2 ignored stubs for Plan 09-03 zero-byte-loss tests.
- `crates/vector-app/tests/reconnect_pass_render.rs` — 6 ignored stubs (4 for Plan 09-04 UI; 2 for Plan 09-05 input-lock + tab-badge).
- `crates/vector-tunnels/tests/reconnect_one_shot.rs` — 1 ignored stub for Plan 09-02 DevTunnel reconnect.
- `crates/vector-tunnels/tests/open_pty_no_shell_override.rs` — 1 ignored stub for Plan 09-02 PERSIST-03 regression guard.
- `crates/vector-tunnels/tests/live_devtunnel_smoke.rs` — 3 ignored stubs for Plan 09-06 live e2e (gated on `VECTOR_E2E_TUNNEL_ID`).

## Decisions Made

- **`Ok(None)` as the LocalDomain sentinel:** Splits "transport hot-swap succeeded" (`Ok(Some(t))`) from "this domain will never give you a new transport, give up" (`Ok(None)`) from "tried and failed; back off and retry" (`Err(_)`). Plan 09-03's state machine maps `None` → terminal `Closed` state, `Err` → `Reconnecting` with backoff increment.
- **`unimplemented!` not `todo!` on DevTunnelDomain:** `unimplemented!` carries a forward-reference message that survives `cargo test --workspace` (the panic only fires inside the should_panic test; the variant is never reachable from production code because Plan 09-02 will install a separate `ReconnectableDevTunnelDomain` impl rather than wiring through the bare `DevTunnelDomain`).
- **Pre-emptively add the no-op `app.rs` match arm here (Rule 3):** The plan said consumers land in Plan 09-05 but appending the variants in Plan 01 broke the exhaustive `user_event` match. Two options: (a) leave the build broken between Plan 01 and 09-05, (b) add a no-op arm now. Picked (b) — Wave-0 plans must keep the workspace green per `cargo test --workspace --no-run` acceptance criterion.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added no-op match arm for new UserEvent variants in app.rs:user_event**
- **Found during:** Task 2 (appending UserEvent variants)
- **Issue:** Adding `PaneReconnecting` and `PaneReconnected` to the enum broke the exhaustive `match event` in `crates/vector-app/src/app.rs:1647` (compile error E0004: non-exhaustive patterns).
- **Fix:** Appended `UserEvent::PaneReconnecting { .. } | UserEvent::PaneReconnected { .. } => {}` no-op arm right after the existing `OpenDevTunnelsPickerMenu` arm, with a Phase-9 marker comment pointing at Plan 09-05 for real consumer wiring.
- **Files modified:** `crates/vector-app/src/app.rs`
- **Verification:** `cargo build -p vector-app` and `cargo clippy -p vector-app -- -D warnings` both pass.
- **Committed in:** `17b3e83` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Minimal — the no-op arm is the smallest possible additive change that preserves the workspace-green invariant. Real PaneReconnecting/PaneReconnected handlers still land in Plan 09-05 as planned; this just keeps the build compiling in the interim.

## Issues Encountered

- **Acceptance criteria wording mismatch:** The plan's Task 3 acceptance criteria say e.g. "Running `cargo test -p vector-app --test pty_actor_reconnect` reports `4 passed; 0 failed; 4 ignored`". With unconditional `#[ignore]` annotations, `cargo test` (no `--ignored`) actually reports `0 passed; 0 failed; 4 ignored`. The substantive intent — "every test compiles AND none run by default" — is met; the literal numeric expectation in the plan was just authoring shorthand. No code change needed.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Plan 09-02 (ReconnectableDevTunnelDomain) is unblocked:** `Domain::reconnect_one_shot(rows, cols)` is the signature it implements; `crates/vector-tunnels/tests/reconnect_one_shot.rs` and `open_pty_no_shell_override.rs` are pre-scaffolded.
- **Plan 09-03 (per-pane actor + backoff) is unblocked:** Both `UserEvent::PaneReconnecting` and `UserEvent::PaneReconnected` exist; `crates/vector-app/tests/pty_actor_reconnect.rs` and `reconnect_byte_integrity.rs` are pre-scaffolded.
- **Plan 09-04 (status-line overlay) is unblocked:** `crates/vector-app/tests/reconnect_pass_render.rs` carries 4 of its 6 ignored stubs.
- **Plan 09-05 (input lock + tab badge) is unblocked:** The no-op match arm added in this plan is the exact place 09-05 will replace with real consumer logic; the remaining 2 of 6 stubs in `reconnect_pass_render.rs` are already named for it.
- **Plan 09-06 (live e2e) is unblocked:** `crates/vector-tunnels/tests/live_devtunnel_smoke.rs` defines the three test function names + the `VECTOR_E2E_TUNNEL_ID` env-var gate.

## Self-Check: PASSED

Verified:
- `crates/vector-mux/src/domain.rs` exists; line 45 has `async fn reconnect_one_shot`.
- `crates/vector-mux/src/local_domain.rs` exists; line 112 has `Ok(None)` body.
- `crates/vector-mux/src/devtunnel_domain.rs` exists; line 47 has Phase-9-Plan-02 panic message.
- `crates/vector-app/src/lib.rs` exists; lines 167/174 declare PaneReconnecting/PaneReconnected.
- `crates/vector-app/src/app.rs` exists; carries no-op match arm for Phase 9 variants.
- All 7 created test files exist and compile.
- Commits `51ce13d`, `17b3e83`, `b94e6ae` all reachable via `git log --oneline -5`.
- `cargo test -p vector-mux --test reconnect_trait` reports 3 passed, 0 failed.

---
*Phase: 09-persistence-reconnect-tmux-auto-attach*
*Completed: 2026-05-22*
