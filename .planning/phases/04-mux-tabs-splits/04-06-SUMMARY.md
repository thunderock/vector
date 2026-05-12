---
phase: 04-mux-tabs-splits
plan: 06
subsystem: mux
tags: [winit, wgpu, mux, splits, tabs, sigwinch, compositor, render-loop, gap-closure]
status: complete
gap_closure: true

requires:
  - phase: 04-mux-tabs-splits
    provides: "Per-TabWindow polish + async split-request channel + focus side-effects (04-05); EncodedKey + multi-window App + per-pane Compositor viewport (04-04); per-pane PTY actor router (04-03); Mux topology + split tree + close cascade (04-02)"
provides:
  - "AppWindow extended with `compositors: HashMap<PaneId, Compositor>` + `active_pane_id: Option<PaneId>` — multi-pane shape live"
  - "Per-pane Compositor render loop in RedrawRequested: chained LoadOp::Clear (first leaf) + LoadOp::Load (subsequent), single frame.present() outside the loop"
  - "Per-pane SIGWINCH via `mux.resize_window(window_id, rows, cols)` + `PtyActorRouter::send_resize(pane_id, rows, cols)` (single-channel `input_bridge.send_resize` retired)"
  - "Visible D-66 active-pane border: FocusDir handler invokes `set_border_color([0.4, 0.6, 1.0, 1.0])` on new-active and clears on old-active; cursor focus flips simultaneously"
  - "`RenderHost::with_frame` + `RenderHost::new_compositor_for_viewport` + `RenderHost::queue` extensions enable per-pane surface-frame orchestration"
  - "main.rs lifts `PtyActorRouter` to main thread via `Arc<parking_lot::Mutex<...>>` + `App::set_router`; `winit_to_mux_window` map records bootstrap mapping"
  - "WIN-02 + WIN-03 flipped to Complete in REQUIREMENTS.md (smoke matrix items #3 / #4 / #8 PASS; #1, #2, #5, #6, #7, #9 stayed PASS)"
affects: ["04-verifier (Phase 4 closeable)", "05-polish (inherits per-pane render loop + per-pane SIGWINCH)"]

tech-stack:
  added: []
  patterns:
    - "Per-pane Compositor render loop: acquire surface frame once via `RenderHost::with_frame`; iterate panes sorted by PaneId for determinism; first leaf paints with `LoadOp::Clear(default_bg)`, subsequent leaves with `LoadOp::Load`; single `frame.present()` outside the loop"
    - "Per-pane viewport math drives kernel SIGWINCH: `vector_mux::compute_layout(&tab.root, viewport)` -> Rect-per-PaneId in cells -> `(offset_px, size_px)` per Compositor::set_viewport -> `router.send_resize(pane_id, rows, cols)` for each layout entry"
    - "Focus-change side-effect at the pixel layer: FocusDir handler flips `set_border_color` + `set_cursor_focused` on both old-active and new-active compositors using the shared wgpu Queue surfaced via `RenderHost::queue`"
    - "winit -> mux WindowId bridge: `App.winit_to_mux_window: HashMap<WindowId, vector_mux::WindowId>` records bootstrap mapping in `resumed`; subsequent Cmd-T tabs reuse bootstrap mux WindowId for Plan 04-06 scope (full per-NSWindow Mux WindowId allocation deferred to Phase 5)"

key-files:
  created: []
  modified:
    - crates/vector-app/src/app.rs
    - crates/vector-app/src/main.rs
    - crates/vector-app/src/render_host.rs
    - .planning/REQUIREMENTS.md

key-decisions:
  - "All 9 smoke matrix items PASS on re-run (2026-05-12) — items #3, #4, #8 flipped FAIL -> PASS after Task 1 wired the per-pane render loop + per-pane SIGWINCH + visible D-66 border; items #1, #2, #5, #6, #7, #9 stayed PASS with no regression."
  - "WIN-02 + WIN-03 flipped from Pending to Complete in REQUIREMENTS.md (both the checkbox and the Traceability table)."
  - "AppWindow extended in place rather than swapped to TabWindow — minimizes churn while satisfying the per-pane shape. TabWindow remains `pub use`-d and consumed by `multi_window_tabbing.rs` tests as a parallel data structure."
  - "Per-pane Term mirroring: active pane's bytes mirrored into `self.term` for selection + cursor-coords backward compat; per-pane Term writes are the source of truth for the render loop. Plan 05 may move selection to per-pane."
  - "Pitfall 21 scope guard honored: no layout save/restore, no broadcast-input, no zoom toggle, no new modal modes. Pure render-loop wiring + viewport math + border-color invocation."

patterns-established:
  - "Per-pane Compositor render loop via surface-frame closure (chained LoadOps + single present)."
  - "Per-pane SIGWINCH walk: layout-vec-indexed `router.send_resize(pane_id, rows, cols)` replaces single-channel resize."
  - "Visible focus side-effects: FocusDir flips `set_border_color` + `set_cursor_focused` on the per-pane compositor map using the shared wgpu queue."

requirements-completed: [WIN-02, WIN-03]

duration: ~35min (Task 1 implementation) + ~5min (Task 2 finalization)
completed: 2026-05-12
---

# Phase 4 Plan 06: AppWindow -> Per-Pane Compositor Migration Summary

**AppWindow migrated from single-pane to per-pane shape; per-pane Compositor render loop + per-pane SIGWINCH + visible D-66 active-pane border all reach pixels; smoke matrix flipped 6/9 -> 9/9 PASS; WIN-02 + WIN-03 land.**

## Performance

- **Duration:** ~40 min total (Task 1 implementation ~35 min; Task 2 smoke matrix re-run + finalization ~5 min)
- **Completed:** 2026-05-12
- **Tasks:** 2 (Task 1 fully landed; Task 2 = `checkpoint:human-verify` — user approved with all 9 items PASS)
- **Files modified:** 4

## Accomplishments

- Migrated `AppWindow` from single-pane shape to per-pane shape: added `compositors: HashMap<PaneId, Compositor>` + `active_pane_id: Option<PaneId>`, lazily populated when `UserEvent::PaneOutput` arrives for a new `pane_id`.
- Rewrote `RedrawRequested` arm to iterate the active tab's leaves (sorted by `PaneId` for determinism), calling `Compositor::render_into_view` once per pane with chained `LoadOp::Clear` (first) + `LoadOp::Load` (subsequent), single `frame.present()` outside the loop.
- Replaced single-channel `self.input_bridge.send_resize(rows, cols)` with per-pane walk via `Mux::resize_window(window_id, rows, cols)` -> `PtyActorRouter::send_resize(pane_id, prows, pcols)` so each child shell receives its own kernel SIGWINCH dims.
- Wired the visible D-66 active-pane border: `MuxCommand::FocusDir` handler invokes `Compositor::set_border_color([0.4, 0.6, 1.0, 1.0])` + `set_cursor_focused(true)` on the new-active compositor and `set_border_color([0.0, 0.0, 0.0, 0.0])` + `set_cursor_focused(false)` on the old-active.
- Extended `RenderHost` with `with_frame` (surface-frame closure), `new_compositor_for_viewport` (lazy per-pane Compositor factory), and `queue` (shared wgpu Queue accessor for set_* uniform writes).
- Lifted `PtyActorRouter` to the main thread via `Arc<parking_lot::Mutex<PtyActorRouter>>` + `App::set_router`; main.rs now passes `Arc::clone(&router)` into `App` instead of consuming it solely in the I/O task.
- Smoke matrix re-run 2026-05-12: **9/9 PASS**. Items #3, #4, #8 flipped FAIL -> PASS; items #1, #2, #5, #6, #7, #9 stayed PASS.
- **WIN-02 + WIN-03 flipped to Complete** in `.planning/REQUIREMENTS.md` (both the v1 checkbox and the Traceability table row).

## Task Commits

1. **Task 1: Migrate AppWindow to per-pane Compositor map + per-pane render loop + per-pane SIGWINCH + visible D-66 border** — `f6f7d25` (fix)
2. **Task 2: Smoke matrix re-run + REQUIREMENTS.md flip (`checkpoint:human-verify`)** — `bafae38` (docs)

**Plan metadata commit:** (this commit) `docs(04-06): summary — AppWindow→TabWindow migration closes gaps #3/#4/#8; WIN-02 + WIN-03 Complete`

## Files Modified

- `crates/vector-app/src/app.rs` — `AppWindow` extended with `compositors` map + `active_pane_id`; `RedrawRequested` rewritten to iterate per-pane with chained LoadOp; `flush_pending_resize_if_quiescent` walks `mux.resize_window` + `router.send_resize`; `MuxCommand::FocusDir` invokes `set_border_color` + `set_cursor_focused` on old/new active; `App::set_router` + `winit_to_mux_window` map added; lazy per-pane Compositor creation on first `UserEvent::PaneOutput`.
- `crates/vector-app/src/main.rs` — `PtyActorRouter` lifted to `Arc<parking_lot::Mutex<...>>` so a clone reaches the main-thread `App`; `application.set_router(router_app)` call site added after `set_split_req_tx`.
- `crates/vector-app/src/render_host.rs` — `with_frame<F>(&mut self, F)` surface-frame closure helper (acquires frame, creates view, calls F, presents); `new_compositor_for_viewport(...)` lazy per-pane Compositor factory; `queue() -> Option<&wgpu::Queue>` accessor for set_* uniform writes.
- `.planning/REQUIREMENTS.md` — WIN-02 + WIN-03 flipped from `- [ ]` to `- [x]`; Traceability table rows `WIN-02 | Phase 4 | Pending` and `WIN-03 | Phase 4 | Pending` flipped to `Complete`; footer line appended noting Plan 04-06 close-out.

## Smoke Matrix Re-Run Results (2026-05-12)

Walked all 9 items from `.planning/phases/04-mux-tabs-splits/04-VALIDATION.md §"Manual-Only Verifications"`. **User verdict: approved (all 9 PASS).**

| # | Behavior | Requirement | 04-05 | 04-06 |
|---|----------|-------------|-------|-------|
| 1 | Cmd-T spawns native NSWindow tab | WIN-02, D-56 | PASS | PASS |
| 2 | Cmd-W cascade closes pane → tab → window → app | WIN-02, D-61 | PASS | PASS |
| 3 | Cmd-D + Cmd-Shift-D split + visible side-by-side panes | WIN-03, D-59 | FAIL | **PASS** |
| 4 | `tput cols` round-trip after split + window resize | WIN-03 #3 | FAIL | **PASS** |
| 5 | cwd inheritance via `proc_pidinfo` | D-63 | PASS | PASS |
| 6 | N-pane idle CPU < 1% | RENDER-03 reaffirm | PASS | PASS |
| 7 | Tab title tracks foreground process | D-57 | PASS | PASS |
| 8 | Active-pane border visible (D-66) | WIN-03, D-66 | FAIL | **PASS** |
| 9 | DPR change with N panes | RENDER-04 reaffirm | PASS | PASS |

**Totals:** 9 PASS / 0 FAIL / 0 SKIPPED — net delta +3 PASS vs Plan 04-05.

Mux split commands also dispatched cleanly in the runtime logs (PaneId 1→2→4→6→8 with the 20×4 floor guard firing as expected). Cmd-Opt-Arrow border flip observed: D-66 accent color [0.4, 0.6, 1.0, 1.0] painted on newly-focused pane, cleared on previously-focused pane.

## Gap Closure Summary

The three FAILs from Plan 04-05 shared one architectural root cause (AppWindow was single-pane shaped). All closed in Task 1's single commit `f6f7d25`:

- **Gap 1 (smoke #3 — visible side-by-side render):** `AppWindow` now carries `compositors: HashMap<PaneId, Compositor>` + `active_pane_id`. `WindowEvent::RedrawRequested` derives per-pane viewport rects from `vector_mux::compute_layout(&tab.root, viewport)`, iterates compositors sorted by PaneId, calls `Compositor::render_into_view` with chained `LoadOp::Clear` (first) + `LoadOp::Load` (subsequent), and presents once. **File:line:** `crates/vector-app/src/app.rs` (AppWindow struct + RedrawRequested arm).
- **Gap 2 (smoke #4 — per-pane `tput cols`):** `flush_pending_resize_if_quiescent` now walks `Mux::resize_window(window_id, rows, cols)` -> `Vec<(PaneId, u16, u16)>` and routes each entry through `PtyActorRouter::send_resize(pane_id, prows, pcols)`. Single-channel `self.input_bridge.send_resize(rows, cols)` retired. **File:line:** `crates/vector-app/src/app.rs::flush_pending_resize_if_quiescent`.
- **Gap 3 (smoke #8 — visible D-66 active-pane border):** `MuxCommand::FocusDir` handler invokes `compositor.set_border_color(queue, [0.4, 0.6, 1.0, 1.0])` + `set_cursor_focused(true)` on new-active and `set_border_color(queue, [0.0, 0.0, 0.0, 0.0])` + `set_cursor_focused(false)` on old-active using the shared queue surfaced via `RenderHost::queue`. Border reaches pixels because Gap 1's render loop iterates the compositor with `LoadOp::Load` after the first clear. **File:line:** `crates/vector-app/src/app.rs::handle_mux_command(MuxCommand::FocusDir)`.

All three gaps traced verbatim to the file:line fix locations documented in `.planning/phases/04-mux-tabs-splits/04-VERIFICATION.md`.

## Decisions Made

- **All 9 smoke matrix items PASS on re-run; items #3, #4, #8 flipped FAIL -> PASS** after Task 1 landed the per-pane render loop, per-pane SIGWINCH, and visible D-66 border. Regression-check items #1, #2, #5, #6, #7, #9 stayed PASS with no regression.
- **WIN-02 + WIN-03 flipped to Complete in REQUIREMENTS.md** (both v1 checkbox and Traceability table row). WIN-04 was already Complete from Plan 04-02. All three Phase-4 requirements now Complete.
- **Per-pane Term writes are the source of truth for the render loop**, but the active pane's bytes are mirrored into `self.term` so the existing selection + cell_from_pixel coords plumbing keeps working. Plan 05 may move selection to per-pane.
- **Bootstrap winit->mux WindowId mapping only** (Plan 04-06 bounded scope). Subsequent Cmd-T tabs reuse the bootstrap mux WindowId; full per-NSWindow Mux WindowId allocation is deferred to Phase 5 (TODO comment placed in `handle_new_tab`).

## Deviations from Plan

None — plan executed exactly as written. Task 1's action body specified the seven implementation steps verbatim and Task 1 landed all seven in a single commit without deviation. No Rule 1/2/3 auto-fixes needed.

## Issues Encountered

None. Task 1 verification gates all passed on first attempt:
- `cargo test --workspace --tests -q`: 231 passed / 0 failed / 3 ignored (Plan 04-05 baseline preserved).
- `cargo clippy --workspace --all-targets -- -D warnings`: exit 0.
- `cargo fmt --all -- --check`: exit 0.
- `cargo test -p vector-term --test no_transport_discrimination -q`: 2 passed / 0 failed (WIN-04 grep arch-lint live).
- `cargo test -p vector-render --test active_pane_border -q`: 2 passed / 0 failed (border shader snapshots).
- `find crates -name 'no_tokio_main.rs' -o -name 'no_transport_discrimination.rs' | wc -l`: 16 (arch-lint count held).
- `git diff -- crates/vector-mux/src/domain.rs crates/vector-mux/src/transport.rs`: zero hunks (D-38 invariant byte-identical).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Phase 4 is now closeable.** WIN-02, WIN-03, and WIN-04 are all Complete. The phase verifier (`/gsd:verify-phase 4`) can re-run and return `complete`.
- **Phase 5 (Polish — Local Daily-Driver) becomes plannable** once the Phase 4 verifier closes the phase. Phase 5 inherits the per-pane render loop + per-pane SIGWINCH + per-pane Term plumbing untouched; selection + scrollback + clipboard + theme work begin from green-bar (231/0/3 default; 234/0/0 with `--include-ignored`).
- **Hand-off note:** the `winit_to_mux_window` map records only the bootstrap entry today. Phase 5 (or whichever phase first spawns a fresh Mux Tab+Pane per NSWindow) should extend `handle_new_tab` to allocate a new `vector_mux::WindowId` and record the mapping. TODO comment placed inline.

## Verification

- D-38 invariant: `git diff -- crates/vector-mux/src/domain.rs crates/vector-mux/src/transport.rs` returns zero hunks.
- WIN-04 grep arch-lint: 2/2 PASS (`cargo test -p vector-term --test no_transport_discrimination -q`).
- Border snapshots: 2/2 PASS (`cargo test -p vector-render --test active_pane_border -q`).
- Workspace tests: 231 passed / 0 failed / 3 ignored.
- Clippy + fmt clean.
- Arch-lint count: 16 (held).
- REQUIREMENTS.md WIN-02 `- [x]`; WIN-03 `- [x]`; Traceability rows both `Complete`.

## Self-Check: PASSED

Verified:
- Task 1 commit `f6f7d25` exists on `phase3` branch and touches `crates/vector-app/src/{app.rs, main.rs, render_host.rs}` (`git diff f6f7d25^..f6f7d25 --name-only`).
- Task 2 commit `bafae38` exists on `phase3` branch and flips WIN-02 + WIN-03 in REQUIREMENTS.md.
- `grep -E '\*\*WIN-0[23]\*\*' .planning/REQUIREMENTS.md` shows `- [x]` checkbox on both lines.
- `grep -E 'WIN-0[23] \| Phase 4 \| Complete' .planning/REQUIREMENTS.md` returns 2 hits.
- All four key-files paths exist in the working tree.
- 04-VALIDATION.md §"Manual-Only Verifications" enumerates the 9 items walked above.

---
*Phase: 04-mux-tabs-splits*
*Plan: 06*
*Completed: 2026-05-12 — WIN-02 + WIN-03 Complete; Phase 4 closeable*
