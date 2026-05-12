---
phase: 04-mux-tabs-splits
verified: 2026-05-12T05:00:00Z
status: gaps_found
score: 2/4 truths verified (WIN-02 + WIN-04 PASS; WIN-03 FAIL; visible-render acceptance FAIL)
re_verification:
  previous_status: none
  note: "Initial verification of Phase 4."

gaps:
  - truth: "Cmd-D / Cmd-Shift-D split the active pane and render each pane independently side-by-side"
    status: failed
    reason: "Mux split tree mutates correctly (unit-test green); per-pane Compositor render loop is architecturally seeded but not iterating in `WindowEvent::RedrawRequested`. The live `AppWindow` struct in `app.rs` does not carry a `compositors` map — only the unused `TabWindow` struct (in `tab_window.rs`) does. Only the active pane's bytes are fed into the single shared `Term`; non-active panes render nothing visible."
    artifacts:
      - path: "crates/vector-app/src/app.rs:32-40"
        issue: "`struct AppWindow` carries only a single `render_host: Option<RenderHost>` — no `compositors: HashMap<PaneId, Compositor>` field. The per-pane render seam exists in `tab_window.rs` as `TabWindow` but is never instantiated by the live `App::resumed` / Cmd-T code path."
      - path: "crates/vector-app/src/app.rs:485-507"
        issue: "`WindowEvent::RedrawRequested` calls `host.render(&mut t, sel)` once against the single shared Term — no iteration over per-pane compositors with the seeded `LoadOp::Clear` first / `LoadOp::Load` subsequent pattern."
      - path: "crates/vector-app/src/app.rs:293-328"
        issue: "`UserEvent::PaneOutput` is a shim that mirrors ONLY the active pane's bytes into the shared Term; background panes' output is consumed but not rendered."
    missing:
      - "Swap `AppWindow` for `TabWindow` (or extend `AppWindow` with a `compositors: HashMap<PaneId, Compositor>` map) in the live `App.windows` HashMap."
      - "Rewrite `RedrawRequested` to iterate `compositors` in z-order, using `LoadOp::Clear(...)` on the first compositor and `LoadOp::Load` on subsequent, with a single `frame.present()` outside the loop. The `Compositor::render_into_view(LoadOp)` API already exists (Plan 04-04)."
      - "Route `UserEvent::PaneOutput` bytes into the per-pane `Term` (held by `Mux::Pane`) instead of the single shared `App.term`, then dirty-flag only that pane's compositor."

  - truth: "Resizing the window propagates new sizes to all panes so `tput cols` reports each pane's per-viewport width"
    status: failed
    reason: "`Mux::resize_window` correctly returns a `Vec<(PaneId, rows, cols)>` driven by `split_tree::redistribute` + `compute_layout` (unit-tested green at the data layer), and `TabWindow::flush_pending_resize_if_quiescent` (in `tab_window.rs`) correctly walks that vec via `router.send_resize`. But that helper is dead code at runtime — the live `App::flush_pending_resize_if_quiescent` in `app.rs` calls `self.input_bridge.send_resize(rows, cols)` against a SINGLE channel for the bootstrap pane, never walking per-pane via `Mux::resize_window`. As a result both panes report the full window width."
    artifacts:
      - path: "crates/vector-app/src/app.rs:107-119"
        issue: "Live `App::flush_pending_resize_if_quiescent` uses `self.input_bridge.send_resize(rows, cols)` — a single channel to the bootstrap pane. It does not call `Mux::resize_window(window_id, rows, cols)` or iterate per-pane via `PtyActorRouter::send_resize(pane_id, rows, cols)`."
      - path: "crates/vector-app/src/tab_window.rs:72-90"
        issue: "Correctly-shaped `TabWindow::flush_pending_resize_if_quiescent` exists (calls `mux.resize_window` + `router.send_resize` per pane) but is unreachable at runtime because `TabWindow` is never instantiated."
    missing:
      - "Replace the body of `App::flush_pending_resize_if_quiescent` in `app.rs:107-119` with the per-pane walk: `for (pane_id, rows, cols) in mux.resize_window(window_id, rows, cols) { router.send_resize(pane_id, rows, cols); }` — mirroring `tab_window.rs:72-90`."
      - "Plumb `Mux` + `PtyActorRouter` references through `App` so the flush call site can reach them (today `App` only holds `InputBridge`, not the Mux/router; the Mux is reachable via `Mux::try_get()`; the router lives on the I/O thread and is reachable via a stored `Arc<PtyActorRouter>` or via the same `EventLoopProxy<UserEvent>` shim used elsewhere)."
      - "Map the live `winit::WindowId` to a `vector_mux::WindowId` so `Mux::resize_window` can be called with the correct window id."

  - truth: "The active pane is visibly distinguished by a colored border (D-66)"
    status: failed
    reason: "Border shader + uniform setter exist (`Compositor::set_border_color`, cell.wgsl edge-distance test, 2 passing offscreen-pixel snapshot tests in `active_pane_border.rs`), and `App::handle_mux_command(MuxCommand::FocusDir)` mutates `Mux::active_pane_id` + calls `self.request_redraw_all()`. But the visible render path never reaches a per-pane Compositor with `set_border_color` invoked — because the per-pane render loop itself is not wired (Gap 1)."
    artifacts:
      - path: "crates/vector-app/src/app.rs:220-235"
        issue: "`MuxCommand::FocusDir` handler calls `mux.focus_direction` + `request_redraw_all()` but does NOT call `set_border_color` against any compositor — the comment at line 225-228 acknowledges this is deferred until per-pane Compositor map goes live."
      - path: "crates/vector-render/src/compositor.rs"
        issue: "Setter `Compositor::set_border_color` is implemented and unit-tested via offscreen snapshot. Not exercised against the visible per-pane render loop."
    missing:
      - "Once Gap 1 lands the per-pane Compositor map: in the `FocusDir` handler (and on `Mux::active_pane_id` mutation in general), call `compositors[new_active].set_border_color([0.4, 0.6, 1.0, 1.0])` + `compositors[old_active].set_border_color([0.0, 0.0, 0.0, 0.0])` before requesting redraw."
      - "Verify against the manual smoke item #8: focused pane shows 1–2 px accent border; clicking another pane moves the border."

human_verification:
  - test: "Plan 04-06 re-walk of smoke items #3, #4, #8 once the per-pane Compositor render loop, per-pane viewport math, and visible D-66 border land"
    expected: "All three items PASS — visible side-by-side panes; `tput cols` reports per-pane viewport widths after Cmd-D + window resize; focused-pane border is visible against both dark and light themes."
    why_human: "Visual verification (pixel-perceptual border rendering, AppKit tab-group behavior, real-PTY SIGWINCH timing) cannot be programmatically asserted with confidence; the offscreen snapshot test covers the shader, not the live pipeline."

---

# Phase 4: Mux — Tabs & Splits — Verification Report

**Phase Goal:** A user can open a new tab with Cmd-T and split a pane with Cmd-D / Cmd-Shift-D, with each pane running an independent local shell.
**Verified:** 2026-05-12T05:00:00Z
**Status:** `gaps_found`
**Re-verification:** No — initial verification of Phase 4.

## Goal Achievement

The phase goal is partially met:

- **Cmd-T**: PASS — native NSWindowTabbingMode tab grouping verified by user smoke item #1.
- **Cmd-D / Cmd-Shift-D**: PARTIAL — the keystroke is recognized, the Mux split tree mutates correctly (data-layer unit tests green), and a fresh `LocalDomain::spawn_local` PTY is plumbed through `Mux::split_pane_async` + `PtyActorRouter::spawn_pane`. Each pane DOES run an independent shell at the I/O layer (PaneOutput events fire for all panes; per-pane `proc_tracker` emits title-change events for non-active panes — verified by `tracing::info!` lines in `app.rs:293-345`). What FAILS is the user-visible acceptance: only the active pane's output reaches pixels; both panes report the full window width to `tput cols`; no D-66 border is visible.

### Observable Truths

| #   | Truth | Status | Evidence |
| --- | ----- | ------ | -------- |
| 1   | Cmd-T opens a new tab and cycles via Cmd-Shift-]/[; Cmd-W cascades pane → tab → window → quit (WIN-02) | ✓ VERIFIED | User smoke #1 + #2 PASS; `mux_close_cascade.rs` + `mux_tab_cycle.rs` unit tests green; `App::handle_mux_command(NewTab)` calls `WinitWindowFactory::create_tabbed` with `setTabbingIdentifier` (D-56) — confirmed by `multi_window_tabbing.rs` mock-driven unit test. |
| 2   | Cmd-D / Cmd-Shift-D splits the active pane; both panes render side-by-side with independent shells and focus routing (WIN-03 visible) | ✗ FAILED | User smoke #3 FAIL. Mux split tree mutates correctly (data-layer green) but only the active pane's Compositor reaches pixels. See Gap 1. |
| 3   | Resizing the window propagates per-pane viewport sizes so `tput cols` reports each pane's width (WIN-03 #3) | ✗ FAILED | User smoke #4 FAIL. Live `App::flush_pending_resize_if_quiescent` (app.rs:107-119) does not walk `Mux::resize_window`. See Gap 2. |
| 4   | `Domain / Pane / PtyTransport` is the only seam between terminal model and transport — zero `enum PaneSource` / `transport.kind()` discrimination in `vector-term` (WIN-04) | ✓ VERIFIED | `vector-term/tests/no_transport_discrimination.rs` LIVE (not ignored); grep returns 0 forbidden hits across `crates/vector-term/src/`; 2/2 tests pass including negative meta-test. |

**Score:** 2/4 truths verified.

### Required Artifacts (Spot-checked against Plan-frontmatter `key-files`)

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `crates/vector-mux/src/mux.rs` | Mux singleton + topology + async helpers + resize_window | ✓ VERIFIED | 429-line file; `resize_window` correctly returns per-pane (rows, cols) from `split_tree::compute_layout`. |
| `crates/vector-mux/src/split_tree.rs` | Pure algorithms (split_at_leaf, redistribute, compute_layout, get_pane_direction, nudge_ratio) | ✓ VERIFIED | Implemented per Plan 04-02; 6 mux unit-test files green. |
| `crates/vector-mux/src/cwd.rs` + `proc_tracker.rs` | D-57 + D-63 + D-64 plumbing | ✓ VERIFIED | User smoke #5 + #7 PASS. |
| `crates/vector-app/src/tab_window.rs` | Per-TabWindow first-paint gate + compositors map + flush helper | ⚠️ ORPHANED | File exists with correct shape (HashMap<PaneId, Compositor>, correctly-shaped flush helper) but `TabWindow` is never instantiated by `App::resumed` / Cmd-T handler. Only `pub use` in `lib.rs`. |
| `crates/vector-app/src/mux_commands.rs` | MuxCommand dispatch + WindowFactory + VECTOR_TABBING_IDENTIFIER | ✓ VERIFIED | Live. |
| `crates/vector-app/src/app.rs` | App struct + per-window first-paint gate + handle_mux_command + RedrawRequested | ⚠️ PARTIAL | Uses an internal `AppWindow` struct (app.rs:32-40) that lacks the `compositors` map; render loop iterates a single host, not per-pane compositors. |
| `crates/vector-render/src/compositor.rs` | Per-pane viewport + border + cursor_focused + render_into_view | ✓ VERIFIED | All setters + new_with_viewport + render_into_view present; 2/2 offscreen snapshot tests green for the border shader. Not yet exercised against the live multi-pane render path. |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | -- | --- | ------ | ------- |
| `App::handle_mux_command(SplitHorizontal/Vertical)` | `Mux::split_pane_async` + `PtyActorRouter::spawn_pane` | `split_req_tx` mpsc channel + tokio I/O task | ✓ WIRED at data layer | Split spawns succeed; new shell runs; PaneOutput fires. Verified by tracing logs in user smoke run. |
| `App::handle_mux_command(SplitHorizontal/Vertical)` | Per-pane Compositor in visible render loop | (none) | ✗ NOT_WIRED | After split, new pane's Compositor is never inserted into the visible per-window compositors map. Active pane's Term receives all visible bytes. |
| Window resize → per-pane SIGWINCH | `Mux::resize_window` → `PtyActorRouter::send_resize(pane_id, rows, cols)` | `App::flush_pending_resize_if_quiescent` | ✗ NOT_WIRED | Live flush helper bypasses `Mux::resize_window`; sends a single window-total resize on `InputBridge`. |
| `MuxCommand::FocusDir` mutation | `Compositor::set_border_color` per-pane | (deferred) | ✗ NOT_WIRED | Handler calls `request_redraw_all()` only; no compositor-level border-color setter invoked. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| Visible side-by-side panes | `App.windows[wid].compositors` | (does not exist on AppWindow) | N/A | ✗ DISCONNECTED — `AppWindow` lacks the field; `TabWindow` carries it but is unused. |
| `tput cols` per-pane viewport | Per-pane `(rows, cols)` from `Mux::resize_window` | `split_tree::compute_layout` | Yes at data layer; not flowing into kernel SIGWINCH in the live flush path | ⚠️ STATIC (single-pane-shaped flush dispatch) |
| D-66 active-pane border | `Compositor.border_color` uniform | `Compositor::set_border_color` | Yes for the offscreen snapshot test; not invoked at the focus-change handler | ✗ HOLLOW_PROP |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| Workspace test suite green | `cargo test --workspace --tests -q` | 231 passed / 0 failed / 3 ignored | ✓ PASS |
| WIN-04 grep arch-lint live | `cargo test -p vector-term --test no_transport_discrimination -q` | 2 passed / 0 failed | ✓ PASS |
| Arch-lint file count = 16 | `find crates -name 'no_*main.rs' -o -name 'no_transport_discrimination.rs' \| wc -l` | 16 | ✓ PASS |
| `enum PaneSource` / `transport.kind()` zero hits in `vector-term` | `grep -rE "..." crates/vector-term/src/` | 0 hits | ✓ PASS |
| Visible side-by-side panes after Cmd-D | manual smoke #3 | FAIL (user-confirmed) | ✗ FAIL |
| `tput cols` per-pane after Cmd-D + window resize | manual smoke #4 | FAIL (user-confirmed) | ✗ FAIL |
| Visible D-66 border on focus change | manual smoke #8 | FAIL (user-confirmed) | ✗ FAIL |

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
| ----------- | -------------- | ----------- | ------ | -------- |
| WIN-02 | 04-02, 04-04, 04-05 | Tabs: Cmd-T new, Cmd-Shift-]/[ cycle, Cmd-W close | ✓ SATISFIED | User smoke #1 + #2 PASS; data-layer unit tests green; `multi_window_tabbing.rs` mock-driven test asserts `setTabbingIdentifier` call. Marked **Pending** in REQUIREMENTS.md → recommend flipping to **Complete** since both acceptance criteria (visible tab group + Cmd-W cascade) hold. |
| WIN-03 | 04-02, 04-03, 04-04, 04-05 | Splits: Cmd-D / Cmd-Shift-D with focus routing + per-pane resize | ✗ BLOCKED | Data-layer green; visible-render acceptance FAIL on smoke items #3, #4, #8. **Stays Pending in REQUIREMENTS.md per Plan 04-05's documented disposition — correct.** Plan 04-06 (gap-closure) is the agreed path to close. |
| WIN-04 | 04-01, 04-02 | `Domain/Pane/PtyTransport` is the only seam — zero discriminations in `vector-term` | ✓ SATISFIED | Live grep arch-lint passing (`no_transport_discrimination.rs`); negative meta-test proves walker fires on synthetic violations. Marked **Complete** in REQUIREMENTS.md — correct. |

**Orphaned requirements check:** No phase-4 requirement is orphaned. The REQUIREMENTS.md → Phase 4 mapping (WIN-02, WIN-03, WIN-04) matches the union of plan frontmatter declarations.

**WIN-02 disposition note:** Plan 04-05's SUMMARY claimed `requirements-completed: [WIN-02]`, but REQUIREMENTS.md still lists WIN-02 as **Pending** at the time of this verification. Both acceptance criteria for WIN-02 (Cmd-T native tab + Cmd-W cascade) are met. The verifier recommends flipping WIN-02 → **Complete** in REQUIREMENTS.md as part of the Plan 04-06 close-out commit (alongside WIN-03 if 04-06 lands its scope). Leaving WIN-02 Pending now is conservative but not load-bearing.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| `crates/vector-app/src/app.rs` | 293-328 | Shim comment: "only the currently-active Mux pane is mirrored into the visible Term" | ℹ️ Info | Documented intentional scope boundary — not a hidden stub. |
| `crates/vector-app/src/app.rs` | 220-235 | Shim comment: "Multi-pane border flip + cursor_focused toggle lands when the per-pane Compositor map goes live" | ℹ️ Info | Documented intentional scope boundary. |
| `crates/vector-app/src/app.rs` | 180-204 | Comment: "Per-pane Compositor wiring + visible second-shell rendering lands in the multi-pane render polish (Plan 04-06 gap-closure)" | ℹ️ Info | Explicit Plan 04-06 handoff annotation. |
| `crates/vector-app/src/tab_window.rs` | 23-37 | Defined `TabWindow` with `compositors` map is `pub use`-exported but never instantiated in the live `App::resumed` / Cmd-T path | ⚠️ Warning (orphan) | The seam is real, the type is in tree; just unused at runtime. Plan 04-06 swaps `AppWindow` → `TabWindow` or extends `AppWindow` to match. |

No blocker anti-patterns. All stubs are intentional, scope-disciplined, and annotated with a Plan 04-06 reference.

### Human Verification Required

After Plan 04-06 lands, a re-run of smoke items #3, #4, #8 is required. See `human_verification` block in frontmatter.

## Gaps Summary

The user-verdict (6 PASS / 3 FAIL on the 9-item smoke matrix) is honest and matches the codebase exactly. Three failed smoke items collapse to one shared root cause and one architectural gap:

**Root cause:** The phase 4 implementation ships two parallel structs for per-window state:
1. `AppWindow` (in `app.rs:32`) — the live struct used at runtime, single-pane shaped.
2. `TabWindow` (in `tab_window.rs:23`) — the multi-pane-correct struct with `compositors: HashMap<PaneId, Compositor>` + a correctly-shaped `flush_pending_resize_if_quiescent` helper, but never instantiated.

**Architectural gap:** The render loop (`app.rs:485-507`) iterates the single `AppWindow.render_host`; it never reaches a per-pane compositor map. Per-pane viewport-derived SIGWINCH (`app.rs:107-119`) never reaches `Mux::resize_window`. The active-pane border setter is never invoked at the focus-change site (`app.rs:220-235`).

**Plan 04-06 scope (handoff for `/gsd:plan-phase 4 --gaps`):**

- **Task 1 — Per-pane Compositor render loop** (closes Gap 1 + Gap 3 simultaneously)
  - File: `crates/vector-app/src/app.rs:32-40` (AppWindow struct), `crates/vector-app/src/app.rs:485-507` (RedrawRequested), `crates/vector-app/src/app.rs:220-235` (FocusDir handler).
  - Either swap `AppWindow` → `TabWindow` or extend `AppWindow` with `compositors: HashMap<PaneId, Compositor>` + `active_pane_id: PaneId`.
  - Iterate compositors in `RedrawRequested` with `LoadOp::Clear` first / `LoadOp::Load` subsequent. Use the existing `Compositor::render_into_view(LoadOp)` API.
  - In `MuxCommand::FocusDir`: call `set_border_color([0.4, 0.6, 1.0, 1.0])` on the new active compositor and clear it on the old. The D-66 border will then reach pixels automatically.
- **Task 2 — Per-pane viewport math drives SIGWINCH** (closes Gap 2)
  - File: `crates/vector-app/src/app.rs:107-119`.
  - Replace `self.input_bridge.send_resize(rows, cols)` with the per-pane walk shape already implemented in `tab_window.rs:72-90`: `for (pane_id, rows, cols) in mux.resize_window(window_id, rows, cols) { router.send_resize(pane_id, rows, cols); }`.
  - Requires plumbing `Mux` (via `Mux::try_get()`) and `PtyActorRouter` reference into the App for the flush call site, plus a `winit::WindowId` → `vector_mux::WindowId` mapping.
- **Task 3 — Route per-pane PaneOutput to per-pane Term**
  - File: `crates/vector-app/src/app.rs:293-328`.
  - Instead of mirroring only the active pane into the single shared `App.term`, feed each pane's output into its own `Mux::Pane.term` (already exists as `Arc<Mutex<Term>>`), and dirty-flag only that pane's compositor.
- **Acceptance:** Re-walk smoke items #3, #4, #8 — all PASS.

**WIN-03 disposition:** Stays **Pending** in REQUIREMENTS.md until Plan 04-06 closes. This is the correct disposition per Plan 04-05's finalization. Phase 4 close-out is deferred to post-04-06.

**Phase 4 overall:** NOT yet ready to close. 2 of 4 phase truths verified; 3 of 9 smoke items failed; WIN-03 unmet at user-visible acceptance. Plan 04-06 (gap-closure) is the bounded, well-scoped next step.

---

_Verified: 2026-05-12T05:00:00Z_
_Verifier: Claude (gsd-verifier)_
