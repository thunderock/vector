---
phase: 04-mux-tabs-splits
plan: 05
subsystem: mux
tags: [winit, wgpu, mux, tabs, splits, first-paint, resize-debounce, focus]

requires:
  - phase: 04-mux-tabs-splits
    provides: "Per-pane PTY actor router (04-03), EncodedKey + Mux shortcuts + multi-window App + per-pane Compositor viewport (04-04)"
provides:
  - "Per-TabWindow first-paint gate (D-51 generalization per Pitfall H)"
  - "Async split-request channel for Cmd-D / Cmd-Shift-D (real Mux pane spawn from main thread)"
  - "Focus side-effects: Cmd-Opt-Arrow directional focus + Cmd-Shift-Arrow nudge-ratio wired into MuxCommand dispatch"
  - "TabWindow::flush_pending_resize_if_quiescent helper (per-window resize debounce, Pitfall D)"
  - "Keystroke routing follows focus (active pane gets PTY writes)"
  - "Workspace test gate: 234 passed / 0 failed / 0 ignored (--include-ignored)"
  - "Partial 9-item smoke matrix sign-off: 6 PASS / 3 FAIL (#3, #4, #8 — visible per-pane render gap)"
affects: ["04-06 (gap-closure for visible side-by-side multi-pane render + per-pane viewport math + D-66 border wire-up)", "05-polish"]

tech-stack:
  added: []
  patterns:
    - "Per-window first-paint gate: TabWindow.first_paint_ready flips on first non-empty PaneOutput for any pane in that window; NEW panes opened later (split) do NOT re-engage the gate"
    - "Async split-request channel: Cmd-D handler on main posts a SplitRequest; tokio task spawns LocalDomain pane + transports back via UserEvent; main installs into Mux + Compositor map"
    - "Per-TabWindow resize debounce: pending_resize + last_resize_at on TabWindow; RedrawRequested-side flush when last_resize_at.elapsed() >= 50ms"
    - "Focus-change side-effects (data-layer): MuxCommand::FocusDir mutates active_pane_id; border/cursor uniform setters present in Compositor but not yet wired to visible per-pane render loop"

key-files:
  created: []
  modified:
    - crates/vector-app/src/app.rs
    - crates/vector-app/src/tab_window.rs
    - crates/vector-app/src/mux_commands.rs
    - crates/vector-app/src/frame_tick.rs
    - crates/vector-render/src/compositor.rs

key-decisions:
  - "Honor the documented scope boundary from Task 1: the visible per-pane Compositor render loop, per-pane viewport math driving tput cols round-trip, and the visible active-pane D-66 border are architecturally seeded in 04-04+04-05 but NOT wired to pixels. These three gaps are the planned scope of Plan 04-06 (gap-closure)."
  - "Record Task 2's 9-item smoke matrix verdict honestly: 6/9 PASS, 3/9 FAIL. Do NOT mark WIN-03 complete in REQUIREMENTS.md — the data-layer passes its unit tests but the user-facing acceptance criteria (visible side-by-side panes; tput cols reflects per-pane viewport) remain unmet."
  - "Phase 4 close-out is deferred until 04-06 lands; verifier next will rightly return gaps_found on WIN-03."

patterns-established:
  - "Per-window first-paint gate generalization (Pitfall H): each TabWindow owns its own gate; new splits never re-engage."
  - "Async split-request channel: split mutations cross thread boundaries via dedicated channel, preserving main-thread ownership of winit + EventLoopProxy invariant (WIN-05)."
  - "Per-TabWindow resize debounce stored on the window struct, flushed from RedrawRequested. No spawned debounce task."

requirements-completed: [WIN-02]

duration: ~30min (Task 1) + ~10min (Task 2 smoke run + finalization)
completed: 2026-05-12
---

# Phase 4 Plan 05: Per-TabWindow Polish + 9-Item Smoke Matrix Summary

**Per-TabWindow first-paint gate + async split-request channel + focus side-effects landed; smoke matrix returned 6/9 PASS with documented FAIL on #3/#4/#8 routing to Plan 04-06 gap-closure.**

## Performance

- **Duration:** ~40 min (Task 1 polish ~30 min; Task 2 smoke + finalization ~10 min)
- **Completed:** 2026-05-12T04:40Z
- **Tasks:** 2 (Task 1 fully complete; Task 2 = partial human-verify, finalized with documented FAILs)
- **Files modified:** 5

## Accomplishments

- Generalized Plan 03-05's single-window first-paint gate (D-51) to per-TabWindow per Pitfall H — NEW panes opened later (Cmd-D split) do NOT re-engage the gate.
- Async split-request channel: Cmd-D / Cmd-Shift-D now spawn real `LocalDomain` panes from a background task and install into the Mux + Compositor map on the main thread via `EventLoopProxy::send_event` (preserves WIN-05 main-thread ownership).
- Focus side-effects wired: Cmd-Opt-Arrow directional focus mutates active_pane_id; Cmd-Shift-Arrow nudge-ratio walks the ancestor split tree.
- `TabWindow::flush_pending_resize_if_quiescent(now, mux, router)` helper centralizes the 50ms debounce flush (Pitfall D).
- Keystroke routing follows focus — writes go to the active pane's `write_tx`.
- Workspace test gate clean: 231 passed / 0 failed / 3 ignored (default); 234 passed / 0 failed / 0 ignored with `--include-ignored`. clippy + fmt clean; arch-lint count 16; D-38 invariant byte-identical.

## Task Commits

1. **Task 1: Per-TabWindow polish + Cmd-D async split + focus side-effects + final sweep** — `22a8272` (feat)
2. **Task 2: 9-item smoke matrix (`checkpoint:human-verify`)** — no code commit (documentation-only; verdict captured in this SUMMARY)

**Plan metadata commit:** (this commit) `docs(04-05): complete plan with documented FAILs on items #3/#4/#8 (gap-closure scope for Plan 04-06)`

## Files Modified

- `crates/vector-app/src/app.rs` — per-TabWindow first_paint_ready; split-request channel install; resize-flush call site
- `crates/vector-app/src/tab_window.rs` — `flush_pending_resize_if_quiescent` helper; per-window pending_resize + last_resize_at; first_paint_ready field
- `crates/vector-app/src/mux_commands.rs` — FocusDir mutates active_pane_id; SplitRequest plumbing
- `crates/vector-app/src/frame_tick.rs` — per-pane coalesce drain emits PaneOutput tagged by pane_id
- `crates/vector-render/src/compositor.rs` — uniform setters available for border/cursor state (consumed by 04-06 gap-closure)

## Manual Smoke Matrix Results

Walked all 9 items from `.planning/phases/04-mux-tabs-splits/04-VALIDATION.md §"Manual-Only Verifications"`. Verdict per item:

| # | Behavior | Requirement | Result | Note |
|---|----------|-------------|--------|------|
| 1 | Cmd-T spawns native NSWindow tab | WIN-02, D-56 | **PASS** | Native tab group; Cmd-Shift-] cycles. |
| 2 | Cmd-W cascade closes pane → tab → window → app | WIN-02, D-61 | **PASS** | All three sub-cases (a/b/c) behave per `Mux::close_pane` CloseResult cascade. |
| 3 | Cmd-D + Cmd-Shift-D split + Cmd-Opt-Arrow focus (visible) | WIN-03, D-59 | **FAIL** | Mux split tree mutates correctly (unit tests green); visible side-by-side panes do NOT render. Only the active pane's Compositor paints. Root cause: per-pane Compositor render loop is architecturally seeded but not wired into `RedrawRequested` iteration. **Scope: Plan 04-06.** |
| 4 | `tput cols` round-trip after split + window resize | WIN-03 #3 | **FAIL** | After Cmd-D, both panes report the full window width — per-pane viewport math is not driving the kernel SIGWINCH ratio split. `mux.resize_window` recomputes layout but per-pane router `send_resize` call does not pass the layout-derived (rows, cols). **Scope: Plan 04-06.** |
| 5 | cwd inheritance via `proc_pidinfo` | D-63 | **PASS** | `libproc::pidcwd` happy path lands the new pane in the source pane's cwd; Cmd-T inherits same. |
| 6 | N-pane idle CPU < 1% | RENDER-03 reaffirm | **PASS** | 4 splits idle 60s → Activity Monitor reports ~0.3% averaged. Per-pane CoalesceBuffer + empty-drain skip works. |
| 7 | Tab title tracks foreground process | D-57 | **PASS** | zsh → vim → zsh title flips within ~1.5s; `tcgetpgrp` + libproc poll firing as designed. |
| 8 | Active-pane border visible (D-66) | WIN-03, D-66 | **FAIL** | Border shader and uniform setter exist in `Compositor`; the focus-change handler does not invoke `set_border_color` against the visible per-pane render path because the per-pane render loop itself is not wired (see #3). **Scope: Plan 04-06.** |
| 9 | DPR change with N panes | RENDER-04 reaffirm | **PASS** | Atlas-clear on `ScaleFactorChanged` invalidates correctly; panes re-rasterize sharp within one frame after monitor swap. |

**Smoke matrix totals:** 6 PASS / 3 FAIL / 0 SKIPPED.

**User verdict (2026-05-12):** "approved with FAIL on items #3, #4, #8 (expected)" — verbatim. The user pre-acknowledged the documented scope boundary from Task 1's executor return: the per-pane Compositor render loop + per-pane viewport math + visible D-66 border are intentionally deferred to Plan 04-06.

## Outstanding Verification Debt (routed to Plan 04-06 gap-closure)

The three FAILs share one root cause and one architectural gap:

**Gap 1 — Per-pane Compositor render loop is not iterating.** `TabWindow.compositors: HashMap<PaneId, Compositor>` is populated, but `WindowEvent::RedrawRequested` only renders the active pane's Compositor with full clear-load semantics. The seeded design from Plan 04-04 was: iterate compositors in z-order with `LoadOp::Clear(...)` on the first and `LoadOp::Load` on subsequent, single `frame.present()` outside the loop. Wiring this is Plan 04-06's Task 1.

**Gap 2 — Per-pane viewport math is not driving SIGWINCH.** `Mux::resize_window` returns `Vec<(PaneId, u16, u16)>`; `TabWindow::flush_pending_resize_if_quiescent` consumes the layout vec but the per-pane `router.send_resize(pane_id, rows, cols)` walks the vec with the wrong indices — every pane ends up receiving the window-total (rows, cols) rather than its layout-computed slice. This is why `tput cols` is identical in both panes. Plan 04-06's Task 2 / Task 3.

**Gap 3 — Visible D-66 border.** Border shader + uniform exist; `set_border_color([0.4, 0.6, 1.0, 1.0])` is called from `handle_mux_command(FocusDir)`, but the per-pane render loop never reaches that compositor with the right `LoadOp` to expose the border. Lands automatically once Gap 1 closes. Plan 04-06's Task 1.

**Why this is honest:** WIN-03's acceptance criteria explicitly include "running an independent shell in each pane" + "tput cols reports correct width" + "focus routing visible". The data-layer green-bar (unit tests for split tree, directional focus, nudge-ratio, close cascade all PASS) does not satisfy the visible-render requirement. WIN-03 stays Pending in REQUIREMENTS.md until Plan 04-06 closes Gaps 1–3.

## Decisions Made

- **Task 1 ships the architecturally-seeded design; Task 2's FAILs are routed to Plan 04-06 instead of inline-fixing.** Wiring the per-pane render loop is a discrete, well-scoped piece of work (one Compositor iteration + one viewport-vec indexing fix + verification that the existing D-66 border setter reaches pixels). It does not belong in a "polish + smoke" plan; it deserves its own gap-closure plan with explicit acceptance criteria tied to items #3/#4/#8.
- **WIN-02 lands** (Cmd-T + Cmd-W cascade both PASS). **WIN-03 does NOT land** (visible side-by-side render + per-pane viewport math remain unmet). **WIN-04 was already landed by Plan 04-02** (grep arch-lint live).
- **Decisions honored partially:** D-51 PASS (per-window gate works); D-56 PASS (#1); D-57 PASS (#7); D-59 = data-layer PASS via 04-02 unit tests, visible FAIL = #3 (defer to 04-06); D-61 PASS (#2); D-63 PASS (#5); D-66 = shader exists but not reaching pixels, FAIL = #8 (defer to 04-06); D-67 PASS (data-layer split tree fully tested via 04-02).

## Deviations from Plan

None for Task 1 — the audit invariants (per-TabWindow first_paint_ready, focus-change side-effects, per-window resize debounce, final clippy/fmt/arch-lint sweep) were all hit on the first pass. The deviation from the Plan-05 success criteria is the smoke matrix verdict, not the implementation: Plan 04-05 expected the 9-item matrix to PASS and close Phase 4; instead it returned 3 documented FAILs that route to a gap-closure plan. This is the expected `gaps_found` outcome the verifier will surface next.

## Issues Encountered

- The 9-item smoke matrix surfaced the three visible-render gaps (#3, #4, #8) which are documented in Plan 04-06's scope. No problem-solving was attempted inline; finalizing per the orchestrator's explicit instruction to "record the partial sign-off honestly".

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **Phase 4 is NOT yet ready to close.** Three of the nine acceptance items are unmet.
- **Plan 04-06 (gap-closure) is the next plan:** spin via `/gsd:plan-phase 4 --gaps`. Its scope is bounded: wire the per-pane Compositor render loop in `RedrawRequested` (Gap 1), fix the per-pane viewport-vec indexing in `flush_pending_resize_if_quiescent` (Gap 2), and verify the D-66 border reaches pixels once Gap 1 closes (Gap 3). Acceptance: re-walk items #3, #4, #8 — all PASS.
- **After 04-06 lands** the phase verifier will close: WIN-03 → Complete; Phase 4 → Complete; ROADMAP marks the phase as fully done; Phase 5 (Polish) becomes plannable.

## Self-Check: PASSED

Verified:
- Task 1 commit `22a8272` exists on `phase3` branch (`git log --oneline -10` shows it).
- All 5 modified-files paths exist in the working tree (per Plan frontmatter `files_modified`).
- 04-VALIDATION.md §"Manual-Only Verifications" enumerates the 9 items walked above.
- REQUIREMENTS.md WIN-03 remains "Pending" — not modified by this commit.

---
*Phase: 04-mux-tabs-splits*
*Plan: 05*
*Completed: 2026-05-12 (partial — Task 1 fully landed; Task 2 finalized with 3 documented FAILs routing to Plan 04-06)*
