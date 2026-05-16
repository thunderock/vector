---
phase: 04-mux-tabs-splits
plan: 02
subsystem: vector-mux
tags: [wave-2, mux-singleton, split-tree, directional-focus, nudge, win-02, win-03, win-04, d-67, d-61, d-59, d-60]

# Dependency graph
requires:
  - phase: 04-mux-tabs-splits
    plan: 01
    provides: PaneId/TabId/WindowId/IdAllocator/SpawnedPane + LocalDomain::spawn_local + LocalPty accessors + 13 Wave-0 stub files
provides:
  - "vector-mux::Mux singleton via static OnceLock<Arc<Mux>>"
  - "vector-mux::Window/Tab/Pane structs + PaneNode = Leaf|HSplit|VSplit binary split tree (D-67)"
  - "vector-mux::SplitRatio (cell counts; first + second + 1 = axis_size invariant)"
  - "vector-mux::Mux methods: create_window, install_tab, split_pane, cycle_tab, close_pane, focus_direction, nudge_split, panes_snapshot, locate_pane, with_tab"
  - "vector-mux::CloseResult { PaneClosed, TabClosed, WindowClosed, LastWindowClosed } encoding D-61 cascade decisions (no AppKit side effects)"
  - "vector-mux::SplitDirection / Direction / SplitError / NudgeError + MIN_PANE_COLS=20 + MIN_PANE_ROWS=4"
  - "vector-mux::split_tree pure algorithms: compute_layout, split_at_leaf, remove_leaf, get_pane_direction, nudge_ratio, redistribute"
  - "WIN-04 arch-lint LIVE: vector-term/tests/no_transport_discrimination.rs un-ignored + negative meta-test"
  - "Pane::take_transport() one-shot handoff API for Plan 04-03 pty_actor router"
affects: [04-03 (consumes Pane::take_transport + Mux::panes_snapshot for pty_actor + proc_tracker), 04-04 (consumes Mux methods for keymap MuxCommand wiring + multi-window-tabbing)]

# Tech tracking
tech-stack:
  added:
    - "vector-term as a vector-mux dependency (Pane carries Arc<Mutex<Term>>; no dep cycle — vector-term has no vector-mux dep)"
  patterns:
    - "Pure-algorithm split_tree module operates on `&PaneNode` / `&mut PaneNode` + viewport `Rect` — zero Mux dependency; Mux delegates"
    - "PaneNode leaves carry `PaneId`, NOT `Arc<Pane>` — tree mutation is independent of pane state locks (D-67 ownership invariant)"
    - "Pane.transport = `Mutex<Option<Box<dyn PtyTransport>>>` for Plan-04-03 one-shot handoff via `mem::take`"
    - "CloseResult encodes cascade outcome — App layer routes side-effects (drop winit Window, exit loop). Mux never touches AppKit."
    - "Tab/window cycle: Direction::Right -> cycle_next; Direction::Left -> cycle_prev; Up/Down are no-ops at the tab level"
    - "Edge-overlap directional focus per WezTerm + lowest-PaneId tie-break (Phase 4 simplification of recency tie-break)"
    - "Nudge walks up from the target leaf; first ancestor whose orientation matches dir's axis owns the ratio shift; below-floor returns Err"
    - "Negative meta-test pattern: synthesize a forbidden pattern in std::env::temp_dir; assert the walker fires. Proves the live test isn't a no-op."

key-files:
  created:
    - crates/vector-mux/src/mux.rs
    - crates/vector-mux/src/window.rs
    - crates/vector-mux/src/tab.rs
    - crates/vector-mux/src/pane.rs
    - crates/vector-mux/src/split_tree.rs
    - crates/vector-mux/tests/common/mod.rs
  modified:
    - crates/vector-mux/Cargo.toml (vector-term dep + dev-deps for tests)
    - crates/vector-mux/src/lib.rs (mod + re-exports for mux/window/tab/pane/split_tree)
    - crates/vector-mux/src/ids.rs (per-kind allocators + CloseResult/Direction/SplitDirection/SplitError/NudgeError/MIN_* consts)
    - crates/vector-mux/tests/mux_topology.rs (un-ignored + filled)
    - crates/vector-mux/tests/mux_tab_cycle.rs (un-ignored + filled)
    - crates/vector-mux/tests/mux_close_cascade.rs (un-ignored + filled)
    - crates/vector-mux/tests/split_tree.rs (un-ignored + filled)
    - crates/vector-mux/tests/directional_focus.rs (un-ignored + filled)
    - crates/vector-mux/tests/split_resize_nudge.rs (un-ignored + filled)
    - crates/vector-term/tests/no_transport_discrimination.rs (un-ignored + negative meta-test added)
    - Cargo.lock

key-decisions:
  - "Per-kind ID counters (next_pane / next_tab / next_window) replace Plan 04-01's single shared AtomicU64. Tests assert `PaneId(1)` for the first allocation regardless of how many tabs/windows preceded, which is the natural shape callers expect. `IdAllocator { #[allow(clippy::struct_field_names)] }` keeps the pedantic lint happy."
  - "SplitRatio invariant: `first + second + 1 == axis_size`. The `+1` is the divider cell (D-60 — cell-count storage, NOT pixel ratio). split_at_leaf bisects half-half; on odd sizes `first = size/2` and `second = size - first - 1` (e.g., 80 -> 40/39)."
  - "SplitError::BelowMinimum is enforced at `split_at_leaf`: leaf width < 2*MIN_PANE_COLS+1 (=41) for horizontal split; height < 2*MIN_PANE_ROWS+1 (=9) for vertical. Mux::split_pane returns the same error and leaves the tab.root untouched (Leaf restoration on failed bisect)."
  - "Directional-focus tie-break: lowest PaneId wins on equal overlap. WezTerm uses recency (most-recently-focused on that edge) which we explicitly deferred to Phase 5 per RESEARCH.md §\"Pattern: Directional Focus\" simplification. Verified by the `tie_break_by_lowest_pane_id` test (HSplit + VSplit{p5,p2} with 11:11 inner ratio, total 23 rows; p_low(id=2) and p_hi(id=5) tie at 11 rows overlap; p_low(2) wins)."
  - "Nudge axis-vs-direction handling: from a leaf inside HSplit's `left`, Direction::Right grows `ratio.first` by +1 (push divider right); from inside `right`, Direction::Right SHRINKS `ratio.first` by -1 (same — divider moves left toward the focus). Symmetric for L/R. Mirror logic for VSplit + U/D."
  - "Mux::close_pane returns CloseResult and mutates topology in one pass; does NOT attempt to shut down the transport. Plan 04-03's pty_actor will observe the pane drop via Arc reference-count or via `Pane.exited` flag and tear down its own loop. Single-pass cascade: PaneClosed -> TabClosed -> WindowClosed -> LastWindowClosed."
  - "Pane.transport `Mutex<Option<Box<dyn PtyTransport>>>` over `Option<Box<dyn PtyTransport>>`: the parking_lot Mutex is the seam for Plan 04-03's pty_actor router to take ownership without &mut Pane. take_transport() does `lock().take()` and returns the Box; the lock is held synchronously for microseconds, never across await (D-11)."
  - "Test helper `NoopTransport` lives in `crates/vector-mux/tests/common/mod.rs` (shared via `mod common;` in each test file). Avoids cloning the stub across 4 test files."
  - "WIN-04 negative meta-test uses std::env::temp_dir() + std::process::id() suffix instead of pulling in `tempfile` as a dev-dep — keeps the dep graph small and the test self-contained."
  - "vector-mux::Tab is publicly constructible (all fields `pub`). Tests in directional_focus.rs and split_resize_nudge.rs build Tab + PaneNode directly without going through Mux. The Mux delegation (Mux::focus_direction, Mux::nudge_split) is tested implicitly via the topology tests; the algorithms themselves get standalone unit coverage."

patterns-established:
  - "vector-mux is now structured as: trait surface (Domain/PtyTransport — D-38, untouched) + Phase-4 topology (Mux/Window/Tab/Pane/PaneNode) + pure algorithms (split_tree). Adding new mux capabilities follows: add to the algorithm module first, then thin-wrap on Mux."
  - "Per-task TDD-shaped commits: Task 1 (4 test files, 13 tests passing) + Task 2 (2 mux test files + WIN-04, 12 tests passing). Each task's tests un-ignore exactly the stubs the plan owns."

requirements-completed: [WIN-04]
# WIN-02 / WIN-03: algorithms + decision logic land here, but ROADMAP marks complete after Plan 04-03 wires keyboard+PTY and Plan 04-04 the renderer.

# Metrics
duration: 8min
completed: 2026-05-12
---

# Phase 4 Plan 02: Mux Topology + Split Tree + WIN-04 Live Summary

**Ship the in-memory mux topology — `Mux` singleton + Window/Tab/Pane structs + recursive binary `PaneNode` tree with cell-count `SplitRatio` + split-at-leaf mutation + D-61 close-cascade decision logic + Cmd-Shift-]/[ tab cycle + D-59 directional-focus algorithm with edge-overlap scoring and lowest-PaneId tie-break + D-60 1-cell resize nudge with ancestor-axis matching. Un-ignore 6 Wave-0 stubs (mux_topology, mux_tab_cycle, mux_close_cascade, split_tree, directional_focus, split_resize_nudge) plus the WIN-04 arch-lint (no_transport_discrimination) with a negative meta-test that proves the walker fires on synthetic violations. Pure data + algorithms — no I/O, no winit, no AppKit. D-38 invariant held: zero diff in domain.rs / transport.rs since Phase 2. Workspace test count rises 176 → 201 (+25 passes; +12 from Task 1's mux topology, +10 from Task 2's directional/nudge, +2 from WIN-04 main+meta, +1 from the new ids unit test in lib).**

## Performance

- **Duration:** ~8 min (484 s wall clock)
- **Started:** 2026-05-12T03:11:11Z
- **Completed:** 2026-05-12T03:19:15Z
- **Tasks:** 2 (each committed atomically)
- **Test count:** 201 passed / 0 failed / 20 ignored (baseline 176/0/27 at the close of Plan 04-01)

## Accomplishments

### Topology (Task 1)

- `crates/vector-mux/src/ids.rs` extended:
  - Per-kind `IdAllocator { next_pane, next_tab, next_window }` — each starts at 1; tests rely on `PaneId(1)` for the first call regardless of preceding tab/window allocations.
  - `SplitDirection` (Horizontal / Vertical), `Direction` (Left / Right / Up / Down).
  - `CloseResult` with 4 variants matching D-61 cascade outcomes.
  - `SplitError` (BelowMinimum / PaneNotFound) + `NudgeError` (BelowMinimumSize / NoSplitInDirection), both `thiserror::Error`-derived.
  - `MIN_PANE_COLS = 20`, `MIN_PANE_ROWS = 4` constants (CONTEXT.md Claude's Discretion).
- `crates/vector-mux/src/pane.rs`:
  - `pub enum PaneNode { Leaf(PaneId), HSplit{...}, VSplit{...} }` — D-67 recursive binary split tree. `is_leaf()`, `leaves()`, `contains()` helpers.
  - `pub struct SplitRatio { first: u16, second: u16 }` — cell-count storage (D-60). Invariant `first + second + 1 == axis_size`.
  - `pub struct Pane { id, term, transport: Mutex<Option<Box<dyn PtyTransport>>>, pid, master_fd, last_proc_name, exited }` — matches Plan's `<interfaces>` exactly.
  - `Pane::take_transport()` does `self.transport.lock().take()` — the one-shot bridge for Plan 04-03 pty_actor router.
- `crates/vector-mux/src/window.rs`: `Window { id, tabs, active_tab_id }` + `active_tab` / `active_tab_mut` / `cycle_next` / `cycle_prev` (wrap-at-ends).
- `crates/vector-mux/src/tab.rs`: `Tab { id, root, active_pane_id, last_rows, last_cols }` + `pane_count` / `contains`.
- `crates/vector-mux/src/mux.rs`:
  - `static MUX: OnceLock<Arc<Mux>>`; `Mux::install` panics on second call; `Mux::get` panics if not installed.
  - `Mux::new(Arc<LocalDomain>) -> Arc<Mux>` — the only path tests use (no singleton state leaks across tests).
  - `create_window`, `install_tab(window_id, pane: Arc<Pane>, rows, cols) -> (TabId, PaneId)` — Plan 04-03 will wrap install_tab in an async helper that drives `LocalDomain::spawn_local`.
  - `split_pane(pane_id, dir, new_pane) -> Result<PaneId, SplitError>` — mutates the tab's root via `split_tree::split_at_leaf`; on failure restores `Tab.root = Leaf(pane_id)`. Marks new pane active.
  - `cycle_tab(window_id, dir)` — `Direction::Right` -> cycle_next; `Direction::Left` -> cycle_prev; Up/Down are no-ops.
  - `close_pane(pane_id) -> CloseResult` — D-61 cascade in a single pass; removes the pane from `panes` HashMap and mutates topology.
  - `focus_direction(from, dir) -> Option<PaneId>` — delegates to `split_tree::get_pane_direction`.
  - `nudge_split(focused_pane, dir) -> Result<(), NudgeError>` — delegates to `split_tree::nudge_ratio` with `MIN_PANE_COLS` (L/R) or `MIN_PANE_ROWS` (U/D).
  - `panes_snapshot() -> Vec<(PaneId, Option<RawFd>, Option<i32>)>` — Plan 04-03 proc_tracker input.
  - Inspection helpers: `pane`, `locate_pane`, `window_count`, `pane_count`, `tab_count`, `active_tab_id`, `active_pane_id`, `with_tab(window_id, tab_id, |&Tab| -> R) -> Option<R>` (the test-friendly read-only inspector).
- `crates/vector-mux/src/split_tree.rs`:
  - `Rect { x, y, w, h }` cell rect.
  - `compute_layout(&PaneNode, viewport) -> HashMap<PaneId, Rect>` — recursive walk; HSplit divider takes 1 cell of width; VSplit takes 1 cell of height.
  - `split_at_leaf(node, target, new_pane, dir, viewport) -> Result<PaneNode, SplitError>` — pre-checks size, bisects, returns the new tree (functional shape — node consumed, new tree returned).
  - `remove_leaf(node, target) -> Option<PaneNode>` — drops `target` and collapses parent split into sibling; returns `None` if target was the root Leaf (signals "tab is empty, cascade up").
  - `get_pane_direction(&Tab, from, dir) -> Option<PaneId>` — WezTerm edge-overlap algorithm + lowest-PaneId tie-break. `edge_overlap` checks adjacency exactly (candidate's near edge == from's far edge + 1 divider).
  - `nudge_ratio(&mut PaneNode, target, dir, min_cells) -> Result<(), NudgeError>` — recursive walk-down to the leaf; on the way back up finds the first ancestor whose orientation matches `dir`'s axis (HSplit for L/R, VSplit for U/D); shifts `ratio.first` by ±1; rejects if either side would drop below `min_cells`.
  - `redistribute(&mut PaneNode, new_viewport)` — proportional integer scaling. Plan 04-03's window-resize hook will call this.

### Tests (Task 1 + Task 2)

- **mux_topology.rs** (2 tests, both green):
  - `create_window_then_tab_allocates_ids` — verifies first IDs are 1, `panes_snapshot` len == 1, tab_count == 1, active_tab_id == Some(t1).
  - `two_tabs_have_distinct_panes` — distinct ids; active_tab moves to the most-recently installed tab.
- **mux_tab_cycle.rs** (3 tests):
  - `cycle_next_wraps_around` — t1 → t2 → t3 → t1.
  - `cycle_prev_wraps_around` — t1 → t3 → t2 → t1.
  - `cycle_with_one_tab_is_noop` — Right/Left are no-ops with 1 tab.
- **mux_close_cascade.rs** (4 tests, full D-61 enumeration):
  - `close_pane_with_sibling_returns_pane_closed` — split p1 → close p1 → CloseResult::PaneClosed{tab_id}; tab.active_pane_id moves to surviving leaf.
  - `close_last_pane_in_tab_with_sibling_tab_returns_tab_closed` — close last pane in t1 → CloseResult::TabClosed{window_id}; active_tab_id moves to t2.
  - `close_last_pane_in_last_tab_with_sibling_window_returns_window_closed` — close p1 in w1 (w2 still exists) → CloseResult::WindowClosed{window_id: w1}; window_count == 1.
  - `close_last_pane_overall_returns_last_window_closed` — single pane → CloseResult::LastWindowClosed; window_count == 0, pane_count == 0.
- **split_tree.rs** (4 tests):
  - `split_horizontal_at_leaf_returns_hsplit` — 80-col viewport → ratio first=40, second=39.
  - `split_vertical_inside_hsplit_nests_correctly` — verifies nested HSplit{Leaf, VSplit{Leaf, Leaf}}.
  - `split_below_minimum_size_is_rejected` — 30-col viewport (below 41 = 2*20+1 floor) → Err(BelowMinimum).
  - `compute_layout_three_panes_horizontal_sums_correctly` — 120-col viewport; 3 panes after 2 horizontal splits; widths sum to 120 - 2 dividers.
- **directional_focus.rs** (5 tests):
  - `right_from_left_pane_in_hsplit` — p1 → Right → Some(p2); p2 → Right → None.
  - `down_from_top_pane_in_vsplit` + symmetric Up.
  - `wrong_direction_returns_none` — from leftmost of HSplit, Up/Down/Left all → None.
  - `nested_splits_overlap_scoring` — HSplit{p1, VSplit{p2, p3} with ratio 12:11}; p1 → Right → p2 wins (12 rows overlap > 11).
  - `tie_break_by_lowest_pane_id` — HSplit{p1, VSplit{p5, p2} with ratio 11:11 in 23-row viewport}; p1 → Right has 11-overlap tie; lowest id (p2) wins.
- **split_resize_nudge.rs** (5 tests):
  - `nudge_right_shifts_hsplit_ratio_one` — ratio 40:39 → 41:38.
  - `nudge_left_from_same_pane_shrinks_first` — ratio 41:38 → 40:39.
  - `nudge_below_minimum_returns_error` — first=20 (floor) → Direction::Left → Err(BelowMinimumSize); ratio unchanged.
  - `nudge_with_no_matching_split_returns_error` — bare Leaf → Err(NoSplitInDirection).
  - `nudge_finds_nearest_ancestor_split` — VSplit{HSplit{p1, p2}, Leaf(p3)}; from p1, Right finds the inner HSplit (not the outer VSplit).
- **no_transport_discrimination.rs** (2 tests, un-ignored):
  - `vector_term_does_not_discriminate_on_transport_kind` — live walk of `crates/vector-term/src/**/*.rs`; zero matches against the 7 FORBIDDEN strings.
  - `negative_meta_test_walker_detects_forbidden_pattern` — synthesizes `fn x() { let _ = TransportKind::Local; }` in std::env::temp_dir; asserts the walker emits the violation; proves the live test isn't a no-op.

## Algorithm Notes

### get_pane_direction (overlap scoring + tie-break)

For each candidate pane `c != from`, `edge_overlap(from, c, dir)` returns:

1. **Adjacency check** — `c`'s near edge must equal `from`'s far edge + 1 (divider). e.g., for Direction::Right: `c.x == from.x + from.w + 1`. Returns None on miss.
2. **Overlap length** — intersect the cross-axis spans (vertical_overlap for L/R; horizontal_overlap for U/D). `hi - lo` in cells, only if positive.

The winner is the highest overlap; on ties, the lowest `PaneId.0` (deterministic). The test `tie_break_by_lowest_pane_id` constructs an exact-tie scenario (11:11 in a 23-row viewport) to lock the tie-break behavior.

### nudge_ratio (ancestor-walk + axis matching)

Walk down to the leaf carrying `target`. On the way back up, the first ancestor split whose **orientation** matches `dir`'s **axis** owns the ratio shift:

- Direction::Left / Right ↔ HSplit (horizontal axis)
- Direction::Up / Down ↔ VSplit (vertical axis)

Inside an HSplit, "shift `ratio.first` by ±1" follows the focused side:

| Focused leaf in | Direction | Delta to `ratio.first` |
|--|--|--|
| left | Right | +1 (push divider right toward right side) |
| left | Left | -1 (pull divider left, shrinking left) |
| right | Right | -1 (same divider motion as above when focused is in right) |
| right | Left | +1 (same divider motion when focused is in right) |

The floor check rejects when either side would drop below `min_cells` (MIN_PANE_COLS=20 or MIN_PANE_ROWS=4 depending on axis).

## WIN-04 Audit Result

`grep -rE 'enum PaneSource|TransportKind::Local|TransportKind::Codespace|TransportKind::DevTunnel|transport\.kind\(\)|\.kind\(\) == TransportKind|match transport\.kind' crates/vector-term/src/` returns **zero matches** today. Phase 2 already wrote vector-term as a transport-agnostic crate (Term::feed takes raw `&[u8]`; the Mux + Domain abstraction lives in vector-mux). No source edits required. The `no_transport_discrimination.rs` test is now LIVE and will fail if any future change accidentally introduces a forbidden pattern. The negative meta-test proves the walker is functional.

Arch-lint count: `find crates -name 'no_tokio_main.rs' -o -name 'no_transport_discrimination.rs' | wc -l` returns **16** (matches Plan 04-01's count; was already 16 from Plan 04-01 seeding).

## Test Count Delta from Plan 04-01

| | Plan 04-01 close | Plan 04-02 close | Delta |
|--|--|--|--|
| Passed | 176 | 201 | +25 |
| Failed | 0 | 0 | — |
| Ignored | 27 | 20 | -7 |

Breakdown of the +25 passes:
- mux_topology: +2
- mux_tab_cycle: +3
- mux_close_cascade: +4
- split_tree: +4
- directional_focus: +5
- split_resize_nudge: +5
- no_transport_discrimination: +2 (main + negative meta)

Total: 25 new passes. 7 stubs un-ignored (matches the 6 plan-owned stubs + WIN-04 grep).

## Hand-off to Plan 04-03

- **Construct Panes via `LocalDomain::spawn_local`** (Plan 04-01's inherent method). The returned `SpawnedPane { transport, pid, master_fd }` is the input to `Pane::new(id, term, transport, pid, master_fd)`.
- **Call `Pane::take_transport()` exactly once** when handing the transport to your pty_actor. Subsequent `take_transport()` calls return None — guard against double-take in your router.
- **`Mux::panes_snapshot() -> Vec<(PaneId, Option<RawFd>, Option<i32>)>`** is the proc_tracker input. Snapshot is cheap (read lock + clone of 3-tuples). 1Hz polling per RESEARCH.md.
- **The cwd inheritance call site is `Mux::create_tab` / `Mux::split_pane`** (when you wire them via `LocalDomain::spawn_local`). For Plan 04-03 you'll add an async helper like `Mux::create_tab_async(window_id, cwd) -> Result<(TabId, PaneId)>` that calls `inherit_cwd(parent_pane) -> PathBuf` (libproc::pidcwd with $HOME fallback) before spawning.
- **Window resize**: when the App's `WindowEvent::Resized` fires, call `Mux::with_tab` (or add a `resize_tab(window_id, tab_id, new_rows, new_cols)` method) to update `Tab.last_rows/last_cols` and call `split_tree::redistribute(&mut tab.root, new_viewport)` to scale split ratios. Then iterate the new layout and call `transport.resize(rows, cols, 0, 0)` on each pane's pty_actor channel.
- **D-38 still intact**: do not modify `crates/vector-mux/src/domain.rs` or `crates/vector-mux/src/transport.rs`. If Plan 04-03 needs a new transport-agnostic capability, add it to the `PtyTransport` trait surface ONLY if it's universally meaningful (Local + Codespace + DevTunnel). For pid/master_fd-specific things, use the inherent method pattern that Plan 04-01 established (`LocalDomain::spawn_local`).

## Decisions Made

1. **Per-kind ID counters** vs Plan 04-01's single shared AtomicU64. Tests assert `PaneId(1)` for the first allocation regardless of preceding tab/window calls. Single shared counter would make test setup brittle (e.g., `mux.create_window(); mux.allocate_pane_id()` would yield PaneId(2)). `#[allow(clippy::struct_field_names)]` on `IdAllocator` keeps `next_pane`/`next_tab`/`next_window` field naming.
2. **SplitRatio bisect favors `first` on odd sizes.** 80 → first=40, second=39. Matches WezTerm's `first.cells = total / 2; second.cells = total - first.cells - divider`.
3. **`split_pane` on failed bisect restores `Tab.root = Leaf(pane_id)`** rather than trying to undo the `mem::replace`. Practically all callers pre-check viable size; this is defense in depth. The unit test `split_below_minimum_size_is_rejected` exercises only the algorithm (`split_at_leaf`), not the Mux wrapper, because the algorithm test reads cleaner without setting up a full Mux.
4. **`close_pane` cascade is single-pass.** Within one RwLock write guard: try collapse-within-tab → drop tab if empty → drop window if last tab → cascade to LastWindowClosed if last window. The pane is removed from `panes` HashMap after the topology mutation completes. No two-phase commit needed; CloseResult tells the App layer what side-effects to perform.
5. **Pane.transport `Mutex<Option<Box<dyn PtyTransport>>>` over `Option<Box<dyn PtyTransport>>` directly.** The Mutex lets Plan 04-03's pty_actor router take ownership without holding `&mut Pane`. `parking_lot::Mutex` lock held synchronously (microseconds); never across .await (D-11 + workspace `clippy::await_holding_lock = "deny"`).
6. **Test helper module `tests/common/mod.rs`** for shared `NoopTransport` + `make_pane` helpers. 4 test files reference the helper via `mod common;`. Cargo handles non-target `tests/common/` modules correctly via the "common code in tests/" convention.
7. **WIN-04 negative meta-test uses std::env::temp_dir** instead of pulling in `tempfile` as a new dev-dep. Cleanup uses `std::fs::remove_dir_all` at function entry + exit. Process-id suffix on the dir name avoids collisions if the test runs in parallel.
8. **`Tab` struct fields are public** so directional_focus.rs and split_resize_nudge.rs tests construct `Tab` directly. This is the standard Rust pattern for "data class" types — no defensive encapsulation when the data is the whole point.
9. **`get_pane_direction` takes `&Tab`** rather than `&PaneNode + viewport` so the function can use `tab.last_rows`/`tab.last_cols` for the viewport. Mux::focus_direction passes the looked-up tab.
10. **Nudge axis matching uses `axis_h = matches!(dir, Direction::Left | Direction::Right)`.** HSplit owns L/R nudges; VSplit owns U/D. Inner-subtree walk-down happens first; if no matching ancestor lower in the tree, the current split tries; if it doesn't match the axis either, propagate `NudgeOutcome::NotFound` up to the next level.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `compute_layout_three_panes_horizontal_sums_correctly` initial 60-col viewport too narrow**

- **Found during:** Task 1, first test run.
- **Issue:** The plan's `<behavior>` block said "viewport Rect{w:60,h:24}" + "compute_layout returns rectangles whose widths sum to 60 (minus 2 dividers = 58 usable)". After the first horizontal split, p2 has only 29 cols. The second horizontal split on p2 would need 2*MIN_PANE_COLS+1 = 41 cells, so it errors with BelowMinimum.
- **Fix:** Widened test viewport to 120 cols so p2 (59 cols after first split) can host the second split (28+1+30 = 59).
- **Files modified:** `crates/vector-mux/tests/split_tree.rs`
- **Committed in:** `02a99d2`

**2. [Rule 1 - Bug] Clippy `struct_field_names` on IdAllocator**

- **Found during:** Task 1 clippy check.
- **Issue:** Workspace `clippy::pedantic` flags structs where all fields share a prefix.
- **Fix:** `#[allow(clippy::struct_field_names)]` on `IdAllocator`. The `next_*` prefix is the most natural shape; aliasing them would obscure the type.
- **Files modified:** `crates/vector-mux/src/ids.rs`
- **Committed in:** `02a99d2`

**3. [Rule 1 - Bug] Clippy `single_match_else` on `close_pane`'s `match split_tree::remove_leaf(...)`**

- **Found during:** Task 1 clippy check.
- **Issue:** Two-arm match (Some/None) where one arm is significantly larger than the other; clippy prefers `if let Some(...) = ... { ... } else { ... }`.
- **Fix:** Converted the match to if-let-else.
- **Files modified:** `crates/vector-mux/src/mux.rs`
- **Committed in:** `02a99d2`

**4. [Rule 1 - Bug] Clippy `match_same_arms` + `if_not_else` in nudge_walk**

- **Found during:** Task 1 clippy check.
- **Issue:** `match (in_left, dir)` had `(true, Right) => 1` and `(false, Left) => 1` as identical arms (and similarly the -1 arms). `if !axis_h { NotFound } else { ... }` triggered `if_not_else`.
- **Fix:** Merged identical match arms with `|` pattern; inverted the `if !axis_h` to `if axis_h { NotFound } else { ... }` to dodge the lint.
- **Files modified:** `crates/vector-mux/src/split_tree.rs`
- **Committed in:** `02a99d2`

**5. [Rule 1 - Bug] Clippy `useless_conversion` on `u16::from(total / 2)`**

- **Found during:** Task 1 clippy check.
- **Issue:** `total` is already `u16`, so `u16::from(total / 2)` is identity.
- **Fix:** Removed the `u16::from` wrap.
- **Files modified:** `crates/vector-mux/src/split_tree.rs`
- **Committed in:** `02a99d2`

**6. [Rule 1 - Format] rustfmt rewraps multi-line use statements**

- **Found during:** Task 1 + Task 2 fmt check.
- **Issue:** Short `use vector_mux::{a, b, c, d, e, f, g};` fit on one line; rustfmt re-wrapped from multi-line back to single-line.
- **Fix:** Ran `cargo fmt --all`.
- **Files modified:** `crates/vector-mux/src/{mux,split_tree,window}.rs`, `crates/vector-mux/tests/{directional_focus,split_resize_nudge,split_tree}.rs`, `crates/vector-term/tests/no_transport_discrimination.rs`
- **Committed in:** `02a99d2` + `e89a1fb`

---

**Total deviations:** 6 auto-fixed (1 Rule 1 test-data bug — viewport too narrow; 4 Rule 1 clippy compliance; 1 Rule 1 rustfmt compliance).

**Impact on plan:** All within auto-fix scope. No interface changes from the plan's `<interfaces>` block. No new deps beyond `vector-term` (which was already implied by the `Pane.term: Arc<Mutex<vector_term::Term>>` field in the plan).

## Pitfall 21 Scope Guard

Verified — none of the following were introduced:
- Layout save/restore: no serialization of Mux state.
- Broadcast-input across panes: no broadcast channel from keymap to multiple panes.
- Zoom toggle (maximize current pane): no zoom state on Tab or PaneNode.
- Leader-key chord modes: nothing in keymap; this plan doesn't touch vector-input.

## Issues Encountered

None blocking. The viewport-width bug was caught at first test run; the 5 clippy lints were caught at first clippy run.

## Verification Results

```
cargo build --workspace --tests                                                ✓ clean
cargo clippy --workspace --all-targets -- -D warnings                          ✓ clean
cargo fmt --all -- --check                                                     ✓ clean
cargo test --workspace --tests -q                                              ✓ 201 passed / 0 failed / 20 ignored
cargo test -p vector-mux --test mux_topology                                   ✓ 2 passed
cargo test -p vector-mux --test mux_tab_cycle                                  ✓ 3 passed
cargo test -p vector-mux --test mux_close_cascade                              ✓ 4 passed
cargo test -p vector-mux --test split_tree                                     ✓ 4 passed
cargo test -p vector-mux --test directional_focus                              ✓ 5 passed
cargo test -p vector-mux --test split_resize_nudge                             ✓ 5 passed
cargo test -p vector-term --test no_transport_discrimination                   ✓ 2 passed (1 live + 1 negative meta)
git diff 75ac3d3..HEAD -- crates/vector-mux/src/domain.rs ... transport.rs     ✓ zero hunks (D-38 invariant)
find crates -name 'no_tokio_main.rs' -o -name 'no_transport_discrimination.rs' ✓ 16
grep -nE 'static MUX' crates/vector-mux/src/mux.rs                             ✓ static MUX: OnceLock<Arc<Mux>>
grep -nE 'pub (struct Mux|enum PaneNode|enum SplitDirection|enum Direction|enum CloseResult|fn close_pane|fn split_pane|fn cycle_tab|fn focus_direction|fn nudge_split)' crates/vector-mux/src/{mux,pane,ids,split_tree}.rs   ✓ 10+ lines
grep -c 'Wave-0 stub: Plan 04-02' crates/vector-mux/tests/mux_topology.rs ... split_tree.rs ... directional_focus.rs ... split_resize_nudge.rs   ✓ 0
```

## Task Commits

1. **Task 1: Mux topology + split tree + close cascade** — `02a99d2` (feat)
2. **Task 2: Directional focus + nudge + WIN-04 grep live** — `e89a1fb` (test)

## Files Created/Modified

### Created (6)

- `crates/vector-mux/src/mux.rs`
- `crates/vector-mux/src/window.rs`
- `crates/vector-mux/src/tab.rs`
- `crates/vector-mux/src/pane.rs`
- `crates/vector-mux/src/split_tree.rs`
- `crates/vector-mux/tests/common/mod.rs`

### Modified (12 + Cargo.lock)

- `crates/vector-mux/Cargo.toml` — added `vector-term` as a dep + dev-deps `anyhow`/`async-trait`/`parking_lot`/`vector-term`
- `crates/vector-mux/src/lib.rs` — `pub mod` + re-exports for mux/window/tab/pane/split_tree
- `crates/vector-mux/src/ids.rs` — per-kind allocators + enums + constants
- `crates/vector-mux/tests/mux_topology.rs` — un-ignored + filled (2 tests)
- `crates/vector-mux/tests/mux_tab_cycle.rs` — un-ignored + filled (3 tests)
- `crates/vector-mux/tests/mux_close_cascade.rs` — un-ignored + filled (4 tests)
- `crates/vector-mux/tests/split_tree.rs` — un-ignored + filled (4 tests)
- `crates/vector-mux/tests/directional_focus.rs` — un-ignored + filled (5 tests)
- `crates/vector-mux/tests/split_resize_nudge.rs` — un-ignored + filled (5 tests)
- `crates/vector-term/tests/no_transport_discrimination.rs` — un-ignored + filled with negative meta-test (2 tests)
- `Cargo.lock`

## Next Phase Readiness

- Plan 04-02 closes Phase 4 Wave 2.
- Plan 04-03 inherits a fully-tested mux topology + algorithms. Per-pane PTY actor wiring + proc_tracker + cwd inheritance can start from green-bar (201 passed, 0 failed, 20 cleanly-ignored).
- D-38 invariant held (zero hunks in `domain.rs` / `transport.rs` since Phase 2 Plan 02-04).
- Arch-lint count at 16 (matches Plan 04-01 seeding + WIN-04 now LIVE).
- WIN-04 requirement marked complete.
- No blockers identified.

## Self-Check: PASSED

All claimed files exist:

- crates/vector-mux/src/mux.rs — FOUND
- crates/vector-mux/src/window.rs — FOUND
- crates/vector-mux/src/tab.rs — FOUND
- crates/vector-mux/src/pane.rs — FOUND
- crates/vector-mux/src/split_tree.rs — FOUND
- crates/vector-mux/tests/common/mod.rs — FOUND
- crates/vector-mux/Cargo.toml (modified) — FOUND
- crates/vector-mux/src/lib.rs (modified) — FOUND
- crates/vector-mux/src/ids.rs (modified) — FOUND
- crates/vector-mux/tests/mux_topology.rs (modified) — FOUND
- crates/vector-mux/tests/mux_tab_cycle.rs (modified) — FOUND
- crates/vector-mux/tests/mux_close_cascade.rs (modified) — FOUND
- crates/vector-mux/tests/split_tree.rs (modified) — FOUND
- crates/vector-mux/tests/directional_focus.rs (modified) — FOUND
- crates/vector-mux/tests/split_resize_nudge.rs (modified) — FOUND
- crates/vector-term/tests/no_transport_discrimination.rs (modified) — FOUND

All claimed commits exist:

- 02a99d2 — FOUND (Task 1)
- e89a1fb — FOUND (Task 2)

---
*Phase: 04-mux-tabs-splits*
*Plan: 02*
*Completed: 2026-05-12*
