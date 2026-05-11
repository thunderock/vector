---
phase: 03-gpu-renderer-first-paint
plan: 01
subsystem: render
tags: [wgpu, metal, winit, parking_lot, alacritty_terminal, pollster, surface, damage]

# Dependency graph
requires:
  - phase: 01-foundation-ci-dmg-pipeline
    provides: "winit + AppKit NSWindow skeleton, EventLoopProxy<UserEvent> threading split, NSTextField overlay (D-12)"
  - phase: 02-headless-terminal-core
    provides: "vector-term::Term (feed/grid/damage), vector-mux::LocalDomain + PtyTransport actor pattern"
provides:
  - "vector-render::RenderContext: wgpu Metal Surface<'static> + Device/Queue, PresentMode::Fifo, render_clear(color)"
  - "vector-app::RenderHost wrapper (Plan 03-03 extends with cell compositor)"
  - "vector-app::pty_actor: I/O-thread LocalDomain spawn + PtyOutput pump"
  - "Term::damage() / reset_damage() + TermDamage re-exports on vector-term"
  - "20 #[ignore = \"Wave-0 stub\"] test files across vector-render (11), vector-fonts (4), vector-input (2), vector-app (3)"
  - "7 workspace deps: wgpu 29, crossfont 0.9, bytemuck 1, parking_lot 0.12, pollster 0.4, etagere 0.2, unicode-width 0.2"
affects: [03-02-atlas, 03-03-compositor, 03-04-input, 03-05-pacing-polish, 04-mux]

# Tech tracking
tech-stack:
  added: [wgpu 29.0.3, crossfont 0.9.0, bytemuck 1.25, parking_lot 0.12.5, pollster 0.4.0, etagere 0.2, unicode-width 0.2.2]
  patterns:
    - "Arc<parking_lot::Mutex<Term>> shared between I/O actor (feed) and main thread (render); never crosses .await per D-11"
    - "RenderContext owns Surface<'static> via Arc<Window>; wgpu's create_surface accepts Arc<Window> directly"
    - "pollster::block_on bridges wgpu's async init synchronously on main thread (arch-lint allowlist scoped to pipeline.rs only)"
    - "Phase-1 overlay drops exactly once on first PtyOutput; subsequent bytes only call feed + request_redraw (D-51)"

key-files:
  created:
    - crates/vector-render/src/pipeline.rs
    - crates/vector-app/src/pty_actor.rs
    - crates/vector-app/src/render_host.rs
    - 20 #[ignore] test stubs (see Wave-0 Stub Map below)
  modified:
    - Cargo.toml
    - crates/vector-app/Cargo.toml
    - crates/vector-app/src/main.rs
    - crates/vector-app/src/app.rs
    - crates/vector-app/tests/win_style_mask.rs
    - crates/vector-render/Cargo.toml
    - crates/vector-render/src/lib.rs
    - crates/vector-render/tests/pipeline_init.rs
    - crates/vector-render/tests/no_tokio_main.rs
    - crates/vector-term/src/term.rs
    - crates/vector-term/src/lib.rs

key-decisions:
  - "Surface<'static> via Arc<Window>: wgpu 29 accepts Arc<Window> as DisplayAndWindowHandle, hoisting the surface lifetime out of any caller scope."
  - "render_clear returns anyhow::Result<()> (not wgpu::SurfaceError): wgpu 29 replaced SurfaceError with the CurrentSurfaceTexture enum; recoverable variants (Suboptimal/Outdated/Lost/Occluded/Timeout) log+skip, Validation surfaces as anyhow::Error."
  - "pollster::block_on in pipeline.rs is allowlisted in vector-render's arch-lint: it bridges wgpu's async init on the macOS main thread, never inside a tokio reactor — D-09 holds."
  - "Plan 02-05 actor pattern carries forward intact: pty_actor.rs owns Box<dyn PtyTransport> on the I/O thread, pumps reader.recv() -> EventLoopProxy<UserEvent::PtyOutput>. Input channel + biased select! land in Plan 03-04."
  - "tick.rs left in place with #[allow(dead_code)] on the module; Plan 03-05 removes."

patterns-established:
  - "RenderContext::new(&Arc<Window>): callers retain ownership; window stays alive for surface lifetime via the Arc clone wgpu holds internally."
  - "render_host.render_clear_default(): one-line theme entrypoint; Plan 03-05 promotes to a theme uniform."
  - "Lock-feed-drop scope in user_event: explicit block scope around `let mut t = self.term.lock(); t.feed(&bytes);` keeps clippy::await_holding_lock = deny satisfied without macros."

requirements-completed: [RENDER-01, RENDER-03, WIN-01]

# Metrics
duration: 11 min
completed: 2026-05-11
---

# Phase 3 Plan 01: Wave-0 Stubs + wgpu Metal Surface Bootstrap Summary

**wgpu 29 Metal surface bootstrapped on the existing winit/AppKit window; Phase-1 NSTextField overlay (D-12, D-51) now drops on first PTY byte; clear-color frame paints at PresentMode::Fifo. 20 #[ignore] test stubs seeded for the remaining Phase 3 plans; Term::damage() exposed for the upcoming compositor.**

## Performance

- **Duration:** 11 min
- **Started:** 2026-05-11T19:24:58Z
- **Completed:** 2026-05-11T19:35:34Z
- **Tasks:** 2 (both TDD-tagged but executed without staged RED→GREEN cycles since this plan is pure scaffolding + a wgpu bootstrap with no behavior to drive a failing test first; verification commits cover acceptance)
- **Files modified:** 12 modified, 23 created (3 src + 20 test stubs)

## Accomplishments
- **Workspace deps locked at the prescribed pins** — `wgpu 29.0.3`, `crossfont 0.9.0`, `bytemuck 1.25`, `parking_lot 0.12.5`, `pollster 0.4.0`, `etagere 0.2`, `unicode-width 0.2.2` declared in `[workspace.dependencies]`. Every later Phase 3 plan compiles against these exact versions.
- **wgpu Metal pipeline operational** — `vector-render::RenderContext` creates a `Surface<'static>` over `Arc<Window>`, configures it with `PresentMode::Fifo` (D-45) on the Metal backend, and clears to xterm-256 dark gray (`#0F0F0F`) per `render_clear_default()`. Recoverable surface states (Suboptimal/Outdated/Lost/Occluded/Timeout) log+skip; Validation surfaces as an error.
- **I/O actor wired** — `vector-app::pty_actor::io_main` spawns `LocalDomain::new()?` on the tokio I/O thread, requests a 24×80 PTY, and pumps `reader.recv() -> EventLoopProxy::send_event(UserEvent::PtyOutput(chunk))`. Single-owner discipline holds: only this task touches the transport (Plan 02-05 actor pattern carries forward intact).
- **Phase-1 overlay drops exactly once on first PtyOutput** — `App::user_event` scope-locks `Arc<parking_lot::Mutex<Term>>`, calls `Term::feed(&bytes)`, drops the lock, then nulls `self.overlay = None` exactly once (D-51) before calling `request_redraw()`. `clippy::await_holding_lock = "deny"` is satisfied at compile time.
- **Term::damage() / reset_damage() exposed** — Plan 03-03's compositor seam is in place. `TermDamage`, `TermDamageIterator`, `LineDamageBounds` re-exported via `vector_term::*` so `vector-render` does not need a direct `alacritty_terminal` dep.
- **20 Wave-0 #[ignore] test stubs live on disk** covering every remaining Phase 3 plan target (mapping below). `cargo test --workspace --tests` reports 55 passed / 0 failed / 18 ignored on completion (baseline 53, +2 = `pipeline_init` + `win_style_mask` un-ignored by this plan).

## Task Commits

1. **Task 1: Wave-0 test stubs + workspace deps + Term::damage() wrapper** — `cd0159d` (feat)
2. **Task 2: wgpu surface lifecycle + clear-color frame + I/O actor wiring** — `eea4540` (feat)

_Plan metadata commit lands separately after this SUMMARY._

## Files Created/Modified

**Created (src):**
- `crates/vector-render/src/pipeline.rs` — `RenderContext::new(&Arc<Window>)` + `resize(w,h)` + `render_clear(&[f64;4])`. Owns `Surface<'static>`, `Device`, `Queue`.
- `crates/vector-app/src/pty_actor.rs` — I/O-thread async actor: `LocalDomain` spawn → `take_reader()` → `EventLoopProxy::send_event(PtyOutput)`.
- `crates/vector-app/src/render_host.rs` — Thin wrapper over `RenderContext` so Plan 03-03 can extend without touching `app.rs`.

**Created (test stubs — 20):** see Wave-0 Stub Map below.

**Modified:**
- `Cargo.toml` — 7 new `[workspace.dependencies]` entries (alphabetical insertion).
- `crates/vector-render/Cargo.toml` — added `wgpu`, `bytemuck`, `pollster`, `parking_lot`, `winit`, `vector-term` per-crate deps.
- `crates/vector-render/src/lib.rs` — replaced `_force_anyhow_use` stub with `mod pipeline` + `pub use RenderContext`.
- `crates/vector-render/tests/no_tokio_main.rs` — `BLOCK_ON_ALLOWLIST` extended with `pipeline.rs` (wgpu init bridge).
- `crates/vector-render/tests/pipeline_init.rs` — un-ignored; probes Metal adapter without surface.
- `crates/vector-app/Cargo.toml` — added `vector-render`, `vector-term`, `vector-mux`, `parking_lot`, `wgpu` deps.
- `crates/vector-app/src/main.rs` — `UserEvent::PtyOutput(Vec<u8>)`; `mod pty_actor; mod render_host; #[allow(dead_code)] mod tick;`; I/O thread now calls `pty_actor::io_main`.
- `crates/vector-app/src/app.rs` — `App` gained `term: Arc<Mutex<Term>>`, `render_host: Option<RenderHost>`, `overlay_dropped: bool`. Wired `resumed`/`user_event`/`window_event` per D-09/D-11/D-51.
- `crates/vector-app/tests/win_style_mask.rs` — un-ignored; compile-checks `NSWindowStyleMask` import path.
- `crates/vector-term/src/term.rs` — added `pub fn damage(&mut self)` + `pub fn reset_damage(&mut self)`.
- `crates/vector-term/src/lib.rs` — re-exported `TermDamage`, `TermDamageIterator`, `LineDamageBounds`.

## Wave-0 Stub Map

20 `#[ignore = "Wave-0 stub"]` test files seeded for later Phase 3 plans:

| File                                                          | Owning Plan | Requirement            |
| ------------------------------------------------------------- | ----------- | ---------------------- |
| `crates/vector-render/tests/snapshot_clearcolor.rs`           | 03-03       | RENDER-01              |
| `crates/vector-render/tests/snapshot_singlecell.rs`           | 03-03       | RENDER-01              |
| `crates/vector-render/tests/snapshot_truecolor.rs`            | 03-03       | RENDER-04              |
| `crates/vector-render/tests/atlas_lru.rs`                     | 03-02       | RENDER-04 (Pitfall 2)  |
| `crates/vector-render/tests/dpr_change_invalidates.rs`        | 03-05       | RENDER-04 (D-48)       |
| `crates/vector-render/tests/pipeline_init.rs`                 | **03-01**   | RENDER-01 (un-ignored) |
| `crates/vector-render/tests/damage_to_quads.rs`               | 03-03       | RENDER-01              |
| `crates/vector-render/tests/pty_coalesce.rs`                  | 03-05       | RENDER-02 (D-47)       |
| `crates/vector-render/tests/idle_no_redraw.rs`                | 03-05       | RENDER-03              |
| `crates/vector-render/tests/cursor_overlay_snapshot.rs`       | 03-03       | RENDER-05              |
| `crates/vector-render/tests/selection_overlay_snapshot.rs`    | 03-04       | RENDER-05              |
| `crates/vector-fonts/tests/crossfont_load_bundled.rs`         | 03-02       | D-41                   |
| `crates/vector-fonts/tests/grayscale_pixel_format.rs`         | 03-02       | D-50                   |
| `crates/vector-fonts/tests/two_atlas_split.rs`                | 03-02       | RENDER-04              |
| `crates/vector-fonts/tests/atlas_lru_eviction.rs`             | 03-02       | RENDER-04 (Pitfall 2)  |
| `crates/vector-input/tests/xterm_key_table.rs`                | 03-04       | D-52                   |
| `crates/vector-input/tests/bracketed_paste_wrap.rs`           | 03-04       | D-53                   |
| `crates/vector-app/tests/win_style_mask.rs`                   | **03-01**   | WIN-01 (un-ignored)    |
| `crates/vector-app/tests/selection_render.rs`                 | 03-04       | RENDER-05 + D-54       |
| `crates/vector-app/tests/frame_pacing.rs`                     | 03-05       | RENDER-02 + RENDER-03  |

**Plan-vs-shipped count:** Plan text states "17" Wave-0 stubs in places, but the concrete `<files>` list enumerates 20 stub files (mapping table identical). Shipped 20, matching the file list. See Deviations.

## Decisions Made

- **Surface<'static> via Arc<Window>**: wgpu 29's `SurfaceTarget::DisplayAndWindow(Box<dyn DisplayAndWindowHandle + 'window>)` accepts `Arc<Window>` (winit's `Window` implements the trait). Cloning the Arc into the surface decouples its lifetime from the caller's scope. `App` retains `Option<Arc<Window>>` for `request_redraw()` and resize.
- **render_clear returns `anyhow::Result<()>`** (not `Result<(), wgpu::SurfaceError>` as written in the plan). wgpu 29 replaced `SurfaceError` with the `CurrentSurfaceTexture` enum — see Deviations below.
- **`pollster::block_on` allowlisted in vector-render's arch-lint** with a single entry: `pipeline.rs`. wgpu's adapter/device init is async-typed but executes synchronously on the macOS main thread (Metal requires it). Bridging via pollster keeps the call out of any tokio reactor; the D-09 invariant (no PTY async on the main thread) is preserved.
- **`#[ignore]` attributes carry a reason string** (`#[ignore = "Wave-0 stub"]`) — workspace clippy lints have `clippy::ignore_without_reason = warn` rolled up by `pedantic`, and `-D warnings` promotes it to deny. See Rule 1 deviation.
- **`tick.rs` kept on disk** with the module declaration marked `#[allow(dead_code)]`. Plan 03-05 deletes the file; doing it here would create a wider blast radius than the plan owns.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] wgpu 29.0.3 API drift from plan's documented snippets**
- **Found during:** Task 2 (RenderContext::new initial compile)
- **Issue:** Plan reproduced wgpu 29 example code that does not match the published 29.0.3 surface:
  - `InstanceDescriptor` no longer implements `Default`; must use `InstanceDescriptor::new_without_display_handle()` and assign fields.
  - `Instance::new(desc: InstanceDescriptor)` takes the descriptor by value (not by reference).
  - `DeviceDescriptor` gained a required `experimental_features: ExperimentalFeatures` field.
  - `RenderPassDescriptor` gained a required `multiview_mask: Option<NonZeroU32>` field.
  - `RenderPassColorAttachment` gained a required `depth_slice: Option<u32>` field.
  - `Surface::get_current_texture()` returns the `CurrentSurfaceTexture` enum (Success | Suboptimal | Timeout | Occluded | Outdated | Lost | Validation), not `Result<SurfaceTexture, SurfaceError>` — `?` does not apply.
  - `Instance::request_adapter` returns `Future<Output = Result<Adapter, RequestAdapterError>>` (not `Option<Adapter>`).
- **Fix:** Rewrote `pipeline.rs` against the actual 29.0.3 API: constructed `InstanceDescriptor` via `new_without_display_handle()`, added `experimental_features: ExperimentalFeatures::disabled()` to `DeviceDescriptor`, added `multiview_mask: None` to `RenderPassDescriptor`, added `depth_slice: None` to `RenderPassColorAttachment`, replaced `Result<…, SurfaceError>` with `anyhow::Result<()>` and pattern-matched `CurrentSurfaceTexture`, replaced `.ok_or_else(…)?` with `.map_err(|e| anyhow!(…))?`.
- **Files modified:** `crates/vector-render/src/pipeline.rs`, `crates/vector-render/tests/pipeline_init.rs`
- **Verification:** `cargo check -p vector-render` clean, `cargo test --workspace --tests` 55 passed / 0 failed; `cargo run -p vector-app --release` alive after 5s with clean exit on SIGTERM.
- **Committed in:** `eea4540`

**2. [Rule 1 - Bug] `clippy::needless_pass_by_value` on `RenderContext::new(window: Arc<Window>)`**
- **Found during:** Task 2 (clippy pass)
- **Issue:** Inside the function the Arc is cloned exactly once (passed into `instance.create_surface`); clippy sees the original binding as un-consumed and demands a borrow.
- **Fix:** Changed signature to `pub fn new(window: &Arc<Window>) -> Result<Self>`; mirrored in `RenderHost::new`. `app.rs` now passes `&window` (the original Arc remains in `self.window`).
- **Files modified:** `crates/vector-render/src/pipeline.rs`, `crates/vector-app/src/render_host.rs`, `crates/vector-app/src/app.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` clean.
- **Committed in:** `eea4540`

**3. [Rule 1 - Bug] `clippy::ignore_without_reason` on every `#[ignore]` stub**
- **Found during:** Task 1 (clippy pass)
- **Issue:** Plan template prescribed bare `#[ignore]`, but the workspace's pedantic clippy lint group denies bare `#[ignore]` without a reason string.
- **Fix:** Replaced `#[ignore]` with `#[ignore = "Wave-0 stub"]` across all 20 stub files via `perl -i -pe`.
- **Files modified:** all 20 stub files listed in the Wave-0 Stub Map.
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` clean.
- **Committed in:** `cd0159d`

**4. [Rule 3 - Blocking] vector-render arch-lint `block_on` allowlist needs `pipeline.rs`**
- **Found during:** Task 2 (test pass)
- **Issue:** `crates/vector-render/tests/no_tokio_main.rs::forbidden_tokio_patterns_absent_from_src` panicked on `pollster::block_on(...)` calls in `pipeline.rs`. The lint has zero tolerance by default (`BLOCK_ON_ALLOWLIST: &[]`). wgpu requires synchronous-looking init on the macOS main thread.
- **Fix:** Added `"pipeline.rs"` to `BLOCK_ON_ALLOWLIST` with a comment explaining the wgpu-on-main-thread rationale. D-09 invariant (no PTY async on main thread) remains intact — these block_on calls are wgpu init, not PTY I/O.
- **Files modified:** `crates/vector-render/tests/no_tokio_main.rs`
- **Verification:** `cargo test -p vector-render --test no_tokio_main` passes; 15==15 arch-lint invariant holds.
- **Committed in:** `eea4540`

**5. [Documentation drift, not a code change] Plan said "17 Wave-0 stubs" but `<files>` list enumerated 20**
- **Found during:** Task 1 (file creation)
- **Issue:** Plan body references "17" in several places (objective, behavior, success_criteria), but the `<files>` list and the action mapping table both enumerate 20 concrete paths.
- **Fix:** Shipped 20 stubs (the file list is the load-bearing source of truth; the "17" tokens are stale prose). Mapping table in this SUMMARY documents all 20.
- **Files modified:** N/A (no code change; documentation discrepancy)
- **Verification:** `find crates/{vector-render,vector-fonts,vector-input,vector-app}/tests -name '*.rs' -not -name 'no_tokio_main.rs' | wc -l` outputs 20.
- **Committed in:** `cd0159d` (mapping table preserved in this SUMMARY for Plan 03-02..05 reference)

---

**Total deviations:** 4 code auto-fixes + 1 documentation discrepancy
**Impact on plan:** All four code fixes are correctness deviations (API drift, clippy gates, arch-lint gate). The "17 vs 20" doc drift required no code change; the 20 stubs all carry their owning Plan tag and are the working contract for downstream plans. No scope creep, no architectural changes (Rule 4 never triggered).

## Issues Encountered

None beyond the deviations above. The wgpu-on-main-thread / pollster pattern, the `Arc<Window>` lifetime hand-off, and the lock-feed-drop scope in `user_event` were prescribed by the plan and worked exactly as written once the wgpu 29 API drift was reconciled.

## User Setup Required

None — no external service configuration required. JetBrains Mono bundling and font-stack wiring lands in Plan 03-02; this plan paints a clear color only.

## Hand-off Notes

**Plan 03-02 (atlas):** `RenderContext` exposes `pub device: Device` and `pub queue: Queue`. The atlas crate should take a `&Device` + `&Queue` in its constructor and live alongside `RenderContext` inside `RenderHost`; do not duplicate the wgpu device. `crossfont 0.9` is at the workspace level — `vector-fonts/Cargo.toml` needs `crossfont.workspace = true`. The four font test stubs are live (Wave-0 Stub Map).

**Plan 03-03 (compositor):** `Term::damage()` returns `alacritty_terminal::term::TermDamage<'_>`; iterate `Partial(TermDamageIterator<'_>)` yielding `LineDamageBounds { line, left, right }`. All three types are re-exported via `vector_term::*` so `vector-render` does NOT need a direct `alacritty_terminal` dep. The renderer should acquire the lock via `Arc<parking_lot::Mutex<Term>>` (already shared between `App` and the I/O actor), iterate damage in a `{ }` scope, drop the lock, then encode draw calls. After successful submit, call `term.reset_damage()` inside a second tight lock scope. The cell snapshot strategy (collect rows under the lock vs. iterate while holding) is the compositor plan's call. Six test stubs are live (snapshot_clearcolor/singlecell/truecolor/damage_to_quads/cursor_overlay_snapshot + atlas_lru in vector-render).

**Plan 03-04 (input):** `pty_actor.rs` currently only pumps reads. The plan's existing comment marker (`Plan 03-04 will add a write channel + biased select! for input`) is in the file. Wire-up steps: introduce a `mpsc::channel::<Vec<u8>>(64)` somewhere on the App side, pass the `Sender` into `App` (e.g., constructor parameter), pass the `Receiver` into `pty_actor::run` as a second argument, and turn the actor's body into a `biased; tokio::select! { reader.recv() => …, write_rx.recv() => transport.write(&bytes).await }`. Five input/render test stubs are live (xterm_key_table, bracketed_paste_wrap, selection_overlay_snapshot, selection_render, and the WIN-01-extension follow-on inside win_style_mask if a richer assertion ever lands).

**Plan 03-05 (pacing + polish):** Four test stubs target this plan (dpr_change_invalidates, pty_coalesce, idle_no_redraw, frame_pacing). When you delete `tick.rs`, remove the `#[allow(dead_code)] mod tick;` line from `main.rs` and drop the `UserEvent::Tick(u64)` variant. The clear-color in `render_host.rs::render_clear_default` is the hand-off point for theme-uniformization (Claude's Discretion D-40).

## Self-Check: PASSED

- FOUND: `crates/vector-render/src/pipeline.rs`
- FOUND: `crates/vector-app/src/pty_actor.rs`
- FOUND: `crates/vector-app/src/render_host.rs`
- FOUND: `.planning/phases/03-gpu-renderer-first-paint/03-01-SUMMARY.md`
- FOUND commit `cd0159d` (Task 1)
- FOUND commit `eea4540` (Task 2)
- Wave-0 stub count: 20 (live on disk under `crates/{vector-render,vector-fonts,vector-input,vector-app}/tests/`)
- Arch-lint invariant: 15 `no_tokio_main.rs` files (unchanged from baseline)

---
*Phase: 03-gpu-renderer-first-paint*
*Completed: 2026-05-11*
