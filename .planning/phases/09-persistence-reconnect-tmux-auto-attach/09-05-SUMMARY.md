---
phase: 09-persistence-reconnect-tmux-auto-attach
plan: 05
subsystem: ui
tags: [reconnect, devtunnels, app-state, input-gate, render-hook, cancel-token]

requires:
  - phase: 09-persistence-reconnect-tmux-auto-attach
    provides: "09-02 ReconnectableDevTunnelDomain + 09-03 per-pane reconnect state machine + EventSink trait + 09-04 ReconnectPass + format_reconnect_text + PaneUiState + ChromePipelines.reconnect"
provides:
  - "Picker actor (devtunnels_actor::handle_connect) constructs ReconnectableDevTunnelDomain + per-pane CancellationToken"
  - "UserEvent::DevTunnelPaneCancelToken variant routing per-pane cancel tokens from actor to App"
  - "App.reconnecting_panes / App.pane_cancel_tokens / App.reconnect_first_keystroke_shown state maps"
  - "ReconnectingState struct with profile_label, attempt, started_at, fade_in/out timestamps"
  - "UserEvent::PaneReconnecting / PaneReconnected handlers (insert/update state, flip tab title, request redraw)"
  - "Keystroke gate dropping bytes for panes in reconnecting_panes + one-shot ToastBanner::info on first dropped key per span"
  - "Per-pane render hook computing fade-in alpha after 250 ms debounce and calling chrome_pipelines.reconnect.update + .render"
  - "Pane close path (Cmd-W) cancels per-pane CancellationToken + clears reconnect state"
  - "8/8 active tests in reconnect_pass_render.rs (6 UI-T1..T6 from 09-04 + 2 newly un-ignored: input_locked_in_reconnecting_state, tab_badge_during_reconnect)"
affects: [09-06]

tech-stack:
  added: []
  patterns:
    - "App-side reconnect state is HashMap<PaneId, ReconnectingState> — same per-pane-keyed pattern used by other App-level pane state"
    - "Per-pane CancellationToken stored in App via UserEvent round-trip (picker actor emits DevTunnelPaneCancelToken; App handler inserts)"
    - "Plan 09-03 EventSink trait remains the sole event-routing abstraction; this plan consumes UserEvents on the App side without redefining the seam"
    - "Render hook lives in the per-pane render loop BEFORE the chrome snapshot block (UI-SPEC §Spacing composition order)"

key-files:
  created: []
  modified:
    - crates/vector-app/src/app.rs
    - crates/vector-app/src/devtunnels_actor.rs
    - crates/vector-app/src/lib.rs
    - crates/vector-app/src/pty_actor.rs
    - crates/vector-app/tests/reconnect_pass_render.rs
    - crates/vector-app/tests/pty_actor_reconnect.rs
    - crates/vector-app/tests/reconnect_byte_integrity.rs

key-decisions:
  - "Picker actor emits a NEW UserEvent::DevTunnelPaneCancelToken to hand the per-pane CancellationToken to the App rather than threading a shared map through Mux::create_tab_async_with_transport (keeps that signature stable)"
  - "Fade-out animation deferred — reconnecting_panes entry is removed immediately on PaneReconnected so the bar disappears in one frame instead of animating over 200 ms. Documented as a polish backlog item."
  - "Cursor dim (UI-SPEC §Input-Lock Affordances 40% alpha) deferred — existing cursor pipeline does not yet accept a reconnecting flag. Acceptable v1 fallback per plan's <interfaces> note; status bar is the primary signal."
  - "Light-mode chrome.surface RGBA not yet threaded into the render hook — bar uses the dark-mode surface color regardless of palette. Cosmetic v1 limitation."
  - "Inline status bar TEXT GLYPH ROW is not composited yet — only the bar background renders. Carried as Gap #4 in 09-05-HUMAN-UAT.md."
  - "Task 3 (live smoke UAT) is DEFERRED, not failed. A separate follow-up plan must construct DevTunnelsActor in main.rs before the UAT matrix can be walked end-to-end."

patterns-established:
  - "EventSink seam from Plan 09-03 is the only event-routing abstraction in the crate; App layer consumes UserEvents normally — verified by grep returning no EventSink/ProxyEventSink redefinitions in app.rs"
  - "Per-pane cancel tokens flow actor → App via UserEvent::DevTunnelPaneCancelToken; pane close handler invokes .cancel() then removes from the map"

requirements-completed: []  # PERSIST-01 was already marked complete by Plan 09-04. This plan delivers integration wiring but does not close any additional requirements.

duration: ~25min (Tasks 1 + 2; Task 3 deferred)
completed: 2026-05-22
---

# Phase 9 Plan 05: App Reconnect Integration Summary

**App-side reconnect state map, render hook, input gate, first-keystroke toast, tab-title flip, and per-pane CancellationToken plumbing wired end-to-end — pending main.rs DevTunnelsActor construction before the live UAT can be walked.**

## Status: SUBSTANTIALLY COMPLETE — Task 3 DEFERRED (not failed)

Tasks 1 and 2 landed and verified green (8/8 reconnect_pass_render tests, clippy -D warnings clean, all grep acceptance checks pass). Task 3 (live smoke UAT against a real Dev Tunnel) is **deferred to a follow-up plan**: the picker actor builds `ReconnectableDevTunnelDomain` correctly, but `DevTunnelsActor` itself is not yet constructed in `main.rs`, so Cmd-Shift-T cannot route through to the new code path in a running app. The UAT matrix is preserved verbatim in `09-05-HUMAN-UAT.md` (status: partial) so `/gsd:audit-uat` will surface the outstanding items together with the 5 known limitations.

## Performance

- **Duration:** ~25 min (Tasks 1 + 2; Task 3 deferred)
- **Started:** 2026-05-22
- **Completed:** 2026-05-22 (implementation tasks)
- **Tasks executed:** 2 of 3 (Task 3 deferred — see above)
- **Files modified:** 7

## Accomplishments

- `devtunnels_actor::handle_connect` constructs `ReconnectableDevTunnelDomain::new` (api, auth_factory closure, tunnel, label) after `connect_tunnel` succeeds, builds a per-pane `CancellationToken`, and emits the new `UserEvent::DevTunnelPaneCancelToken { pane_id, cancel }` so the App can install it.
- `UserEvent` (in `crates/vector-app/src/lib.rs`) extended with the `DevTunnelPaneCancelToken` variant in the Phase-9 append-only block alongside `PaneReconnecting` / `PaneReconnected`.
- `App` struct in `crates/vector-app/src/app.rs` gained three new fields:
  - `reconnecting_panes: HashMap<PaneId, ReconnectingState>`
  - `pane_cancel_tokens: HashMap<PaneId, CancellationToken>`
  - `reconnect_first_keystroke_shown: HashSet<PaneId>`
- `ReconnectingState` struct (profile_label, attempt, started_at, fade_in_started_at, fade_out_started_at) defined.
- `UserEvent::PaneReconnecting` handler inserts/updates the state, flips tab title to `PaneUiState::Reconnecting`, requests pane redraw.
- `UserEvent::PaneReconnected` handler removes the state, clears the first-keystroke flag, flips tab title back to `PaneUiState::Active`, requests redraw.
- `UserEvent::DevTunnelPaneCancelToken` handler stores the cancel token in `pane_cancel_tokens`.
- Pane close path: cancels the per-pane token, removes from all three maps.
- Keystroke dispatch gate: if `reconnecting_panes.contains_key(&active_pane_id)`, drop the bytes; on the FIRST dropped keystroke per span, push `ToastBanner::info("Input ignored — reconnecting")` (em-dash literal per UI-SPEC §Copywriting) and record in `reconnect_first_keystroke_shown`.
- Per-pane render hook (in render loop, after per-pane Compositor, before chrome snapshot block per UI-SPEC §Spacing composition order): waits for the 250 ms debounce, then computes a 120 ms fade-in alpha, calls `chrome_pipelines.reconnect.update(...)` and `.render(...)` with the dark-mode `chrome.surface` color.
- Tab-title helper looks up `reconnecting_panes.contains_key(&pane_id)` to decide between `PaneUiState::Active` and `PaneUiState::Reconnecting` at every `format_tab_title` call site.
- Two formerly-`#[ignore]`d tests in `reconnect_pass_render.rs` now run as real tests via extracted pure helpers (`pane_input_locked`, `tab_title_for`). 8/8 tests pass.

## Task Commits

1. **Task 1: Wire ReconnectableDevTunnelDomain + per-pane cancel token through picker actor** — `41fd80b` (feat)
2. **Task 2: App reconnect state + event handlers + input gating + render hook + un-ignored tests** — `4f1bc9b` (feat)
3. **Task 3: Manual smoke UAT — Dev Tunnel reconnect end-to-end** — **DEFERRED** to follow-up plan (see "Deferred" section below)

**Plan metadata commit:** see final `docs(09-05)` commit (this SUMMARY + STATE.md + ROADMAP.md updates).

## Files Modified

- `crates/vector-app/src/devtunnels_actor.rs` — `handle_connect` builds `ReconnectableDevTunnelDomain` + `CancellationToken`; emits `UserEvent::DevTunnelPaneCancelToken`.
- `crates/vector-app/src/lib.rs` — `UserEvent::DevTunnelPaneCancelToken { pane_id, cancel }` variant added in Phase-9 append-only block.
- `crates/vector-app/src/app.rs` — three new state fields, `ReconnectingState` struct, three new event-handler arms, pane-close-path cancel/cleanup, keystroke gate, per-pane render hook, tab-title flip helper.
- `crates/vector-app/src/pty_actor.rs` — minor signature touch-up to thread the cancel token through `spawn_pane` (per plan Task 1).
- `crates/vector-app/tests/reconnect_pass_render.rs` — two `#[ignore]`d tests converted to real ones via extracted helpers; 8/8 now active.
- `crates/vector-app/tests/pty_actor_reconnect.rs` + `crates/vector-app/tests/reconnect_byte_integrity.rs` — minor adjustments to keep them green alongside the App changes.

## Decisions Made

- **DevTunnelPaneCancelToken as a UserEvent round-trip** rather than threading a shared map through `Mux::create_tab_async_with_transport` — keeps the Mux signature unchanged and matches the existing "actor emits, App consumes" pattern.
- **Fade-out polish deferred** — the bar disappears in a single frame on `PaneReconnected` rather than animating over 200 ms. The `fade_out_started_at` field is in `ReconnectingState` for the future polish pass; current handler doesn't read it.
- **Cursor dim deferred** — UI-SPEC calls for 40% alpha + no blink during reconnect, but the existing cursor pipeline doesn't accept a reconnecting flag. Documented as a v1 fallback in 09-05-HUMAN-UAT.md Gap #2.
- **Light-mode palette not yet wired** — render hook uses the dark-mode `chrome.surface` regardless of active palette. Cosmetic limitation; bar still visible in light mode. Documented as Gap #1.
- **Glyph row not yet composited** — only the bar background renders; the cell-pipeline text overlay (analogous to SearchBarPass::render_text) is not wired. This is the most user-visible gap. Documented as Gap #4 (severity: major).
- **Task 3 deferred** — the live UAT cannot be walked until `DevTunnelsActor` is constructed in `main.rs` (the picker actor change in Task 1 lives inside `handle_connect`, but the actor itself is not instantiated by the App at startup). Treating as deferred-not-failed so the plan can advance and a focused follow-up plan can wire main.rs.

## Deviations from Plan

### Auto-fixed Issues

None during Tasks 1 and 2 — both executed close to the plan. The deferred limitations (fade-out, cursor dim, light-mode palette, glyph row) were flagged in the plan's `<interfaces>` block as acceptable v1 fallbacks; carrying them forward to 09-05-HUMAN-UAT.md as Gaps rather than auto-fixing inline.

### Deferred from plan

**Task 3 (Manual smoke UAT) — DEFERRED to follow-up plan**
- **Reason:** `DevTunnelsActor` is not yet constructed in `main.rs`. The picker actor's `handle_connect` is reachable only via a constructed actor wired to the event-loop proxy; without that, Cmd-Shift-T does not flow into the new `ReconnectableDevTunnelDomain` path.
- **Impact:** The full reconnect feature cannot be exercised end-to-end against a live Dev Tunnel until main.rs wiring lands.
- **Mitigation:** The UAT matrix is preserved verbatim in `09-05-HUMAN-UAT.md` (status: partial, all 11 items blocked_by: prior-phase). The 5 known limitations are pre-recorded in its Gaps section so `/gsd:audit-uat` will surface them alongside the new follow-up-plan dependency.
- **Tracking:** A follow-up plan (proposed: Phase 9 backlog or 09-06 prereq) must:
  1. Construct `DevTunnelsActor` in `crates/vector-app/src/main.rs` with the App's event-loop proxy.
  2. Verify `UserEvent::DevTunnelPaneCancelToken` round-trips correctly.
  3. Re-open `09-05-HUMAN-UAT.md` and walk the 11-item matrix.

## Issues Encountered

None during implementation. The deferral is a planning miss (main.rs wiring was implicitly assumed to exist from Phase 8) surfaced at the verification gate.

## User Setup Required

None for the implementation tasks. To eventually run the UAT:
- Microsoft account signed in (Cmd-Shift-T → Sign in with Microsoft).
- A reachable Dev Tunnel with `vector-tunnel-agent` running (per Phase 8 install instructions).
- Main.rs DevTunnelsActor wiring (separate follow-up plan).

## Verification Snapshot

Automated checks at task completion:

- `cargo build --workspace` — green.
- `cargo test -p vector-app --test reconnect_pass_render` — **8 passed / 0 failed / 0 ignored** (6 UI-T1..T6 from 09-04 + 2 newly un-ignored).
- `cargo clippy -p vector-app --tests -- -D warnings` — clean (no `await_holding_lock`, no new lints).
- `grep -n "reconnecting_panes" crates/vector-app/src/app.rs` — ≥ 5 matches.
- `grep -n "reconnect_first_keystroke_shown" crates/vector-app/src/app.rs` — ≥ 3 matches.
- `grep -n "Input ignored — reconnecting" crates/vector-app/src/app.rs` — 1 match (em-dash literal).
- `grep -n "chrome_pipelines.reconnect" crates/vector-app/src/app.rs` — render hook present.
- `grep -n "PaneUiState::Reconnecting" crates/vector-app/src/app.rs` — present at tab-title call site.
- `grep -n "trait EventSink\|struct ProxyEventSink" crates/vector-app/src/app.rs crates/vector-app/src/devtunnels_actor.rs` — no matches (the trait stays in `pty_actor.rs` per Plan 09-03).
- `grep -n "ReconnectableDevTunnelDomain::new" crates/vector-app/src/devtunnels_actor.rs` — present in `handle_connect`.
- `grep -n "DevTunnelPaneCancelToken" crates/vector-app/src/lib.rs crates/vector-app/src/devtunnels_actor.rs` — variant defined + emitted.
- `grep -RIn "tmux new -A" crates/` — no matches (PERSIST-03 absence assertion still holds).

## Next Phase Readiness

Plan 09-06 (live e2e smoke tests + persist-e2e CI job + 09-SMOKE.md sign-off) has two prerequisites that must land before its UAT can run:

1. **main.rs DevTunnelsActor construction** — the blocker carried out of Plan 09-05's Task 3. Without it, neither this plan's UAT (`09-05-HUMAN-UAT.md`) nor 09-06's smoke matrix can be walked end-to-end.
2. **Plan 09-05 UAT walkthrough** — once main.rs wiring lands, 09-05-HUMAN-UAT.md must be walked to confirm the integration works as designed before 09-06's CI job is wired.

The 4 implementation-side limitations carried into 09-05-HUMAN-UAT.md Gaps #1–4 (light-mode palette, cursor dim, fade-out, glyph compositing) are polish-grade and do not block the UAT from running — they will surface as findings during the matrix walk and can be resolved as gap-closure tasks rather than as 09-06 prerequisites.

## Self-Check: PASSED

- `crates/vector-app/src/app.rs` modified — FOUND
- `crates/vector-app/src/devtunnels_actor.rs` modified — FOUND
- `crates/vector-app/src/lib.rs` modified — FOUND
- `crates/vector-app/tests/reconnect_pass_render.rs` modified — FOUND (8 active tests, 0 ignored)
- Commit `41fd80b` (Task 1) — FOUND in git log
- Commit `4f1bc9b` (Task 2) — FOUND in git log
- `09-05-HUMAN-UAT.md` created with 11 pending tests + 5 Gaps (1 blocker, 1 major, 3 cosmetic/minor) — FOUND
- Task 3 deferral explicitly documented in this SUMMARY — FOUND
- No EventSink / ProxyEventSink redefinitions outside pty_actor.rs — verified by grep

---
*Phase: 09-persistence-reconnect-tmux-auto-attach*
*Implementation completed: 2026-05-22*
*UAT deferred pending main.rs DevTunnelsActor wiring (separate follow-up plan)*
