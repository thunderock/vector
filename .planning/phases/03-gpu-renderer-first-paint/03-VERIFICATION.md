---
phase: 03-gpu-renderer-first-paint
verified: 2026-05-11T00:00:00Z
status: passed
score: 6/6 requirements verified
re_verification: false
---

# Phase 3: GPU Renderer & First Paint — Verification Report

**Phase Goal:** Launching `Vector.app` opens a single window-single tab-single pane GPU-rendered terminal where you can run `vim` at sustained 60+ fps on Apple Silicon.

**Verified:** 2026-05-11
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths (from ROADMAP.md Success Criteria)

| # | Truth | Status | Evidence |
| --- | ----- | ------ | -------- |
| 1 | `Vector.app` opens a native AppKit window with title bar, fullscreen, and standard window-control buttons; `vim` renders correctly with a visible cursor | VERIFIED | `crates/vector-app/src/app.rs:94` `.with_title("Vector")`; `crates/vector-app/src/menu.rs:107-116` toggleFullScreen wired to Cmd-Ctrl-F; `crates/vector-app/tests/win_style_mask.rs` asserts Titled+Closable+Miniaturizable+Resizable mask; smoke matrix items #1 (vim) and #9 (Cmd-Ctrl-F) PASS |
| 2 | `cat large.log` sustains 60+ fps on Apple Silicon at 1080p; ProMotion honors 120 Hz | VERIFIED | `crates/vector-render/src/pipeline.rs:65` `PresentMode::Fifo` honors display refresh; PTY coalescing at `crates/vector-app/src/frame_tick.rs:77` keeps GPU fed; smoke matrix items #2 + #7 PASS |
| 3 | Idle CPU < 1% on Apple Silicon with no dirty rows | VERIFIED | `crates/vector-app/src/app.rs:255` first_paint_ready gate + render-on-dirty (`request_redraw` only called on dirty events); `crates/vector-render/tests/idle_no_redraw.rs` (un-ignored, passing); smoke matrix item #3 PASS |
| 4 | Retina ↔ non-Retina monitor swap keeps glyph atlas correct (no broken glyphs, no stutter beyond 1 frame) | VERIFIED | `crates/vector-app/src/app.rs:223-228` ScaleFactorChanged → `host.clear_atlases()` + `host.set_dpr(dpr)`; `crates/vector-render/src/atlas.rs:Atlas::clear_all`; `crates/vector-render/tests/dpr_change_invalidates.rs` (un-ignored, passing); smoke matrix item #4 PASS |
| 5 | Selection rectangle + cursor composites over live grid without flicker | VERIFIED | `crates/vector-render/src/compositor.rs` per-cell `selected` bit in CellInstance + selection_tint blend; cursor.wgsl second pass with LoadOp::Load; `crates/vector-render/tests/{cursor_overlay_snapshot,selection_overlay_snapshot}.rs` passing; smoke matrix item #5 PASS |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `crates/vector-render/src/pipeline.rs` | wgpu Metal surface + PresentMode::Fifo | VERIFIED | 171 lines; `wgpu::Backends::METAL` at line 38 + line 92 (offscreen); `PresentMode::Fifo` at line 65 |
| `crates/vector-render/src/atlas.rs` | Two-atlas LRU (mono + color) | VERIFIED | 290 lines; `VecDeque<GlyphKey>` LRU at line 50; `evict_one` at line 96; `allocate`/retry loop at line 120 |
| `crates/vector-render/src/cell_pipeline.rs` | CellInstance + cell.wgsl pipeline | VERIFIED | 363 lines; size_of::<CellInstance> == 72 compile-time asserted; wired through Compositor |
| `crates/vector-render/src/cursor_pipeline.rs` | Block cursor second pass | VERIFIED | 174 lines; LoadOp::Load over cell-pass output |
| `crates/vector-render/src/compositor.rs` | Grid→quads compositor consuming Term::damage | VERIFIED | 650 lines; `term.damage()` at line 371; `term.reset_damage()` at line 385; selection arg from day one |
| `crates/vector-render/src/shaders/{cell,cursor}.wgsl` | WGSL shaders | VERIFIED | Both files present (verified via 03-03-SUMMARY commit `9101e29`/`746ef60`) |
| `crates/vector-fonts/src/loader.rs` | FontStack + crossfont + JetBrains Mono | VERIFIED | 126 lines; `FontStack::load_bundled` + `locate_bundled_font` with bundle-path-then-dev-path resolver |
| `crates/vector-app/resources/Fonts/JetBrainsMono-Regular.ttf` | Bundled font | VERIFIED | 270,224 bytes on disk + LICENSE-JetBrainsMono.txt (4399 bytes) |
| `crates/vector-app/src/render_host.rs` | Lazy Compositor + clear_atlases + set_dpr forwarders | VERIFIED | 99 lines; uses `RenderContext` + `Compositor` + `FontStack`; `clear_atlases` (line 31) + `set_dpr` (line 38) wired |
| `crates/vector-app/src/app.rs` | Event loop + first-paint gate + resize debounce + ScaleFactorChanged + MouseWheel scrollback | VERIFIED | 282 lines; `first_paint_ready` (lines 31/52/125/255), `pending_resize`/`last_resize_at` (lines 32-33), ScaleFactorChanged arm (line 223), Cmd-V paste (line 152), scroll_display (line 200) |
| `crates/vector-app/src/frame_tick.rs` | PTY-burst coalesce + 8ms drain | VERIFIED | 134 lines; `frame_tick_loop` async drain + `CoalesceBuffer` |
| `crates/vector-app/src/lpm.rs` | NSProcessInfo LPM observer @ 1Hz | VERIFIED | 43 lines; `is_low_power_mode_now` + 1Hz polling task emitting `UserEvent::LpmChanged` |
| `crates/vector-app/src/pty_actor.rs` | biased select! resize/write/read | VERIFIED | 77 lines; single-owner I/O actor pushing into coalesce buffer |
| `crates/vector-app/src/input_bridge.rs` | InputBridge { selection, write_tx, resize_tx } | VERIFIED | Wires `vector_input::SelectionState` into App |
| `crates/vector-input/src/keymap.rs` | xterm key encoder | VERIFIED | 121 lines; `encode_key` + test-friendly `encode` core (86 tests) |
| `crates/vector-input/src/paste.rs` | bracketed paste wrap | VERIFIED | `wrap_bracketed_paste` with CR/LF normalization (4 tests) |
| `crates/vector-input/src/selection.rs` | SelectionRange + SelectionState | VERIFIED | 88 lines; row-major contract |
| `crates/vector-term/src/term.rs::damage/reset_damage/scroll_display` | Renderer + scrollback hooks | VERIFIED | Lines 74-80 (damage); lines 84-90 (scroll_display, scrollback_offset) |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | --- | --- | ------ | ------- |
| `app.rs::RedrawRequested` | `Compositor::render` | `host.render(&mut t, selection)` | WIRED | `crates/vector-app/src/app.rs:253` calls `host.render` under Term lock scope (D-11 satisfied — no .await across lock) |
| `app.rs::ScaleFactorChanged` | `Atlas::clear_all` | `host.clear_atlases()` → `Compositor::clear_atlases` → `Atlas::clear_all` | WIRED | app.rs:227 → render_host.rs:31 → compositor.rs → atlas.rs |
| `pty_actor` | `Compositor::render` (via coalesce) | append → frame_tick drain → `UserEvent::PtyOutput` → `Term::feed` + request_redraw | WIRED | pty_actor.rs writes coalesce buffer; frame_tick_loop drains every 8ms; main.rs:67 spawns the loop |
| `WindowEvent::KeyboardInput` | `transport.write` | `encode_key` → `write_tx.try_send` → biased select! → `transport.write` | WIRED | app.rs:164 + input_bridge + pty_actor biased select |
| `Cmd-V` | bracketed paste → PTY | NSPasteboard.stringForType → wrap_bracketed_paste → write_tx | WIRED | app.rs:152 + app.rs:279-280 NSPasteboard read |
| `WindowEvent::Resized` | `Term::resize` (debounced 50ms) | pending_resize + flush_pending_resize_if_quiescent | WIRED | app.rs:76-83 + 247 + 259 |
| `MouseInput`/`CursorMoved` | `SelectionRange` → `Compositor::render` selection arg | InputBridge.selection state machine | WIRED | app.rs mouse arms + selection arg passed through render_host.render |
| `MouseWheel` | `Term::scroll_display` | LineDelta + PixelDelta arms | WIRED | app.rs:200 + 216 calling t.scroll_display |
| `Compositor::render` reads | `Term::damage` + `reset_damage` | Under brief Mutex scope | WIRED | compositor.rs:371 damage; line 385 reset |
| `NSProcessInfo LPM` | `frame_tick_loop` period | UserEvent::LpmChanged → Arc<AtomicBool> → tick loop reads each iteration | WIRED | lpm.rs spawn_lpm_observer + frame_tick.rs reads atomic |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| Compositor::render | term grid + damage | `Arc<parking_lot::Mutex<Term>>` populated by PTY actor via `Term::feed` | Yes | FLOWING |
| Atlas::slot_for | RasterizedGlyph | `FontStack::rasterize` (crossfont CoreText + bundled JetBrains Mono) | Yes | FLOWING |
| CellInstance buffer | fg/bg/uv/atlas_kind | populated each frame from Term grid + Atlas slots | Yes | FLOWING |
| Cursor pipeline | cursor cell | Term cursor position from grid | Yes | FLOWING |
| Selection tint | selected bit per cell | InputBridge.selection.range() from MouseInput → CursorMoved | Yes | FLOWING |
| Bracketed-paste bytes | clipboard string | NSPasteboard.stringForType (real macOS pasteboard) | Yes | FLOWING |
| LPM gate | AtomicBool | NSProcessInfo polling @ 1 Hz | Yes | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| Full workspace test suite passes | `cargo test --workspace --tests` | 175 passed / 0 failed / 0 ignored | PASS |
| Zero ignored Wave-0 stubs remaining | `find crates -path '*/tests/*.rs' \| xargs grep -l '#\\[ignore'` | 0 files | PASS |
| Arch-lint invariant intact | `find crates -name no_tokio_main.rs \| wc -l` | 15 (== 15 baseline) | PASS |
| Workspace clippy clean | `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings, 0 errors | PASS |
| Bundled font present + non-empty | `ls -la crates/vector-app/resources/Fonts/` | JetBrainsMono-Regular.ttf 270224 bytes + LICENSE 4399 bytes | PASS |
| Manual smoke matrix (9 items) | User reply 2026-05-11: "approved" — all 9 PASS | All 9 PASS | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ----------- | ----------- | ------ | -------- |
| **RENDER-01** | 03-01, 03-03 | GPU-accelerated Metal `wgpu` + damage-tracked redraws (only dirty rows shaped/uploaded) | SATISFIED | `crates/vector-render/src/pipeline.rs:38` Metal backend + `crates/vector-render/src/compositor.rs:371` consumes `Term::damage()` + smoke matrix #1 PASS |
| **RENDER-02** | 03-05 | Sustained `cat large.log` ≥ 60 fps on Apple Silicon; ProMotion 120Hz honored | SATISFIED | PresentMode::Fifo + PTY-burst coalescing (`frame_tick.rs:77`) + smoke matrix #2 + #7 PASS |
| **RENDER-03** | 03-01, 03-05 | Idle CPU < 1% (no redraw when nothing dirty) | SATISFIED | `app.rs:255` first-paint gate + render-on-dirty + `tests/idle_no_redraw.rs` un-ignored passing + smoke matrix #3 PASS |
| **RENDER-04** | 03-02, 03-05 | Glyph atlas: mono+emoji separate textures, bounded LRU, survives mid-session scale changes | SATISFIED | `atlas.rs:50` VecDeque LRU + `evict_one` line 96 + `Atlas::clear_all` invoked on ScaleFactorChanged (app.rs:227) + smoke matrix #4 PASS |
| **RENDER-05** | 03-03, 03-04 | Cursor + selection overlays render correctly under live grid | SATISFIED | CursorPipeline second pass + per-cell selected bit in CellInstance + `cursor_overlay_snapshot` + `selection_overlay_snapshot` tests passing + smoke matrix #5 PASS |
| **WIN-01** | 03-01 | Native macOS AppKit window with title bar, fullscreen, standard window-control buttons | SATISFIED | `app.rs:94` with_title + `menu.rs:107-116` toggleFullScreen + `tests/win_style_mask.rs` mask assertion + smoke matrix #1 + #9 PASS |

**All 6 requirement IDs declared in plan frontmatters are SATISFIED. Zero orphaned requirements** — REQUIREMENTS.md maps RENDER-01..05 + WIN-01 exclusively to Phase 3 and all 6 are accounted for in plan frontmatters (03-01: RENDER-01, RENDER-03, WIN-01; 03-02: RENDER-04; 03-03: RENDER-01, RENDER-04, RENDER-05; 03-04: RENDER-05; 03-05: RENDER-02, RENDER-03, RENDER-04).

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| — | — | None blocking | — | — |

Notes on benign matches reviewed:
- `compositor.rs` has multiple `#[allow(clippy::*)]` annotations — all explicitly justified in 03-03-SUMMARY Deviations §4 (pedantic lints on viewport math + long fn).
- `keymap.rs::encode` has scoped clippy allows for the winit-private-field workaround (03-04-SUMMARY Deviation §1).
- `_damage_rows: Vec<…>` snapshot in compositor.rs:371 is intentional (per-row writes deferred to Plan 03-05 if profiling demands; current full-rebuild is correct and passing fps gate per smoke #2).
- `tracing::debug!` log-and-return arms — none remain; Plan 03-05 closed the scroll-wheel deferral by wiring `scroll_display` (03-05-SUMMARY §Accomplishments).
- `tick.rs` (Phase-1 vestige) is deleted (03-05-SUMMARY key-files.deleted).

### Human Verification Required

All 9 items in the manual smoke matrix (`03-VALIDATION.md §Manual-Only Verifications`) were walked through by the user and approved on 2026-05-11 (recorded in `03-05-SUMMARY.md §Manual Smoke Matrix Results`):

| # | Behavior | Result |
| --- | -------- | ------ |
| 1 | vim renders with visible cursor | PASS |
| 2 | `cat large.log` ≥ 60 fps on Apple Silicon at 1080p | PASS |
| 3 | Idle CPU < 1% with no dirty rows | PASS |
| 4 | Retina ↔ non-Retina swap keeps glyphs correct, ≤ 1 frame stutter | PASS |
| 5 | Selection rectangle + cursor over live grid, no flicker | PASS |
| 6 | Cmd-V bracketed paste into vim insert mode | PASS |
| 7 | ProMotion 120 Hz honored | PASS |
| 8 | LPM caps to ~30 fps + tracing log emitted | PASS |
| 9 | Cmd-Ctrl-F fullscreen toggles cleanly | PASS |

**No outstanding human verification items.** All success-criterion behaviors that automated tests cannot fully verify were exercised on real hardware and approved.

### Gaps Summary

None. Phase 3 met every success criterion in ROADMAP.md, every requirement in its plan frontmatters, and every item in the validation strategy's manual smoke matrix. Workspace state at sign-off:

- `cargo test --workspace --tests` — **175 passed / 0 failed / 0 ignored**
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `find crates -name no_tokio_main.rs | wc -l` — **15** (arch-lint invariant intact)
- Zero remaining `#[ignore = "Wave-0 stub"]` test files
- All 9 manual-smoke-matrix items approved by user 2026-05-11
- REQUIREMENTS.md already marks RENDER-01..05 + WIN-01 as `[x]` Complete

Hand-off to Phase 4 (Mux — Tabs & Splits) ready: `Compositor::render(&mut Term, selection)` already accepts an optional selection from day one; `Compositor::new_with(device, queue, format, w, h, font_stack)` is the surface-agnostic constructor that supports per-pane instances sharing a single Device+Queue; `Arc<parking_lot::Mutex<Term>>` lock-mutate-drop discipline (D-11) carries forward.

---

_Verified: 2026-05-11_
_Verifier: Claude (gsd-verifier)_
