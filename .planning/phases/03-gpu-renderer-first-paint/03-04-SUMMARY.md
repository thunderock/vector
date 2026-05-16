---
phase: 03-gpu-renderer-first-paint
plan: 04
subsystem: input
tags: [winit, xterm-keymap, bracketed-paste, nspasteboard, mpsc, tokio-select, selection-overlay, objc2-app-kit]

requires:
  - phase: 03-01
    provides: pty_actor (single-owner I/O actor), RenderContext, UserEvent skeleton
  - phase: 03-02
    provides: FontStack (used transitively via RenderHost::cell_metrics_px)
  - phase: 03-03
    provides: Compositor::render(term, selection) with per-cell selected bit + selection_tint
provides:
  - vector-input::encode/encode_key (xterm key table, D-52)
  - vector-input::wrap_bracketed_paste (D-53)
  - vector-input::SelectionRange + SelectionState (D-54)
  - vector-app::input_bridge::InputBridge (write_tx + resize_tx + selection)
  - pty_actor biased select! over resize/write/read (extends Plan 02-05 actor)
  - UserEvent::Resized { rows, cols } variant (SIGWINCH round-trip path)
  - RenderHost::cell_metrics_px (pixel → cell coord conversion for input)
  - read_clipboard() via NSPasteboard.generalPasteboard().stringForType
affects: [03-05]

tech-stack:
  added: [vector-input (real impl), winit::keyboard::ModifiersState, NSPasteboard read path]
  patterns:
    - "encode_key wraps a thin encode(&Key, Option<&str>, ElementState, ModState) core to dodge winit 0.30 KeyEvent's private platform_specific field in tests"
    - "Compositor stays dep-free of vector-input by duplicating the row-major SelectionRange::cells logic locally"
    - "PTY actor biased select!: resize > write > read so SIGWINCH isn't starved"
    - "Cell coords derived from PhysicalPosition + RenderHost::cell_metrics_px; saturating u16 casts for very large windows"
    - "Drop-on-full write_tx try_send so keystroke handling never blocks main thread"

key-files:
  created:
    - crates/vector-input/src/keymap.rs
    - crates/vector-input/src/mods.rs
    - crates/vector-input/src/paste.rs
    - crates/vector-input/src/selection.rs
    - crates/vector-app/src/input_bridge.rs
  modified:
    - crates/vector-input/Cargo.toml
    - crates/vector-input/src/lib.rs
    - crates/vector-app/Cargo.toml
    - crates/vector-app/src/app.rs
    - crates/vector-app/src/main.rs
    - crates/vector-app/src/pty_actor.rs
    - crates/vector-app/src/render_host.rs
    - crates/vector-render/src/compositor.rs
    - crates/vector-input/tests/xterm_key_table.rs
    - crates/vector-input/tests/bracketed_paste_wrap.rs
    - crates/vector-render/tests/selection_overlay_snapshot.rs
    - crates/vector-app/tests/selection_render.rs

key-decisions:
  - "Tests call vector_input::encode directly (private platform_specific field on winit::KeyEvent blocks struct-literal construction outside winit)"
  - "Selection cells enumerated row-major (anchor → EOL → middle rows full → BOL → cursor), matching xterm/macOS text-selection convention"
  - "Scroll-wheel deferred to Plan 03-05 (vector-term wrapper doesn't expose Term::scroll_display; PixelDelta + LineDelta both logged as debug)"
  - "Compositor duplicates SelectionRange::cells contract inline rather than depending on vector-input — keeps render dep edges flat"

patterns-established:
  - "Pattern: winit private-field workaround — expose test-friendly thin core (encode) alongside the user-facing helper (encode_key)"
  - "Pattern: biased tokio::select! ordering in I/O actor — resize > write > read"

requirements-completed: [RENDER-05]

duration: 35m
completed: 2026-05-11
---

# Phase 3 Plan 4: Input Pipeline Summary

**xterm keymap + bracketed paste + click-drag selection rendering; winit input flows main → mpsc → I/O actor → transport.write; SelectionRange lights up the per-cell selected bit through Compositor::render.**

## Performance

- **Duration:** ~35 min
- **Tasks:** 2
- **Files created:** 5
- **Files modified:** 12

## Accomplishments

- `vector-input` filled in: `encode_key`/`encode`, `wrap_bracketed_paste`, `SelectionRange`/`SelectionState`.
- 86 xterm key-table tests cover the four arrows × 8 mod combos (32), F1–F12 base + 4 modified, Home/End/PgUp/PgDn/Insert/Delete × no-mod and 1 modifier, Esc/Tab/Shift-Tab/Backspace/Enter/Space, 8 Ctrl chords, 5 Option chords, 4 plain-char (incl. CJK), 3 released/unmapped negatives.
- 4 bracketed-paste tests pass (ASCII, empty, CRLF→LF, lone CR→LF).
- `vector-app::pty_actor` now uses `tokio::select! { biased; ... }` over resize / write / read receivers. Resize prioritized per Plan 02-05 hand-off (SIGWINCH starvation avoided).
- `UserEvent::Resized { rows, cols }` round-trips: window resize → mpsc → actor calls `transport.resize` → proxy sends `UserEvent::Resized` back → main locks `Term`, resizes grid.
- App handles `ModifiersChanged`, `KeyboardInput` (encode → write_tx), `MouseInput Left` (selection mouse_down/up), `CursorMoved` (drag mouse_move + redraw), `MouseWheel` (logged, deferred to Plan 03-05), `Resized` (cell-metric-driven cols/rows propagation).
- `Cmd-V` reads the macOS pasteboard via `NSPasteboard::generalPasteboard().stringForType(NSPasteboardTypeString)` and wraps via `wrap_bracketed_paste`.
- `Compositor` per-cell `selected` flag now derives from a row-major selection contract (was a bounding box in Plan 03-03).
- `selection_render` (vector-app) un-ignored: 6 contract tests for `SelectionState` transitions and `SelectionRange::cells`.
- `selection_overlay_snapshot` (vector-render) un-ignored: pixel-readback asserts the blue selection tint dominates red and out-blues unselected cells.

## Task Commits

1. **Task 1: vector-input — keymap + paste + selection types + tests** — `fc506e7` (feat)
2. **Task 2: wire vector-input into vector-app + compositor + tests** — `6aac789` (feat)

## Files Created/Modified

- `crates/vector-input/src/keymap.rs` — `encode_key` + test-friendly `encode` core
- `crates/vector-input/src/mods.rs` — `ModState` from `winit::ModifiersState`
- `crates/vector-input/src/paste.rs` — `wrap_bracketed_paste` with CR/LF normalization
- `crates/vector-input/src/selection.rs` — `SelectionRange` + `SelectionState`
- `crates/vector-input/Cargo.toml` — added `winit.workspace = true`
- `crates/vector-input/src/lib.rs` — exports
- `crates/vector-input/tests/xterm_key_table.rs` — 86 test cases (was Wave-0 stub)
- `crates/vector-input/tests/bracketed_paste_wrap.rs` — 4 test cases (was Wave-0 stub)
- `crates/vector-app/Cargo.toml` — added `vector-input = { path = "../vector-input" }`
- `crates/vector-app/src/input_bridge.rs` — `InputBridge { selection, write_tx, resize_tx }`
- `crates/vector-app/src/app.rs` — full input pipeline + clipboard read + cell-from-pixel
- `crates/vector-app/src/main.rs` — `UserEvent::Resized`, mpsc channels, `App::new(write_tx, resize_tx)`
- `crates/vector-app/src/pty_actor.rs` — biased `tokio::select!` over resize/write/read
- `crates/vector-app/src/render_host.rs` — added `cell_metrics_px(&self)`
- `crates/vector-render/src/compositor.rs` — `is_cell_selected` rewritten to row-major
- `crates/vector-render/tests/selection_overlay_snapshot.rs` — pixel-readback assertion (was Wave-0 stub)
- `crates/vector-app/tests/selection_render.rs` — 6 contract tests (was Wave-0 stub)

## Decisions Made

- **`encode` core alongside `encode_key`.** `winit::event::KeyEvent` has a private `platform_specific` field, so unit tests can't construct it via struct literal. Solution: expose `encode(&Key, Option<&str>, ElementState, ModState) -> Option<Vec<u8>>` as the test entry point, with `encode_key(&KeyEvent, ModState)` as a one-line forwarder. The 86 keymap tests call `encode` directly.
- **Row-major selection contract.** `SelectionRange::cells` (in vector-input) and `is_cell_selected` (in vector-render) both implement: partial first row from anchor to EOL → all intervening rows full-width → partial last row from BOL to cursor. Matches macOS Terminal / iTerm selection feel. Single-row selections degenerate to anchor..=cursor (column range).
- **Compositor stays vector-input-free.** Mirroring the row-major logic inline in `compositor.rs` keeps the dep graph flat. Documented in a comment near `is_cell_selected`.
- **Scroll wheel deferred to Plan 03-05.** `vector-term` doesn't expose `Term::scroll_display` (alacritty's `Term` has it but our wrapper doesn't surface it). Both `MouseScrollDelta::LineDelta` and `PixelDelta` arms log at `tracing::debug` and return. Plan 03-05 ratifies the surface + wiring.
- **Drop-on-full write channel.** `mpsc::Sender::try_send` for both keystroke bytes and resize events — main thread never blocks. Channel sized 64 (writes) / 8 (resizes) — generous given typical typing cadence; warn-logged on full.
- **Cmd is not an xterm modifier.** `ModState::xterm_mod_param` only mixes Shift/Alt/Ctrl. Cmd routes to app shortcuts (Cmd-V handled in `app.rs`; Cmd-C deferred per D-53; Cmd-W via menu).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] winit 0.30 KeyEvent has a private field — tests can't struct-literal it**
- **Found during:** Task 1 (xterm_key_table.rs initial build)
- **Issue:** `KeyEvent { physical_key, logical_key, text, location, state, repeat }` failed to compile because `platform_specific: KeyEventExtra` is `pub(crate)`. The plan's test scaffold assumed construction via struct literal.
- **Fix:** Split the encoder into `encode_key(&KeyEvent, ModState)` (production) + `encode(&Key, Option<&str>, ElementState, ModState)` (test-friendly). Tests now call `encode` directly with two helpers `named(NamedKey, ModState)` and `ch(&str, ModState)`. Behavior is identical — `encode_key` just unpacks the KeyEvent fields and forwards.
- **Files modified:** `crates/vector-input/src/keymap.rs`, `crates/vector-input/src/lib.rs`, `crates/vector-input/tests/xterm_key_table.rs`
- **Verification:** 86 tests pass; `encode_key` is still used live in `vector-app::app.rs::WindowEvent::KeyboardInput`.
- **Committed in:** `fc506e7`

**2. [Rule 2 - Missing Critical] Plan-03-03 selection contract was a bounding box; rewrote to row-major**
- **Found during:** Task 2 (Compositor signature already had `selection: Option<((u16,u16),(u16,u16))>` from Plan 03-03; its `is_cell_selected` was rectangular)
- **Issue:** The Plan 03-04 `SelectionRange::cells` (and the user expectation per D-54) is row-major: partial first row, full middle rows, partial last row. Plan 03-03's bounding box would highlight a rectangle in the middle of multi-row selections — wrong shape, confusing visual.
- **Fix:** Replaced `is_cell_selected` body in `compositor.rs` with a row-major test that mirrors `SelectionRange::cells`. Added a comment noting the intentional duplication (avoids a vector-render → vector-input dep edge).
- **Files modified:** `crates/vector-render/src/compositor.rs`
- **Verification:** `selection_overlay_snapshot` test passes (selected cell blue dominates unselected cell blue + red); single-row + multi-row contract tests pass in `selection_render`.
- **Committed in:** `6aac789`

**3. [Rule 3 - Blocking] Clippy `cast_possible_truncation` + `cast_sign_loss` on f64→u32→u16**
- **Found during:** Task 2 (workspace clippy)
- **Issue:** `cell_from_pixel` converts `PhysicalPosition<f64>` to u16 cell coords; the f64→u32 cast tripped two pedantic lints.
- **Fix:** Added `#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]` at the cast sites; clamped negatives to 0 and capped at `u32::MAX` first; final u16 conversion via `u16::try_from(...).unwrap_or(u16::MAX)`.
- **Files modified:** `crates/vector-app/src/app.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` exits 0.
- **Committed in:** `6aac789`

**4. [Rule 2 - Missing Critical] `struct_excessive_bools` lint on `ModState`**
- **Found during:** Task 1 (clippy)
- **Issue:** `ModState { shift, alt, ctrl, cmd: bool }` has 4 bools — `clippy::struct_excessive_bools` triggers at 3+.
- **Fix:** `#[allow(clippy::struct_excessive_bools)]` on the struct with a comment "4 modifier flags maps 1:1 to xterm mod_param" — the bit-flag alternative would not improve clarity at this layer.
- **Files modified:** `crates/vector-input/src/mods.rs`
- **Verification:** `cargo clippy -p vector-input --all-targets -- -D warnings` exits 0.
- **Committed in:** `fc506e7`

---

**Total deviations:** 4 auto-fixed (1 Rule 2 missing-critical [selection contract], 1 Rule 2 missing-critical [lint config], 2 Rule 3 blocking [winit private field + clippy casts])
**Impact on plan:** Deviation 2 (row-major selection) corrects an inconsistency between Plan 03-03 and 03-04 contracts; both selection_overlay_snapshot and the contract tests now pass. The other three are minor build-fix shims.

## Issues Encountered

- `vector-term::Term::scroll_display` is not exposed in the wrapper; scroll-wheel wiring deferred to Plan 03-05 with a `tracing::debug` placeholder. Both `LineDelta` and `PixelDelta` variants matched.
- `clippy::await_holding_lock = "deny"` invariant holds: `pty_actor` never locks; `app.rs` only locks under sync winit callbacks (no `.await` boundaries).

## Verification

- `cargo build --workspace` — clean
- `cargo test --workspace --tests` — **163 passed, 0 failed, 4 ignored** (the 4 remaining ignored stubs are Plan 03-05 scope: frame_pacing, dpr_change_invalidates, idle_no_redraw, pty_coalesce)
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --all -- --check` — clean
- `find crates -name no_tokio_main.rs | wc -l` — **15** (invariant preserved)
- `grep -c '#\[test\]' crates/vector-input/tests/xterm_key_table.rs` — **86** (target ≥ 80)
- `grep -c '#\[test\]' crates/vector-input/tests/bracketed_paste_wrap.rs` — **4**
- Compositor signature includes `selection: Option<((u16, u16), (u16, u16))>`
- `pty_actor.rs` has `tokio::select!` with `biased;` and both `transport.write` + `transport.resize` calls

## Known Stubs

None. Scroll-wheel handling is a deliberate deferral (logged events; Plan 03-05 finalizes), tracked in the next-phase notes below — not a stub flowing into UI.

## Next Phase Readiness

**Plan 03-05 hand-off:**
- **Scroll-wheel scrollback:** wire `Term::scroll_display` (or expose alacritty's grid display offset) in vector-term; replace the `tracing::debug` stubs in `app.rs::WindowEvent::MouseWheel { delta: LineDelta | PixelDelta }`. Throttle if needed.
- **Cursor blink:** add a half-period timer (530 ms default) firing a `UserEvent::CursorBlink`; cursor pipeline already has the visibility input.
- **LPM throttle:** detect `NSProcessInfo.lowPowerModeEnabled` + `processInfoPowerStateDidChange`; cap render ticks at 30 fps; trace-log activations (D-46).
- **DPR atlas clear:** on `ScaleFactorChanged` clear `Compositor::atlas_mut()` and let the next frame lazily re-rasterize (D-48).
- **First-paint timing gate (D-51):** drop the Phase 1 overlay only after shell-spawn + first PTY read + font loaded + first row dirty. Currently we drop on the first `UserEvent::PtyOutput`; that's close but should be tightened against the atlas being ready.
- **Manual smoke matrix (03-VALIDATION.md):** 9-item smoke (vim, `cat large.log`, drag-select multi-row, Cmd-V into less, ProMotion, DPR change, LPM cap, resize live, idle render skip).

**Invariants preserved:**
- 15× `no_tokio_main.rs` arch-lint (D-08)
- `clippy::await_holding_lock = "deny"` (D-11)
- single-owner PTY actor (Plan 02-05)
- main-thread AppKit only (D-09)

---
*Phase: 03-gpu-renderer-first-paint*
*Plan: 04*
*Completed: 2026-05-11*

## Self-Check: PASSED

- All 5 created files verified present on disk.
- Both task commits (`fc506e7`, `6aac789`) verified in `git log`.
