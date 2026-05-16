---
phase: 05-polish-local-daily-driver
plan: 16
subsystem: vector-app/chrome
tags: [chrome, wgpu, render-loop, HIGH-2, MEDIUM-2, LOW-2, gap-closure]
dependency_graph:
  requires: [05-14, 05-15]
  provides: [chrome-pipelines-wired, chrome-draw-order-enforced]
  affects: [render_window, AppWindow, chrome_pass]
tech_stack:
  added: [ChromePipelines, PaneRectPx]
  patterns: [parallel-field borrow split, pre-borrow snapshot, per-frame chrome encoder]
key_files:
  created:
    - crates/vector-app/src/chrome.rs
    - crates/vector-app/tests/chrome_render_orchestration.rs
  modified:
    - crates/vector-app/src/lib.rs
    - crates/vector-app/src/app.rs
decisions:
  - "ChromePipelines lives in chrome.rs as a parallel field on AppWindow (not nested in RenderHost) — resolves wgpu double-borrow flagged in HIGH-2 review"
  - "Chrome pass App-state snapshots computed BEFORE aw borrow to avoid borrow-checker conflict with self.windows.get_mut"
  - "PaneRectPx.w_px used for search bar content_w (LOW-2 fix); rect.y_px + rect.h_px - BAR_HEIGHT for top-y (MEDIUM-2 fix)"
metrics:
  duration: "~48 minutes"
  completed: "2026-05-14"
  tasks: 2
  files: 4
---

# Phase 05 Plan 16: ChromePipelines wired into render loop (gap #1 closure) Summary

Chrome render surfaces (TintStripe, SearchBar, Toast, ProfilePicker) instantiated once per window and invoked from the live render loop in UI-SPEC §11 order.

## What Was Built

### Task 1 — ChromePipelines struct + AppWindow parallel field (HIGH-2)

`crates/vector-app/src/chrome.rs` introduces `ChromePipelines { tint, search_bar, toast, picker }` constructed once per `AppWindow` immediately after `RenderHost::new` succeeds. The key architectural decision (locked from 05-REVIEWS.md HIGH-2): the struct is a **parallel field** on `AppWindow` alongside `render_host`, NOT nested inside `RenderHost`. This makes `aw.render_host.as_mut()` and `aw.chrome_pipelines.as_mut()` independently borrowable at the field level — no wgpu double-mutable-borrow possible.

Construction sites updated (3):
- `resumed()` — bootstrap window
- `handle_new_tab()` — Cmd-T tab window
- `handle_app_shortcut(SpawnNewWindow)` — Cmd-N ungrouped window

`PaneRectPx { x_px, y_px, w_px, h_px }` struct added; `App.active_pane_rect: Option<PaneRectPx>` field added (MEDIUM-2 setup).

### Task 2 — Per-frame chrome orchestration + active_pane_rect snapshot

**Toast tick:** `self.toasts.tick(Instant::now())` is called at the top of `render_window` so Info toasts auto-expire after 5 s without an external timer.

**MEDIUM-2 active_pane_rect snapshot:** `self.active_pane_rect = None` reset before the pane loop; inside the loop, when `is_active == true`, `self.active_pane_rect = Some(PaneRectPx { ... })` captures the active pane's pixel rect (computed from `offset_px` + `size_px` in the compositor loop where the rect is already in scope).

**Borrow-checker strategy:** To avoid a conflict between `self.windows.get_mut(&id)` (mutable) and `self.active_profile_tint_rgba()` (immutable `&self`), all App-level state needed for the chrome pass is snapshotted into local variables (`active_tint_rgba`, `search_bar_draw`, `toast_draw`, `picker_draw`) BEFORE the final `aw = self.windows.get_mut(&id)` borrow for the chrome RenderPass. This is clean and avoids `unsafe`.

**Chrome pass block** (after pane loop, LoadOp::Load):

```
chrome-passes encoder -> chrome RenderPass (LoadOp::Load)
  1. chrome.tint.draw(...)         -- iff active_profile_tint_rgba is Some (UI-SPEC §9.3)
  2. [OSC-8 hover — inside grid, no chrome pass]
  3. chrome.search_bar.draw(...)   -- iff search_bar.open AND active_pane_rect is Some (MEDIUM-2/LOW-2)
  4. chrome.toast.draw(...)        -- iff toasts.current() is Some
  5. chrome.picker.draw_scrim(...) -- iff profile_picker.open
     chrome.picker.draw_modal(...) -- same guard
```

**LOW-2 fix:** search bar `content_w = rect.w_px` (active pane width). Before this plan, the fallback was `surface_w` which made the bar span the full window in multi-pane layouts.

**MEDIUM-2 fix:** `bar_top_y = rect.y_px + rect.h_px - SEARCH_BAR_HEIGHT_PX` places the bar at the active pane's bottom edge, not the surface bottom.

**Helper methods added:**
- `active_profile_tint_rgba()` — reads `current_config.profile[active_profile].tint`
- `parse_hex_rgba(hex)` — parses `#RRGGBB` into `[r, g, b, 1.0]`

**No `active_pane_bottom_y_px` function** — the MEDIUM-2 v1-fallback function mentioned in earlier drafts does NOT exist. The snapshot replaces it entirely.

### Tests (7/7 pass)

`crates/vector-app/tests/chrome_render_orchestration.rs`:

| Test | Verifies |
|------|---------|
| `default_state_all_false` | Default App → all chrome surfaces hidden |
| `search_bar_open_no_rect_does_not_draw` | MEDIUM-2: open search bar + no rect → skip |
| `search_bar_open_with_rect_draws` | Open search bar + rect known → draw |
| `picker_open_draws` | ProfilePicker.open → draw_picker |
| `toast_shown_draws` | ToastStack.current Some → draw_toast |
| `tint_color_configured_draws` | Config tint on active profile → draw_tint |
| `chrome_draw_order_matches_ui_spec_section_11` | W6: byte offsets of draw calls in app.rs are monotonically increasing |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `wgpu::RenderPassDescriptor` missing `multiview_mask` field**
- **Found during:** Task 2 compile
- **Issue:** `wgpu 29` added `multiview_mask: None` field to `RenderPassDescriptor`; the plan's code snippet didn't include it.
- **Fix:** Added `multiview_mask: None` to the chrome `RenderPassDescriptor`.
- **Files modified:** `crates/vector-app/src/app.rs`
- **Commit:** d0a79a8

**2. [Rule 1 - Bug] Borrow checker conflict: `self.active_profile_tint_rgba()` inside `aw` borrow**
- **Found during:** Task 2 compile
- **Issue:** The plan's template code called `self.active_profile_tint_rgba()` while `aw = self.windows.get_mut(&id)` was held, causing E0502. The plan's suggested borrow structure `(aw.render_host.as_mut(), aw.chrome_pipelines.as_mut())` is correct for the chrome RenderPass itself but the App-state reads (tint, search_bar, toast, picker) still conflict.
- **Fix:** Moved ALL App-state snapshots (tint, search_bar_draw, toast_draw, picker_draw) to local variables computed BEFORE the final `aw` borrow. Chrome pass then uses only those locals + `aw.render_host`/`aw.chrome_pipelines`.
- **Files modified:** `crates/vector-app/src/app.rs`
- **Commit:** d0a79a8

**3. [Rule 3 - Blocking] `chrome.picker.draw_scrim` uses `content_w`/`content_h` from surface dims**
- **Found during:** Task 2 implementation
- **Issue:** The plan's picker code passed `content_w`/`content_h` as separate parameters, but the scrim covers the full surface. Unified to `surface_w`/`surface_h` since there's no separate "content" region distinct from the surface in v1.
- **Fix:** `draw_scrim(queue, surface_w, surface_h, surface_w, surface_h, &mut rpass)` — symmetric, correct.
- **Files modified:** `crates/vector-app/src/app.rs`
- **Commit:** d0a79a8

## Known Stubs

**Chrome text glyph rendering:** The four chrome passes draw BACKGROUNDS only. Text content (search query string, match counter, toast text, profile names in picker) is not rendered. This is explicitly OUT OF SCOPE per plan §7 ("Glyph rendering for chrome text is OUT OF SCOPE for this plan"). Follow-up task will wire glyph atlas over chrome rects.

## Self-Check: PASSED

Created files exist:
- `crates/vector-app/src/chrome.rs` — FOUND
- `crates/vector-app/tests/chrome_render_orchestration.rs` — FOUND

Commits exist:
- `50d6aeb` (Task 1) — FOUND
- `d0a79a8` (Task 2) — FOUND

Key grep checks:
- `pub struct ChromePipelines` in chrome.rs — PASS
- `chrome_pipelines:` in app.rs — PASS
- `pub mod chrome` in lib.rs — PASS
- No TintStripePipeline/SearchBarPass/ToastPass/PickerPass in render_host.rs — PASS
- 3 `ChromePipelines::new` call sites — PASS
- `chrome-passes` encoder label — PASS
- `self.toasts.tick` call site — PASS
- W6 order test — 7/7 PASS
