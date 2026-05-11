---
phase: 03-gpu-renderer-first-paint
plan: 03
subsystem: render
tags: [wgpu, wgsl, compositor, cell-pipeline, cursor-pipeline, damage, truecolor, xterm-256, offscreen, surface-recovery]

# Dependency graph
requires:
  - phase: 03-gpu-renderer-first-paint
    plan: 01
    provides: "RenderContext (device/queue/surface/config), Arc<parking_lot::Mutex<Term>>, Term::damage()/reset_damage() + TermDamage re-exports, Wave-0 stub paths"
  - phase: 03-gpu-renderer-first-paint
    plan: 02
    provides: "Atlas::new + slot_for + mono_view/color_view + clear_all, FontStack::load_bundled + rasterize + cell_metrics, BitmapKind::{Mono,Color}"
provides:
  - "vector-render::Compositor::new + Compositor::new_with (device/queue/format/w/h/font_stack) — surface-free build path for tests"
  - "Compositor::render(&RenderContext, &mut Term, Option<((u16,u16),(u16,u16))>) -> Result<(), CompositorError> — selection arg from day one (Plan 03-04 populates)"
  - "Compositor::render_offscreen + render_offscreen_with — Rgba8Unorm offscreen render + padded staging readback (returns OffscreenFrame { width, height, pixels, format })"
  - "Compositor::cell_width_px / cell_height_px / surface_format / atlas_mut — Plan 03-04 + 03-05 hooks"
  - "vector-render::CellPipeline + CellInstance (72-byte Pod) + cell.wgsl (vertex + fragment, mono/color/empty atlas-kind branch, per-cell selected blend to selection_tint)"
  - "vector-render::CursorPipeline + cursor.wgsl (block cursor, second render pass with LoadOp::Load over cell pass)"
  - "vector-render::CompositorError { Outdated, Lost, Timeout, Validation } — replaces wgpu 29's removed SurfaceError on the render path; Outdated/Lost auto-reconfigure the surface"
  - "vector-render::Offscreen + RenderContext::new_offscreen — headless device+queue probe for snapshot tests; no winit window required"
  - "vector-render::OffscreenFrame public type — exported for downstream tests"
  - "vector-app::RenderHost::render(&mut Term, Option<((u16,u16),(u16,u16))>) — lazy Compositor init; clear-color fallback if FontStack/Compositor fails"
  - "5 Wave-0 stubs un-ignored: damage_to_quads, snapshot_singlecell, snapshot_truecolor, snapshot_clearcolor, cursor_overlay_snapshot"
affects: [03-04-input, 03-05-pacing-polish, 04-mux]

# Tech tracking
tech-stack:
  added: []  # all deps locked at workspace level in Plan 03-01
  patterns:
    - "CellInstance: #[repr(C)] Pod+Zeroable, 72 bytes per instance, 8 vertex attributes (cell_pos u32x2, fg/bg/uv f32x4, atlas_kind/selected/flags u32) plus a u32 pad — naga relaxed instance-stride layout"
    - "Compositor renders in two passes per frame: cell pass (LoadOp::Clear to default_bg) → cursor pass (LoadOp::Load); single command encoder; one queue.submit per frame"
    - "Term lock scope in app.rs::RedrawRequested: `let mut t = self.term.lock(); host.render(&mut t, None)` — guard drops at end of arm; no .await in render path; clippy::await_holding_lock = deny satisfied at compile time (D-11)"
    - "Selection arg baked into Compositor::render from day one; Plan 03-03 callers (RedrawRequested, snapshot tests) pass None; Plan 03-04 will plumb the selection state machine's range; no signature drift"
    - "Surface-free test harness: RenderContext::new_offscreen returns a Device+Queue+format without a winit window; Compositor::new_with consumes that triple; render_offscreen_with renders to a self-allocated Rgba8Unorm texture and reads back through a padded staging buffer (COPY_BYTES_PER_ROW_ALIGNMENT-aligned)"
    - "CompositorError::{Outdated,Lost} auto-recovers: Compositor::render reconfigures the surface in-place; vector-app::RenderHost::render swallows those variants and lets the next RedrawRequested retry"

key-files:
  created:
    - crates/vector-render/src/cell_pipeline.rs
    - crates/vector-render/src/cursor_pipeline.rs
    - crates/vector-render/src/compositor.rs
    - crates/vector-render/src/shaders/cell.wgsl
    - crates/vector-render/src/shaders/cursor.wgsl
    - crates/vector-render/tests/common/offscreen.rs
    - crates/vector-render/tests/fixtures/.gitkeep
  modified:
    - Cargo.lock
    - crates/vector-render/Cargo.toml (+alacritty_terminal direct dep + dev-dep)
    - crates/vector-render/src/lib.rs (mod tree extended, pub use Compositor/CompositorError/OffscreenFrame/CursorPipeline/CursorInstance/Offscreen)
    - crates/vector-render/src/pipeline.rs (added Offscreen + RenderContext::new_offscreen test path)
    - crates/vector-render/tests/damage_to_quads.rs (pixel-asserts red-dominant top-row strip after feed of "\x1b[31mA\x1b[0m")
    - crates/vector-render/tests/snapshot_singlecell.rs (feed 'X' lands at grid[0,0])
    - crates/vector-render/tests/snapshot_truecolor.rs (\x1b[38;2;255;128;0mZ lands as Color::Spec(Rgb { 255,128,0 }))
    - crates/vector-render/tests/snapshot_clearcolor.rs (empty grid is mostly-dark; cursor cell within budget)
    - crates/vector-render/tests/cursor_overlay_snapshot.rs (cursor cell center is near light-gray RGB > 150)
    - crates/vector-app/Cargo.toml (+vector-fonts dep)
    - crates/vector-app/src/render_host.rs (lazy Compositor init + render(&mut Term, selection) + Outdated/Lost handling)
    - crates/vector-app/src/app.rs (RedrawRequested locks Term, calls host.render(&mut t, None), drops lock)

key-decisions:
  - "CellInstance is 72 bytes (8+16+16+16+4+4+4+4 = 72) with a u32 _pad — naga accepts this; 16-byte alignment is not strictly required for instance buffers in wgpu 29 (vertex strides aren't subject to std140-style padding rules). Compile-time `const _: () = assert!(size_of::<CellInstance>() == 72);` guards future drift."
  - "xterm-256 palette source: standard xterm 256-color table — 16 ANSI base + 6×6×6 cube (CUBE_STEPS = [0, 95, 135, 175, 215, 255]) + 24-step grayscale ramp (v = 8 + 10·i). Inlined as a constexpr-built [[f32; 4]; 256] in `xterm_256_palette()`. Source comment cites https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit and the xterm sources."
  - "CompositorError enum replaces wgpu 29's removed SurfaceError. wgpu 29 returns `CurrentSurfaceTexture::{Success, Suboptimal, Outdated, Lost, Timeout, Occluded, Validation}` from `Surface::get_current_texture()`; our render path pattern-matches that into our local error enum so downstream callers get stable names regardless of wgpu's internal status renaming."
  - "Damage tracking: snapshot `TermDamage` (Full or Partial) into an owned `Vec<(u16,u16,u16)>` before any GPU work, then `term.reset_damage()` immediately — both under the caller's Term lock scope (app.rs scopes the lock to the `host.render(&mut t, ...)` call). Plan 03-03 always rebuilds the entire instance buffer per frame for simplicity; partial slice rewrites are tracked in `_damage_rows` and remain available for Plan 03-05 if profiling demands per-row writes."
  - "Cursor pipeline pass uses `LoadOp::Load` so it composites over the cell-pass output without erasing it. Block-cursor color = [0.85, 0.85, 0.85, 1.0] (light gray); Plan 03-05 promotes to a theme uniform and adds blink (D-40 discretion deferred per CONTEXT.md)."
  - "Surface-free test path: `RenderContext::new_offscreen(w, h)` requests an adapter with `compatible_surface: None` (no window) and builds Device+Queue; `Compositor::new_with(device, queue, format, w, h, font_stack)` consumes that. Tests don't need a winit `Window` on the main test thread — `cargo test` parallelism is preserved."
  - "Render-path pattern: `RenderHost::render` calls `ensure_compositor()` (one-shot lazy build that records `compositor_failed = true` on FontStack/Compositor::new error), then matches `comp.render(...)` result. `Outdated|Lost` is `Ok(())` because the surface was already reconfigured by Compositor::render; the next RedrawRequested will retry. Other errors propagate via anyhow."
  - "Plan-vs-shipped: the plan referenced a `selection_overlay_snapshot.rs` un-ignore but that test belongs to Plan 03-04 per the Wave-0 stub map — left ignored. Per-row instance-buffer slice rewrites are tracked but not exercised in Plan 03-03 (full-rebuild is correct, just slightly more work); Plan 03-05's pacing pass will exercise the slice-rewrite seam if profiling shows it matters."

patterns-established:
  - "Compositor::new_with(device, queue, format, w, h, font_stack) is the canonical builder for test paths; `new` is the production path over a RenderContext. Phase 4's mux can use either."
  - "render_offscreen_with(device, queue, w, h, term, selection) is the surface-free render entrypoint — same uniform set-up + pass encoding as the on-screen path, ends in `copy_texture_to_buffer` + map_async read-back."
  - "`u32::from(bool)` for boolean-to-u32 packing (instead of `if x { 1 } else { 0 }`); avoids clippy `bool_to_int_with_if`."
  - "Module-level `#![allow(clippy::cast_precision_loss, too_many_lines, similar_names, items_after_statements)]` in compositor.rs scoped to the file — viewport float math + the long render fn + xterm_256_palette's inline constants are all pre-approved per the plan's `<behavior>` description."

requirements-completed: [RENDER-01, RENDER-04, RENDER-05]

# Metrics
duration: 14 min
completed: 2026-05-11
---

# Phase 3 Plan 03: Cell + Cursor Pipelines + Compositor — Summary

**Cell + cursor wgpu pipelines compositing `vector_term::Term.grid()` over a wgpu Metal surface; 24-bit truecolor + 256-color SGR paths through CellInstance fg/bg; per-cell `selected` bit wired in the fragment shader (Plan 03-04 populates the state machine); WIDE_CHAR_SPACER cells skipped per Pitfall 4; `Term::damage()/reset_damage()` consumed under a brief Mutex scope per D-11. 5 Wave-0 stubs un-ignored — three with offscreen pixel-snapshot assertions, two as plumbing smokes. RENDER-01, RENDER-04, RENDER-05 land.**

## Performance

- **Duration:** 14 min
- **Started:** 2026-05-11T19:55:20Z
- **Completed:** 2026-05-11T20:09:44Z
- **Tasks:** 2 (both TDD-tagged; Wave-0 stub files provided the failing baseline, Task 1 un-ignored 3 as plumbing smokes, Task 2 upgraded 3 to pixel-snapshot asserts + un-ignored 2 more)
- **Files modified:** 12 modified, 7 created (5 src + 1 test harness module + 1 fixtures dir marker)

## Accomplishments

- **CellPipeline + cell.wgsl ship the cell-grid render path.** Instanced quad over the screen's cells: `CellInstance { cell_pos: [u32; 2], fg: [f32; 4], bg: [f32; 4], uv: [f32; 4], atlas_kind: u32, selected: u32, flags: u32, _pad: u32 }`, 72 bytes per instance, `#[repr(C)]` + `Pod+Zeroable`. Vertex shader maps cell_pos × cell_size_px → NDC with wgpu's y-down flip; fragment branches on `atlas_kind`:
  - 0 = Mono → `mix(bg.rgb, fg.rgb * sample.rgb, max(sample.r, sample.g, sample.b))`
  - 1 = Color → `mix(bg.rgb, sample.rgb, sample.a)` (premultiplied emoji)
  - 2 = Empty → `bg.rgb`
  Then `frag_selected == 1u` blends `mix(out.rgb, selection_tint.rgb, selection_tint.a)`; tint = `[0.27, 0.48, 0.78, 0.40]` (xterm-ish translucent blue). `INVERSE` flag swaps fg/bg in the vertex stage.
- **CursorPipeline + cursor.wgsl ship the block cursor.** Single-quad draw call per frame; uniform = `{ viewport_size_px, cell_size_px, cursor_cell, cursor_color }`; fragment returns the cursor color (light gray `[0.85; 4]`). Second render pass with `LoadOp::Load` composites over the cell-pass output. Blink rate deferred to Plan 03-05 per CONTEXT discretion.
- **Compositor::render reads Term::damage()/reset_damage() under a brief lock scope (D-11).** Snapshot grid → drop lock-equivalent scope → upload instances → encode 2-pass draw → submit. Pitfall 4 honored: `Flags::WIDE_CHAR_SPACER` cells skipped (lead cell paints the wide glyph in its own cell rectangle for v1; widening to a 2-cell quad is a Phase 4+ improvement).
- **24-bit truecolor + 256-color paths.** `color_to_rgba` maps `Color::Spec(Rgb { r, g, b }) → [r/255, g/255, b/255, 1.0]`, `Color::Indexed(i) → palette_256[i]`, and `Color::Named(NamedColor)` → palette index or `default_fg`/`default_bg`. The xterm-256 palette is built once at compositor construction (`xterm_256_palette() -> [[f32; 4]; 256]`) — 16 ANSI base + 6×6×6 cube + 24-step grayscale ramp, well-known table cited inline.
- **`Compositor::render_offscreen` + `render_offscreen_with`** ship a surface-free render path for snapshot tests. The `_with` variant takes raw `&Device + &Queue + width + height` so tests can build Device+Queue via `RenderContext::new_offscreen` (also new this plan) without a winit window. Render goes to a self-allocated `Rgba8Unorm` texture; readback uses `copy_texture_to_buffer` with `COPY_BYTES_PER_ROW_ALIGNMENT`-padded staging + `Buffer::map_async` + `device.poll(PollType::wait_indefinitely())`.
- **vector-app wired end-to-end.** `RenderHost::render(&mut Term, selection)` lazy-builds the Compositor on first call (FontStack::load_bundled → Compositor::new). On init failure, the field `compositor_failed` is set and subsequent renders fall back to the Plan-03-01 clear-color path. `app.rs::RedrawRequested` scope-locks `self.term`, calls `host.render(&mut t, None)`, drops the guard — `clippy::await_holding_lock = "deny"` (D-11) satisfied at compile time. `CompositorError::Outdated|Lost` is swallowed because Compositor::render already reconfigures the surface; the next RedrawRequested retries.
- **Surface error recovery (Open Question #4).** Compositor::render's match on `CurrentSurfaceTexture::{Outdated, Lost}` reconfigures the surface in-place via `surface.configure(&device, &config)` and returns the corresponding CompositorError; the caller treats that as `Ok(())` (handled). `Validation` logs + propagates. `Occluded` short-circuits with `Ok(())`. `Timeout` propagates.
- **5 Wave-0 stubs un-ignored:**
  - `damage_to_quads.rs::red_a_cell_paints_red_pixels` — feed `b"\x1b[31mA\x1b[0m"`, offscreen render, assert ≥ 20 red-dominant pixels in the top-row cell strip (r > 150, g < 80, b < 80).
  - `snapshot_singlecell.rs::feeding_single_char_writes_to_grid` — feed `b"X"`, assert `grid[(0,0)].c == 'X'`.
  - `snapshot_truecolor.rs::truecolor_sgr_lands_as_rgb_spec` — feed `b"\x1b[38;2;255;128;0mZ\x1b[0m"`, assert `cell.fg == Color::Spec(Rgb { 255, 128, 0 })`.
  - `snapshot_clearcolor.rs::empty_grid_paints_bg_color` — empty grid, offscreen render, bright pixel count below cursor budget.
  - `cursor_overlay_snapshot.rs::cursor_paints_light_block_in_cursor_cell` — empty grid, assert cell (0,0) center pixel is near light-gray (RGB > 150 each).
- **Workspace test ledger:** baseline (post 03-02) 61 passed / 0 failed / 13 ignored. Post 03-03: **66 passed / 0 failed / 8 ignored.** Net +5 passes / −5 ignored — matches the 5 un-ignored stubs above. Arch-lint `find crates -name no_tokio_main.rs | wc -l` = 15 (unchanged).

## Task Commits

1. **Task 1: Cell pipeline + cell.wgsl + Compositor::render with truecolor/256-color + WIDE_CHAR_SPACER skip + damage consumption** — `9101e29` (feat)
2. **Task 2: Cursor pipeline + cursor.wgsl + offscreen render harness + vector-app wiring + 5 stubs un-ignored** — `746ef60` (feat)
3. **Fixup: CellInstance size doc correction (72 not 80) + compile-time size assertion** — `b35ffad` (fix)

_Plan metadata commit lands separately after this SUMMARY._

## Files Created/Modified

**Created (src):**
- `crates/vector-render/src/cell_pipeline.rs` — CellPipeline + CellInstance Pod struct + new()/rebind_atlas()/ensure_capacity()/upload_instances()/update_uniforms()/draw().
- `crates/vector-render/src/cursor_pipeline.rs` — CursorPipeline (single block-cursor quad) + new()/update()/draw().
- `crates/vector-render/src/compositor.rs` — Compositor::new + new_with (test path) + render + render_offscreen + render_offscreen_with; prepare_frame_raw + encode_passes_raw shared between the two render entrypoints; color_to_rgba (Named/Spec/Indexed branch); xterm_256_palette helper; CompositorError enum.
- `crates/vector-render/src/shaders/cell.wgsl` — vertex + fragment for the cell pipeline (mono/color/empty branch + per-cell selected blend).
- `crates/vector-render/src/shaders/cursor.wgsl` — vertex + fragment for the cursor pipeline (constant cursor_color).
- `crates/vector-render/tests/common/offscreen.rs` — `build_compositor(w, h)` test harness (probes for Metal adapter; returns None on Linux dev shells); `channel_indices(format)` translates wgpu surface format into r/g/b byte offsets.
- `crates/vector-render/tests/fixtures/.gitkeep` — fixtures directory seed (PNG fixtures will land here in future plans).

**Modified:**
- `Cargo.lock` — wgpu transitive resolution refreshed.
- `crates/vector-render/Cargo.toml` — added direct + dev `alacritty_terminal.workspace = true` (compositor uses `Point/Line/Column/Flags/Color/NamedColor/Rgb` types; tests use the same types directly for grid-level asserts).
- `crates/vector-render/src/lib.rs` — extended module tree (`cell_pipeline`, `compositor`, `cursor_pipeline`), pub use `Compositor`, `CompositorError`, `OffscreenFrame`, `CellInstance`, `CursorPipeline`, `CursorInstance`, `Offscreen`.
- `crates/vector-render/src/pipeline.rs` — added `Offscreen` struct + `RenderContext::new_offscreen(w, h)` for headless test paths.
- `crates/vector-render/tests/damage_to_quads.rs` — Wave-0 stub → red-dominant pixel-count assertion.
- `crates/vector-render/tests/snapshot_singlecell.rs` — Wave-0 stub → grid character placement assertion.
- `crates/vector-render/tests/snapshot_truecolor.rs` — Wave-0 stub → `Color::Spec(Rgb)` assertion.
- `crates/vector-render/tests/snapshot_clearcolor.rs` — Wave-0 stub → bright-pixel-count budget assertion.
- `crates/vector-render/tests/cursor_overlay_snapshot.rs` — Wave-0 stub → cursor-cell-center light-gray assertion.
- `crates/vector-app/Cargo.toml` — added `vector-fonts = { path = "../vector-fonts" }` for `FontStack::load_bundled`.
- `crates/vector-app/src/render_host.rs` — replaced clear-only stub with lazy-init Compositor + selection-aware render method; CompositorError::Outdated|Lost auto-recover.
- `crates/vector-app/src/app.rs` — `WindowEvent::RedrawRequested` now locks `self.term`, calls `host.render(&mut t, None)`, drops lock at arm end. `None` is the explicit Plan-03-03-Phase contract — Plan 03-04 will substitute the selection range.

## Decisions Made

- **`CellInstance` is 72 bytes, not 80.** The plan's `<behavior>` block specified "16-byte aligned" but `#[repr(C)]` packs `[u32; 2] + [f32; 4]×3 + u32×4 = 72`. WGSL instance buffers don't require std140-style 16-byte padding; naga validates the layout against our shader's `@location` declarations and accepts 72. Compile-time `const _: () = assert!(size_of::<CellInstance>() == 72);` guards future drift.
- **`xterm_256_palette()` is `Source: xterm 256-color palette` (en.wikipedia.org/wiki/ANSI_escape_code#8-bit; verified against xterm git refs).** 16 ANSI base colors (Black .. BrightWhite — xterm's `cd 00 00` / `e5 e5 e5` family), 6×6×6 cube starting at index 16 (`CUBE_STEPS = [0, 95, 135, 175, 215, 255]`), 24-step grayscale ramp at 232 (`v = 8 + 10·i`). All values cited inline in the function.
- **Selection arg in `Compositor::render` from day one.** Plan 03-04's selection state machine will populate the `Option<((u16,u16),(u16,u16))>` argument; Plan 03-03 callers (app.rs RedrawRequested + the 5 snapshot tests) pass `None`. No signature drift between phases. `is_cell_selected(selection, col, row)` is the helper that maps a row-major bounding box to per-cell hit-testing; selection is inclusive on both endpoints.
- **`CompositorError` replaces wgpu's removed `SurfaceError`.** wgpu 29 returns the `CurrentSurfaceTexture` enum from `Surface::get_current_texture()` rather than `Result<_, SurfaceError>`. We pattern-match it into our local `CompositorError { Outdated, Lost, Timeout, Validation }` so downstream callers (RenderHost, future Phase 4 mux) get a stable type regardless of wgpu's status-renaming churn.
- **Outdated/Lost auto-recovery happens inside Compositor::render.** `Surface::get_current_texture()` returning `Outdated`|`Lost` triggers `surface.configure(&device, &config)` then the Compositor returns the error variant. RenderHost::render's `match` swallows both via `Ok(()) | Err(Outdated|Lost) => Ok(())`. Next RedrawRequested retries cleanly.
- **`clippy::await_holding_lock = "deny"` holds at compile time.** `app.rs::RedrawRequested` has zero `.await` between `let mut t = self.term.lock();` and the end of the arm — the entire render path is synchronous (wgpu submits + presents synchronously; the device.poll in `render_offscreen_with` is in tests, not the live render path).
- **Cursor blink rate decision: always-on block cursor in Plan 03-03.** Per CONTEXT.md "Claude's Discretion — Cursor visuals: block style is conventional; blink rate matches macOS default if simple, otherwise pick a fixed rate (e.g., 530 ms half-period) and move on." Blink + cursor color in a theme uniform both deferred to Plan 03-05 per the Cursor Visuals discretion clause.
- **Test path bypasses winit.** Initial approach tried `EventLoop::create_window` from a test thread, but winit's macOS `Window` requires main-thread construction and tests run in a thread pool. Solution: `RenderContext::new_offscreen` requests a Metal adapter with `compatible_surface: None`; tests build Compositor via `new_with` and render via `render_offscreen_with`. No surface, no window, fully headless on macOS.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] wgpu 29 API drifts from plan snippets**
- **Found during:** Task 1 + Task 2 builds
- **Issue:** The plan reproduced wgpu shader/pipeline snippets that no longer match wgpu 29.0.3:
  - `wgpu::PipelineLayoutDescriptor.push_constant_ranges: &[]` → renamed to `immediate_size: u32` (we use `0`).
  - `wgpu::PipelineLayoutDescriptor.bind_group_layouts: &[&BindGroupLayout]` → now `&[Option<&BindGroupLayout>]` (we wrap each layout in `Some(&layout)`).
  - `wgpu::RenderPipelineDescriptor.multiview: Option<NonZeroU32>` → renamed to `multiview_mask`.
  - `wgpu::FilterMode::Nearest` for `mipmap_filter` field → the field type is now `MipmapFilterMode` (distinct enum), so `MipmapFilterMode::Nearest` is the correct value.
  - `wgpu::PollType::Wait` (as a value) → now a struct variant `Wait { submission_index, timeout }`; we use `wgpu::PollType::wait_indefinitely()` convenience constructor.
  - `wgpu::SurfaceError` → removed in wgpu 29 entirely; we defined a local `CompositorError` enum with the same conceptual variants.
- **Fix:** Rewrote each call site against the 29.0.3 API surface. All changes are mechanical translations; no behavioral semantics changed.
- **Files modified:** `crates/vector-render/src/cell_pipeline.rs`, `crates/vector-render/src/cursor_pipeline.rs`, `crates/vector-render/src/compositor.rs`
- **Verification:** `cargo build -p vector-render` clean; `cargo test -p vector-render --tests` passes 5 new tests.
- **Committed in:** `9101e29` (cell-pipeline drifts) and `746ef60` (cursor-pipeline + offscreen drifts)

**2. [Rule 1 - Bug] Headless test path cannot build a winit `Window` from a test thread on macOS**
- **Found during:** Task 2 (initial test harness wired through `EventLoop::create_window`)
- **Issue:** winit 0.30's macOS NSWindow construction must happen on the main thread. `cargo test` runs each integration test binary on the main thread of its own process but tests within a binary run on a thread pool by default — and even with `--test-threads=1`, the first `EventLoop::new()` + `create_window` panics outside of an `ApplicationHandler::resumed` callback in newer winit releases.
- **Fix:** Added `RenderContext::new_offscreen(w, h)` that builds Device+Queue via `Adapter::request_device` with `compatible_surface: None`, plus `Compositor::new_with(device, queue, format, w, h, font_stack)` to build the compositor without a `RenderContext`-with-real-surface. `Compositor::render_offscreen_with` takes raw device+queue+w+h and skips the surface acquisition entirely.
- **Files modified:** `crates/vector-render/src/pipeline.rs` (+Offscreen + new_offscreen), `crates/vector-render/src/compositor.rs` (+new_with + render_offscreen_with + prepare_frame_raw + encode_passes_raw), `crates/vector-render/tests/common/offscreen.rs`
- **Verification:** All 3 pixel-snapshot tests pass headless on macOS without instantiating a window.
- **Committed in:** `746ef60`

**3. [Rule 1 - Bug] Plan-stated CellInstance size "16-byte aligned (size = 80)" was wrong**
- **Found during:** Wrote a compile-time assertion to enforce the boundary, then read the actual `repr(C)` size.
- **Issue:** Plan body said the layout would be 16-byte aligned (implying size = 80 or similar multiple). Actual `[u32; 2] + [f32; 4]×3 + u32×4` packs to 72 bytes with no internal padding because all fields are 4-byte-aligned scalars/arrays. WGSL instance buffers don't require 16-byte stride padding; naga accepts 72.
- **Fix:** Corrected the doc comment ("72 bytes per instance") and replaced the silent `let _ = [(); size_of % 16];` with a real `const _: () = assert!(size_of::<CellInstance>() == 72);` that fails the build if the layout ever drifts.
- **Files modified:** `crates/vector-render/src/cell_pipeline.rs`, `crates/vector-render/src/compositor.rs`
- **Verification:** `cargo build -p vector-render` clean; assertion is enforced.
- **Committed in:** `b35ffad`

**4. [Rule 1 - Bug] Multiple clippy pedantic lints (cast_precision_loss, too_many_lines, similar_names, items_after_statements, bool_to_int_with_if, cast_possible_truncation, manual_let_else, many_single_char_names, match_same_arms, unnecessary_cast)**
- **Found during:** Task 1 + Task 2 clippy passes
- **Issue:** Workspace `clippy::pedantic = warn` rolled to `-D warnings` flags many otherwise-acceptable patterns:
  - `u32 as f32` for viewport math (cast_precision_loss) — values fit comfortably in f32 mantissa
  - Both `prepare_frame_raw` (the cell-instance builder) and `encode_passes_raw` had > 100 lines (too_many_lines)
  - The xterm_256_palette helper had constants inside the function body (items_after_statements)
  - `if x { 1 } else { 0 }` → `u32::from(x)` for selected packing
  - The render-failure match arms `Ok(())` and `Err(Outdated|Lost) => Ok(())` (match_same_arms) — collapsed into a `Ok(()) | Err(Outdated|Lost) => Ok(())`
  - Local single-char names `r/g/b/x/y` in cursor + damage pixel-asserts (many_single_char_names) — kept the names but added `#[allow]`
  - `let total = (w*h) as u32` where `w` and `h` are already u32 (unnecessary_cast)
  - Plan's `match Option { Some(x) => x, None => return }` → `let Some(x) = … else { return };` (manual_let_else)
- **Fix:** Module-level `#![allow(clippy::cast_precision_loss, too_many_lines, similar_names, items_after_statements)]` in compositor.rs + `#![allow(clippy::too_many_lines, default_trait_access, dead_code)]` in cell_pipeline.rs + `#![allow(clippy::too_many_lines, default_trait_access)]` in cursor_pipeline.rs. Per-call-site `#[allow(clippy::many_single_char_names)]` on the two pixel-assert tests. Mechanical conversions for the others (`u32::from(bool)`, `let-else`, removing redundant casts, collapsing identical match arms).
- **Files modified:** `crates/vector-render/src/compositor.rs`, `crates/vector-render/src/cell_pipeline.rs`, `crates/vector-render/src/cursor_pipeline.rs`, `crates/vector-render/tests/{damage_to_quads, snapshot_clearcolor, cursor_overlay_snapshot}.rs`, `crates/vector-app/src/render_host.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` clean.
- **Committed in:** `9101e29` (Task 1 set), `746ef60` (Task 2 set)

**5. [Rule 1 - Documentation/test scope drift] Plan referenced `selection_overlay_snapshot.rs` deferral to Plan 03-04**
- **Found during:** Task 2 acceptance criteria pass
- **Issue:** Plan body in places implies 4 or 5 Wave-0 stubs un-ignored in Plan 03-03, but the Wave-0 stub map in 03-01-SUMMARY assigns `selection_overlay_snapshot.rs` to Plan 03-04. We left it `#[ignore = "Wave-0 stub"]`.
- **Fix:** None needed — Plan 03-04 owns the selection state machine. Plan 03-03 ships the rendering path (per-cell `selected` flag in CellInstance, `selection_tint` blend in cell.wgsl, `is_cell_selected` hit-test in compositor.rs) so Plan 03-04 only needs to populate the selection range.
- **Files modified:** N/A (documentation, not code)
- **Verification:** `selection_overlay_snapshot` still `ignored, Wave-0 stub`; 5 other stubs newly green.
- **Committed in:** N/A (intentional deferral)

---

**Total deviations:** 4 code auto-fixes (Rule 1 — all API drift / lint compliance / size-doc correctness) + 1 intentional scope deferral (selection_overlay_snapshot left for Plan 03-04). 0 Rule 4 architectural decisions. No scope creep.

**Impact on plan:** All four code fixes are mechanical corrections. Plan's behavioral contract (RENDER-01: damage-tracked rendering via Term::damage()/reset_damage(); RENDER-04: 24-bit truecolor + 256-color via Color::Spec(Rgb) / Color::Indexed(u8); RENDER-05: cursor over live grid) is met exactly. Plus the bonus contract: per-cell `selected` bit is wired through CellInstance → vertex stage → fragment stage with a uniform `selection_tint` blend, so Plan 03-04 just needs to populate the selection range.

## Issues Encountered

None beyond the deviations above. The wgpu 29 API surface required mechanical translation from the plan snippets; the offscreen test harness required a small constructor addition (`new_offscreen` + `new_with`) to skip winit. The pixel-snapshot asserts use loose thresholds (e.g., "≥ 20 red-dominant pixels") rather than committing PNG fixtures — deterministic enough to gate CI without inflating the repo size; PNG fixtures land in `tests/fixtures/` in future plans.

## User Setup Required

None. The compositor uses the bundled JetBrains Mono from Plan 03-02; no external configuration required.

## Hand-off Notes

**Plan 03-04 (input):**
- `Compositor::render` signature already takes `selection: Option<((u16, u16), (u16, u16))>` — Plan 03-04 populates this from the selection state machine. `((col_anchor, row_anchor), (col_cursor, row_cursor))` is the contract; `is_cell_selected` does the row-major inclusive bounding box check (anchor ≤ cell ≤ cursor in (row, col) lex order, swapping if necessary).
- `selection_overlay_snapshot.rs` is still `#[ignore = "Wave-0 stub"]`. Plan 03-04 fills it once it has a selection range — pattern matches the other snapshot tests (`offscreen::build_compositor`, render with selection populated, assert tint-shifted pixels at the selected cell rectangle).
- `vector-app/src/app.rs::WindowEvent::RedrawRequested` passes `None` for selection. Plan 03-04 replaces that with `self.input_bridge.selection.range().map(|r| (r.anchor, r.cursor))` (or whatever shape the input crate ships).
- The CellInstance shader inputs include a `flags: u32` field (bit 0 = inverse, bit 1 = bold reserved). Plan 03-04 can add bits 2..31 for underline/strikethrough/etc. without changing the layout.

**Plan 03-05 (pacing + polish):**
- `Compositor::atlas_mut() -> &mut Atlas` is the public accessor — `ScaleFactorChanged` calls `compositor.atlas_mut().clear_all()` (Plan 03-02 already shipped `Atlas::clear_all()`) and the next-frame glyph rasterizations re-populate at the new DPR.
- `CompositorError::{Outdated, Lost}` already auto-reconfigures the surface; Plan 03-05's pacing pass can use the same retry-once pattern if it wires the device.poll throttle.
- Cursor blink: `CursorPipeline::update(queue, cursor_cell, cell_size, viewport, cursor_color)` accepts a per-frame `cursor_color`. Plan 03-05 toggles between the lit color and the bg color on a 530 ms half-period (or matches macOS's blink rate via NSUserDefaults).
- Damage-driven partial buffer rewrites: `prepare_frame_raw` snapshots damage into `_damage_rows: Vec<(u16, u16, u16)>` but currently does a full rebuild. Plan 03-05's pacing pass can wire row-slice writes via `cell_pipeline.upload_instances(&queue, &row_instances, row_offset)` if profiling against `cat large.log` shows the full rebuild costs > 1 ms.
- Theme uniform: `default_fg`, `default_bg`, `selection_tint`, `cursor_color` are all stored on `Compositor` as `[f32; 4]` fields. Plan 03-05 can collapse them into a single uniform buffer with setters.

**Plan 04 (mux):**
- `Compositor::new_with` is the surface-agnostic constructor — Phase 4's per-pane Compositor instances can share a single Device+Queue but each owns its own atlas + pipelines + scratch.
- Atlas is `!Sync` (`HashMap` mutation through `&mut self`); each pane's Compositor owns its own atlas. Sharing a single atlas across panes is a Phase 5+ optimization, not required for correctness.

## Self-Check: PASSED

- FOUND: `crates/vector-render/src/cell_pipeline.rs`
- FOUND: `crates/vector-render/src/cursor_pipeline.rs`
- FOUND: `crates/vector-render/src/compositor.rs`
- FOUND: `crates/vector-render/src/shaders/cell.wgsl`
- FOUND: `crates/vector-render/src/shaders/cursor.wgsl`
- FOUND: `crates/vector-render/tests/common/offscreen.rs`
- FOUND: `crates/vector-render/tests/fixtures/.gitkeep`
- FOUND: `.planning/phases/03-gpu-renderer-first-paint/03-03-SUMMARY.md`
- FOUND commit `9101e29` (Task 1: cell pipeline + Compositor)
- FOUND commit `746ef60` (Task 2: cursor + offscreen + vector-app wiring)
- FOUND commit `b35ffad` (Fixup: CellInstance size doc + compile-time assertion)
- Wave-0 stubs un-ignored: 5 (damage_to_quads, snapshot_singlecell, snapshot_truecolor, snapshot_clearcolor, cursor_overlay_snapshot)
- Wave-0 stubs still ignored: 8 (selection_overlay_snapshot → 03-04; xterm_key_table → 03-04; bracketed_paste_wrap → 03-04; selection_render → 03-04; dpr_change_invalidates → 03-05; pty_coalesce → 03-05; idle_no_redraw → 03-05; frame_pacing → 03-05)
- Arch-lint: 15 `no_tokio_main.rs` files (15==15 invariant holds)
- Workspace: 66 passed / 0 failed / 8 ignored (vs. baseline 61/0/13; net +5 passes / −5 ignored)
- `clippy::await_holding_lock = "deny"` satisfied at compile time (no `.await` in the render path)

---
*Phase: 03-gpu-renderer-first-paint*
*Completed: 2026-05-11*
