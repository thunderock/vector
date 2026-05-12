---
phase: 04-mux-tabs-splits
verified: 2026-05-12T12:00:00Z
status: passed
score: 4/4 must-haves verified
re_verification:
  previous_status: gaps_found
  previous_score: 2/4 truths verified
  previous_verified: 2026-05-12T05:00:00Z
  gaps_closed:
    - "Cmd-D / Cmd-Shift-D split the active pane and render each pane independently side-by-side (smoke #3)"
    - "Resizing the window propagates new sizes to all panes so `tput cols` reports each pane's per-viewport width (smoke #4)"
    - "The active pane is visibly distinguished by a colored border (D-66, smoke #8)"
  gaps_remaining: []
  regressions: []
  closure_path: "Plan 04-06 — AppWindow extended in place with `compositors: HashMap<PaneId, Compositor>` + `active_pane_id`; per-pane render loop in `RedrawRequested` (chained LoadOp::Clear/Load + single present); per-pane SIGWINCH via `Mux::resize_window` + `PtyActorRouter::send_resize`; `FocusDir` handler invokes `set_border_color` + `set_cursor_focused` on old/new active. Commits: f6f7d25 (fix), bafae38 (REQUIREMENTS flip), f75e6ed (summary), 8c663a8 (state/roadmap)."
  user_signoff:
    smoke_matrix: "approved (9/9 PASS)"
    date: "2026-05-12"
    location: "04-06-SUMMARY.md §Smoke Matrix Re-Run Results"
---

# Phase 4: Mux — Tabs & Splits — Verification Report (Re-Verification)

**Phase Goal:** A user can open a new tab with Cmd-T and split a pane with Cmd-D / Cmd-Shift-D, with each pane running an independent local shell.
**Verified:** 2026-05-12T12:00:00Z
**Status:** `passed`
**Re-verification:** Yes — initial verification on 2026-05-12T05:00:00Z flagged 3 gaps (smoke items #3, #4, #8); Plan 04-06 closed all three; user signed off on the smoke matrix re-run (9/9 PASS).

## Goal Achievement

The phase goal is met end-to-end. Cmd-T spawns native NSWindow tabs (smoke #1 PASS); Cmd-D / Cmd-Shift-D split with visible side-by-side panes (smoke #3 PASS post-04-06); each pane runs an independent local shell with cwd inheritance (smoke #5 PASS); window resize propagates per-pane SIGWINCH to each child (smoke #4 PASS post-04-06); the active pane is visibly distinguished by the D-66 border (smoke #8 PASS post-04-06); the `Domain / Pane / PtyTransport` seam holds with zero discrimination in `vector-term` (WIN-04 arch-lint live).

### Observable Truths

| #   | Truth | Status | Evidence |
| --- | ----- | ------ | -------- |
| 1   | Cmd-T opens a new tab and cycles via Cmd-Shift-]/[; Cmd-W cascades pane → tab → window → quit (WIN-02) | ✓ VERIFIED | Smoke #1 + #2 PASS (user-approved 2026-05-12); `mux_close_cascade.rs` + `mux_tab_cycle.rs` unit tests green; `multi_window_tabbing.rs` mock-driven test asserts `setTabbingIdentifier` (D-56). |
| 2   | Cmd-D / Cmd-Shift-D splits the active pane; both panes render side-by-side with independent shells and focus routing (WIN-03 visible) | ✓ VERIFIED | Smoke #3 PASS post-04-06 (user-approved). `AppWindow.compositors: HashMap<PaneId, Compositor>` + `active_pane_id` populated lazily on `PaneOutput`; `RedrawRequested` iterates per-pane compositors with chained `LoadOp::Clear` (first) + `LoadOp::Load` (subsequent) + single `frame.present()` (`crates/vector-app/src/app.rs:208-347`). Mux split commands logged dispatching PaneId 1→2→4→6→8 in user smoke run. |
| 3   | Resizing the window propagates per-pane viewport sizes so `tput cols` reports each pane's width (WIN-03 #3) | ✓ VERIFIED | Smoke #4 PASS post-04-06 (user-approved). `flush_pending_resize_if_quiescent` (`crates/vector-app/src/app.rs:140-175`) calls `mux.resize_window(mux_window_id, rows, cols)` → iterates `Vec<(PaneId, prows, pcols)>` → `router.send_resize(pane_id, prows, pcols)` per layout entry. Single-channel `input_bridge.send_resize` retired. |
| 4   | `Domain / Pane / PtyTransport` is the only seam between terminal model and transport — zero `enum PaneSource` / `transport.kind()` discrimination in `vector-term` (WIN-04) | ✓ VERIFIED | `vector-term/tests/no_transport_discrimination.rs` LIVE (2/2 pass including negative meta-test); arch-lint file count = 16. |

**Score:** 4/4 truths verified.

### Required Artifacts (Spot-checked against Plan-frontmatter `key-files`)

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `crates/vector-mux/src/mux.rs` | Mux singleton + topology + async helpers + resize_window | ✓ VERIFIED | `resize_window` returns per-pane `Vec<(PaneId, rows, cols)>` from `split_tree::compute_layout`; now invoked from live flush path. |
| `crates/vector-mux/src/split_tree.rs` | Pure algorithms (split_at_leaf, redistribute, compute_layout, get_pane_direction, nudge_ratio) | ✓ VERIFIED | 6 mux unit-test files green. |
| `crates/vector-mux/src/cwd.rs` + `proc_tracker.rs` | D-57 + D-63 + D-64 plumbing | ✓ VERIFIED | Smoke #5 + #7 PASS. |
| `crates/vector-app/src/app.rs` | App struct + per-window first-paint gate + handle_mux_command + RedrawRequested | ✓ VERIFIED | `AppWindow` now carries `compositors: HashMap<PaneId, Compositor>` + `active_pane_id: Option<PaneId>` + `winit_to_mux_window: HashMap<WindowId, MuxWindowId>`; RedrawRequested iterates per-pane compositors; FocusDir handler flips `set_border_color` + `set_cursor_focused` on old/new active. |
| `crates/vector-app/src/render_host.rs` | Surface-frame closure + lazy per-pane Compositor factory + queue accessor | ✓ VERIFIED | `with_frame<F>`, `new_compositor_for_viewport`, and `queue()` extensions present and exercised at render time. |
| `crates/vector-app/src/main.rs` | `PtyActorRouter` lifted to main thread via `Arc<parking_lot::Mutex<...>>` + `App::set_router` | ✓ VERIFIED | `set_router` call site present after `set_split_req_tx`. |
| `crates/vector-app/src/mux_commands.rs` | MuxCommand dispatch + WindowFactory + VECTOR_TABBING_IDENTIFIER | ✓ VERIFIED | Live. |
| `crates/vector-app/src/tab_window.rs` | Per-TabWindow first-paint gate + compositors map + flush helper | ✓ VERIFIED (carried forward) | Parallel data structure; consumed by `multi_window_tabbing.rs` test. AppWindow was extended in place per 04-06 key-decision rather than swapped — orphan downgrade resolved by intentional dual-data-structure choice. |
| `crates/vector-render/src/compositor.rs` | Per-pane viewport + border + cursor_focused + render_into_view | ✓ VERIFIED | Now exercised against the live per-pane render loop, not just offscreen snapshots. |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| `App::handle_mux_command(SplitHorizontal/Vertical)` | `Mux::split_pane_async` + `PtyActorRouter::spawn_pane` | `split_req_tx` mpsc channel + tokio I/O task | ✓ WIRED | Split spawns succeed; new shell runs; PaneOutput fires per pane. |
| `App::handle_mux_command(SplitHorizontal/Vertical)` | Per-pane Compositor in visible render loop | `AppWindow.compositors` map + `RenderHost::new_compositor_for_viewport` lazy creation on first `UserEvent::PaneOutput` | ✓ WIRED | New pane's Compositor inserted; visible side-by-side render confirmed by smoke #3. |
| Window resize → per-pane SIGWINCH | `Mux::resize_window` → `PtyActorRouter::send_resize(pane_id, rows, cols)` | `App::flush_pending_resize_if_quiescent` (app.rs:140-175) | ✓ WIRED | `tput cols` per-pane confirmed by smoke #4. |
| `MuxCommand::FocusDir` mutation | `Compositor::set_border_color` + `set_cursor_focused` per-pane | `RenderHost::queue` shared wgpu Queue + per-pane compositor map lookup | ✓ WIRED | Border flip + cursor focus flip confirmed by smoke #8 (color `[0.4, 0.6, 1.0, 1.0]` on new-active, cleared on old-active). |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| Visible side-by-side panes | `AppWindow.compositors` | Lazily populated on `UserEvent::PaneOutput` via `RenderHost::new_compositor_for_viewport`; viewport rects from `vector_mux::compute_layout(&tab.root, viewport)` | Yes — per-pane Term bytes flow through per-pane Compositor; user smoke #3 confirms | ✓ FLOWING |
| `tput cols` per-pane viewport | Per-pane `(rows, cols)` from `Mux::resize_window` | `split_tree::compute_layout` → `router.send_resize(pane_id, prows, pcols)` → kernel SIGWINCH per child | Yes — user smoke #4 confirms `tput cols` reports per-pane widths after Cmd-D + window resize | ✓ FLOWING |
| D-66 active-pane border | `Compositor.border_color` uniform | `Compositor::set_border_color([0.4, 0.6, 1.0, 1.0])` invoked in `FocusDir` handler on shared wgpu Queue | Yes — user smoke #8 confirms visible accent border on focused pane | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| Workspace test suite green | `cargo test --workspace --tests -q` | 231 passed / 0 failed / 3 ignored | ✓ PASS |
| WIN-04 grep arch-lint live | `cargo test -p vector-term --test no_transport_discrimination -q` | 2 passed / 0 failed | ✓ PASS |
| D-66 border snapshot tests | `cargo test -p vector-render --test active_pane_border -q` | 2 passed / 0 failed | ✓ PASS |
| Clippy clean (`-D warnings`) | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, no warnings | ✓ PASS |
| Rustfmt clean | `cargo fmt --all -- --check` | exit 0 | ✓ PASS |
| Arch-lint file count | `find crates -name 'no_tokio_main.rs' -o -name 'no_transport_discrimination.rs' \| wc -l` | 16 | ✓ PASS |
| **D-38 zero-diff invariant** | `git diff -- crates/vector-mux/src/domain.rs crates/vector-mux/src/transport.rs \| wc -l` | **0** | ✓ PASS — Phase 2 final trait surface byte-identical |
| Visible side-by-side panes after Cmd-D | manual smoke #3 (user verdict 2026-05-12) | PASS | ✓ PASS |
| `tput cols` per-pane after Cmd-D + window resize | manual smoke #4 (user verdict 2026-05-12) | PASS | ✓ PASS |
| Visible D-66 border on focus change | manual smoke #8 (user verdict 2026-05-12) | PASS | ✓ PASS |

### Manual Smoke Matrix — 9-Item Verdict (Plan 04-06 Re-Run, User-Approved)

The smoke matrix in `04-VALIDATION.md §"Manual-Only Verifications"` is by-design manual (visual/tactile/real-PTY-timing items). The user re-walked all 9 items on 2026-05-12 after Plan 04-06 landed and signed off: **9/9 PASS, 0 FAIL**. Sign-off recorded in `04-06-SUMMARY.md §"Smoke Matrix Re-Run Results"` table and commit `bafae38`.

| # | Behavior | Requirement | 04-05 verdict | 04-06 verdict |
|---|----------|-------------|---------------|---------------|
| 1 | Cmd-T spawns native NSWindow tab | WIN-02, D-56 | PASS | PASS |
| 2 | Cmd-W cascade closes pane → tab → window → app | WIN-02, D-61 | PASS | PASS |
| 3 | Cmd-D + Cmd-Shift-D split + visible side-by-side panes | WIN-03, D-59 | **FAIL** | **PASS** ← closed by 04-06 |
| 4 | `tput cols` round-trip after split + window resize | WIN-03 #3 | **FAIL** | **PASS** ← closed by 04-06 |
| 5 | cwd inheritance via `proc_pidinfo` | D-63 | PASS | PASS |
| 6 | N-pane idle CPU < 1% | RENDER-03 reaffirm | PASS | PASS |
| 7 | Tab title tracks foreground process | D-57 | PASS | PASS |
| 8 | Active-pane border visible (D-66) | WIN-03, D-66 | **FAIL** | **PASS** ← closed by 04-06 |
| 9 | DPR change with N panes | RENDER-04 reaffirm | PASS | PASS |

Net delta vs prior verification: **+3 PASS** (items #3, #4, #8 flipped FAIL → PASS); no regressions on the previously-green six.

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
| ----------- | -------------- | ----------- | ------ | -------- |
| WIN-02 | 04-02, 04-04, 04-05 | Tabs: Cmd-T new, Cmd-Shift-]/[ cycle, Cmd-W close | ✓ SATISFIED | `- [x]` in REQUIREMENTS.md; Traceability row `WIN-02 \| Phase 4 \| Complete`; smoke #1 + #2 PASS. Flipped by Plan 04-06 commit `bafae38`. |
| WIN-03 | 04-02, 04-03, 04-04, 04-05, 04-06 | Splits: Cmd-D / Cmd-Shift-D with focus routing + per-pane resize | ✓ SATISFIED | `- [x]` in REQUIREMENTS.md; Traceability row `WIN-03 \| Phase 4 \| Complete`; smoke #3, #4, #8 PASS. Flipped by Plan 04-06 commit `bafae38`. |
| WIN-04 | 04-01, 04-02 | `Domain/Pane/PtyTransport` is the only seam — zero discriminations in `vector-term` | ✓ SATISFIED | `- [x]` in REQUIREMENTS.md; Traceability row `WIN-04 \| Phase 4 \| Complete`; live grep arch-lint passes (2/2 in `no_transport_discrimination.rs`). |

**Orphaned requirements check:** No phase-4 requirement is orphaned. REQUIREMENTS.md → Phase 4 mapping (WIN-02, WIN-03, WIN-04) is the exact union of plan-frontmatter declarations.

**REQUIREMENTS.md footer:** `*Last updated: 2026-05-12 — Plan 04-06 closed: WIN-02 + WIN-03 complete after smoke matrix re-run (items #3, #4, #8 PASS).*` — consistent with this verification.

### Anti-Patterns Found

None of blocker severity. The three documented-stub comments flagged in the prior verification (`app.rs:293-328` shim, `app.rs:220-235` border-flip deferral, `app.rs:180-204` Plan 04-06 handoff comment) are resolved — the FocusDir handler now invokes `set_border_color` + `set_cursor_focused` on the per-pane compositor map (`crates/vector-app/src/app.rs:193-200, 199-200, 307-315`), the per-pane render loop iterates compositors (`crates/vector-app/src/app.rs:319-347`), and the per-pane Term mirroring is documented as the intentional shape (Plan 04-06 key-decision: "Per-pane Term writes are the source of truth for the render loop"; selection movement to per-pane is explicitly deferred to Phase 5).

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| `crates/vector-app/src/app.rs` | (handle_new_tab) | TODO: subsequent Cmd-T tabs reuse the bootstrap mux WindowId; full per-NSWindow Mux WindowId allocation deferred to Phase 5 | ℹ️ Info | Documented, bounded scope-discipline. Smoke #1 (Cmd-T native tab) PASSes today because the bootstrap mapping suffices; Phase 5 picks up multi-NSWindow Mux WindowId allocation. |

No blocker anti-patterns.

### Human Verification Required

The 9-item smoke matrix is by-design human-verified (visual contrast judgment, AppKit tab-group behavior, real-PTY SIGWINCH timing, DPR change between physical monitors). The user re-walked all 9 items on 2026-05-12 and approved the matrix (9/9 PASS, 0 FAIL). No re-walk is required for this verifier round — human verification is satisfied; sign-off recorded in `04-06-SUMMARY.md`.

## Closure Summary

Plan 04-06 closed the three FAILs from the prior verification with one architectural fix (commit `f6f7d25`): `AppWindow` was extended in place with `compositors: HashMap<PaneId, Compositor>` + `active_pane_id: Option<PaneId>`. The same migration unlocked all three gaps simultaneously:

1. **Gap 1 (smoke #3 — visible side-by-side render):** `RedrawRequested` now derives per-pane viewport rects from `vector_mux::compute_layout`, iterates compositors sorted by PaneId for determinism, calls `Compositor::render_into_view` with chained `LoadOp::Clear` (first leaf) + `LoadOp::Load` (subsequent), and presents once outside the loop.
2. **Gap 2 (smoke #4 — per-pane `tput cols`):** `flush_pending_resize_if_quiescent` now walks `Mux::resize_window(mux_window_id, rows, cols)` → `Vec<(PaneId, prows, pcols)>` → `PtyActorRouter::send_resize(pane_id, prows, pcols)` per layout entry.
3. **Gap 3 (smoke #8 — visible D-66 active-pane border):** `MuxCommand::FocusDir` handler invokes `comp.set_border_color(queue, [0.4, 0.6, 1.0, 1.0])` + `comp.set_cursor_focused(true)` on new-active and clears on old-active using the shared wgpu Queue surfaced via `RenderHost::queue`.

Support extensions: `RenderHost::with_frame<F>` surface-frame closure; `RenderHost::new_compositor_for_viewport` lazy per-pane Compositor factory; `RenderHost::queue` shared-queue accessor; `PtyActorRouter` lifted to `Arc<parking_lot::Mutex<...>>` so `App::set_router` reaches the main-thread render+resize site (`main.rs`); `winit_to_mux_window` map records bootstrap mapping.

All automated verification gates held across the migration: workspace tests 231/0/3 (baseline preserved); clippy clean with `-D warnings`; rustfmt clean; WIN-04 arch-lint live (2/2); D-66 snapshots live (2/2); arch-lint file count = 16; D-38 zero-diff invariant confirmed (`git diff -- crates/vector-mux/src/domain.rs crates/vector-mux/src/transport.rs` returns zero hunks).

## Cross-Phase / Deferred Notes

- **Phase 5 hand-off (Plan 04-06 key-decision):** `winit_to_mux_window` records only the bootstrap entry. Phase 5 (or whichever phase first spawns a fresh Mux Tab+Pane per NSWindow) should extend `handle_new_tab` to allocate a new `vector_mux::WindowId` and record the mapping. TODO comment placed inline.
- **Phase 5 hand-off (Plan 04-06 key-decision):** Per-pane Term writes are the source of truth for the render loop, but the active pane's bytes are mirrored into `self.term` so existing selection + `cell_from_pixel` coords plumbing keeps working. Plan 05 may move selection to per-pane.
- **`tab_window.rs` retained:** Plan 04-06 chose to extend `AppWindow` in place rather than swap to `TabWindow`. `TabWindow` remains `pub use`-exported and consumed by `multi_window_tabbing.rs` as a parallel data structure — intentional dual-data-structure choice documented in 04-06-SUMMARY.md key-decisions; not an orphan.

## Verdict

**Phase 4 is closeable.** All four phase-4 observable truths verified; WIN-02 + WIN-03 + WIN-04 all Complete in REQUIREMENTS.md; manual smoke matrix 9/9 PASS with user sign-off (2026-05-12); D-38 trait-surface invariant byte-identical to Phase 2 final shape; arch-lint count held at 16. No regressions on previously-green items. Phase 5 (Polish — Local Daily-Driver) is plannable from green-bar.

---

_Verified: 2026-05-12T12:00:00Z_
_Verifier: Claude (gsd-verifier)_
_Re-verification of: 2026-05-12T05:00:00Z (initial gaps_found verdict, closed by Plan 04-06)_
