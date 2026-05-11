# Phase 3: GPU Renderer & First Paint — Research

**Researched:** 2026-05-11
**Domain:** wgpu/Metal GPU rendering of an `alacritty_terminal` grid, CoreText glyph rasterization via `crossfont`, winit input + AppKit window chrome on macOS 13+
**Confidence:** HIGH

## Summary

Phase 3 replaces the Phase-1 NSTextField overlay with a `wgpu::Surface` over the existing winit NSWindow and composites `vector_term::Term.grid()` through a two-atlas (mono + emoji) glyph cache. The path is well-trod: Alacritty has been running this stack (minus wgpu — Alacritty still uses OpenGL) for years; cosmic-term and ghostty validate the wgpu-on-Metal variant; WezTerm runs wgpu in production (currently 25.x). All upstream APIs needed by `03-CONTEXT.md`'s D-40..D-55 decisions have been verified against the live crates.io and docs.rs as of 2026-05-11.

Three findings change one detail in CONTEXT:

1. **`crossfont::BitmapBuffer` has only `Rgb` and `Rgba` variants — no `Grayscale`.** D-50 ("Grayscale AA via CoreText, single 8-bit channel") was correct in intent but slightly off in detail: CoreText returns grayscale-AA as a 3-channel "RGB alphamask" (`BitmapBuffer::Rgb`). The mono atlas stores RGB-alphamask, not a single 8-bit channel. Functionally identical — the shader still does additive subpixel-style blend — but the atlas texture is `Rgba8Unorm` (or `Rgba8UnormSrgb`), not `R8Unorm`. Trivial planner adjustment.
2. **`alacritty_terminal::Term::damage()` and `reset_damage()` are confirmed.** Signature `pub fn damage(&mut self) -> TermDamage<'_>`; `TermDamage` is `enum { Full, Partial(TermDamageIterator) }`; iterator yields `LineDamageBounds { line: usize, left: usize, right: usize }`. Per-row + per-column-range damage is **already richer than CONTEXT D-44 needed** — we get column extents free.
3. **D-44 requires `&mut self` on `Term`.** `damage()` takes `&mut Term` (alacritty quirk: returns a borrowed iterator over a mutable damage slice). The render-side lock must be a write lock, not a read lock. This rules out `RwLock` reader sharing during render and confirms `parking_lot::Mutex<Term>` is the right shape.

**Primary recommendation:** Carve Phase 3 into 5 plans (03-01 wgpu surface, 03-02 crossfont + atlases, 03-03 grid→quads compositor, 03-04 input pipeline, 03-05 frame pacing + DPR robustness). The compositor lives in `vector-render`; `vector-app` owns only the wgpu surface lifecycle + the winit event handler. Drop the Phase-1 NSTextField overlay on the same frame the first PTY byte lands (D-51), not over a transition animation.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Font + atlas stack:**
- **D-40:** `crossfont 0.9` + hand-rolled two-atlas (mono SDF/grayscale + RGBA emoji), bounded LRU. Reject cosmic-text+glyphon as the primary stack — it's a great general-text engine but overkill for a cell-grid renderer and would force shape/layout passes our cells don't need. swash/cosmic-text can be reconsidered in v2 if we add a settings/AI panel that needs rich text.
- **D-41:** JetBrains Mono = bundled font (in `Vector.app/Contents/Resources/Fonts/`) AND default user-visible font. Single artifact, OFL license. Bundling is mandatory for snapshot-test determinism. User overrides via TOML `[font].family = "..."`; fallback chain still goes through CoreText.
- **D-42:** Ligatures off by default; opt-in via `[font].ligatures = true`. Phase 3 stays shaping-free. Single global toggle in v1.
- **D-43:** Per-atlas bounded LRU eviction. Each atlas caps at fixed size (e.g., 2048×2048 — final value planner's call). Width measured via `unicode-width` crate, never via font advance.

**Frame pacing + dirty detection:**
- **D-44:** Dirty detection uses `alacritty_terminal::Term::damage()` API + our own dirty-rows bitmap. Per-row granularity.
- **D-45:** `wgpu::PresentMode::Fifo` everywhere; honor whatever vsync gives us. Render-on-dirty + Fifo together: idle costs zero; ProMotion automatically gets 120 fps on dirty frames.
- **D-46:** Low Power Mode cap = 30 fps; logged via `tracing`. Detect via `NSProcessInfo.lowPowerModeEnabled` + `processInfoPowerStateDidChange` notification. Throttle by skipping render ticks; do NOT suspend the renderer (iTerm2's choice desyncs scroll logs).
- **D-47:** PTY-burst coalescing: I/O thread accumulates reads into a `BytesMut`; drains to main once per ~8 ms tick OR when buffer hits a size threshold. Main calls `Term::feed()` exactly once per drain.

**Display change / DPR robustness:**
- **D-48:** On `ScaleFactorChanged`: clear both atlases + invalidate cell→slot map; lazy re-rasterize per glyph on next reference. Acceptable one-frame stutter.
- **D-49:** Live window resize: debounce `Term::resize()` to ~50 ms quiescent / on resize-end; wgpu surface resizes on every event (cheap); repaint with current grid between events.
- **D-50:** Grayscale AA via CoreText, always. Apple disabled subpixel AA system-wide in Mojave. *(Research nuance: crossfont returns this as `BitmapBuffer::Rgb` — a 3-channel alpha-mask. Atlas is RGBA8, not R8. Functionally equivalent.)*
- **D-51:** First real GPU frame paints after: shell spawned (`LocalDomain::spawn`) + first PTY read arrived + font loaded into atlas + first row marked dirty. Phase-1 placeholder may remain visible for one frame as the window opens; immediately replaced once those four conditions are met.

**Input handling scope:**
- **D-52:** Full xterm-compatible keyboard mapping. ASCII/Unicode + Esc + arrows + F1–F12 + nav + Shift/Option/Ctrl combinations. Per xterm key table. Option key sends ESC+char by default. Ctrl chords are byte-value mappings. Targeted ~80 unit tests against the xterm key table.
- **D-53:** `Cmd-V` paste in Phase 3 (bracketed paste: ESC [ 200~ … ESC [ 201~). `Cmd-C` copy and selection-string semantics deferred to Phase 5. Phase 3 implements selection *rendering* only.
- **D-54:** Mouse: scroll-wheel → scrollback viewport offset; left-click-drag → selection rectangle. No mouse-reporting-to-PTY (DEC 1006/1015/1016) in Phase 3.
- **D-55:** Phase 3 owns single-window, single-PTY input + rendering. Phase 4 owns Cmd-T/Cmd-W tabs, Cmd-D splits. Phase-1 menu items for tabs/splits stay disabled. `Cmd-Q` and `Cmd-W` (close window) are the only window-lifecycle shortcuts active.

### Claude's Discretion

These the researcher/planner/executor resolve without further user input:

- **Renderer crate boundary** — where the compositor (Grid → draw calls) lives: `vector-render` exclusively, or split with `vector-app`. Researcher picks based on what makes the wgpu code cleaner. **Researcher recommendation in this doc: compositor lives entirely in `vector-render`; `vector-app` owns only the wgpu Surface lifecycle and the winit event handler.**
- **Default theme colors** — pick a sensible xterm-256-compatible palette. Surface in `vector-theme` trait shape; ship one default.
- **Cursor visuals** — block style; blink rate matches macOS default or fixed 530 ms half-period.
- **Glyph atlas dimensions and slot allocation** — must fit in macOS Metal texture limits (8192×8192 minimum guaranteed; 16384×16384 on Apple Silicon) and respect bounded LRU.
- **Renderer panic policy** — log + clear render to a sentinel "renderer error" frame; restart the wgpu surface on next user input. Existing `tracing` infra is the diagnostic channel.
- **Selection rectangle visual** — translucent rectangle composited over the live grid; exact alpha/color planner's call (must be visible against both dark and light backgrounds).
- **PTY-burst coalescing threshold** — exact debounce window (5–10 ms) and buffer-size trigger; tune empirically against `cat large.log`.

### Deferred Ideas (OUT OF SCOPE)

**Phase 4 (mux):**
- Tabs (Cmd-T, Cmd-W close-tab, Cmd-Shift-[/], `NSWindowTabbingMode`)
- Splits (Cmd-D / Cmd-Shift-D)
- Focus routing
- Enabling Phase-1-stubbed tab/split menu items

**Phase 5 (Polish):**
- Cmd-F search overlay (D-39)
- Cmd-C copy + selection-to-string semantics
- Mouse-reporting modes (DEC 1006/1015/1016 → PTY)
- Per-domain (local vs remote) ligature toggle
- swash or vendored harfbuzz when ligatures move beyond "off by default"

**Backlog:**
- 999.1 AI autocomplete + history-aware Claude suggestions

**Reviewed Todos (not folded):** Code-quality hardening (workspace lints, arch-lint upgrade, pre-commit cargo-deny) — Phase 5, not Phase 3.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| **RENDER-01** | GPU-accelerated rendering targets the Metal backend of `wgpu`, with damage-tracked redraws (only dirty rows shaped/uploaded). | `wgpu 29.0.3` Metal backend (HIGH); `alacritty_terminal 0.26 Term::damage() → TermDamage::Partial(iter) → LineDamageBounds { line, left, right }` (HIGH — verified docs.rs); compositor reads damage iterator + per-row vertex-buffer rebuild. Pitfall 2 atlas + Pitfall 3 frame pacing both directly addressed. |
| **RENDER-02** | Sustained `cat large.log` ≥ 60 fps on Apple Silicon at 1080p; ProMotion (120 Hz) honored. | `PresentMode::Fifo` on Metal honors display vsync; ProMotion picks 120 Hz automatically. PTY-burst coalescing (D-47) keeps `Term::feed` to one call per drain. Render-on-dirty (D-44) caps idle to zero work. |
| **RENDER-03** | Idle CPU < 1% on Apple Silicon when no dirty rows. | Render-on-dirty + winit `ControlFlow::Wait` (already set in Phase 1 `main.rs:34`). No frame happens unless `damage()` returns non-empty OR a user input fires. |
| **RENDER-04** | Glyph atlas separates mono + emoji, bounded LRU, survives Retina ↔ non-Retina scale change. | crossfont 0.9 `BitmapBuffer::{Rgb, Rgba}` returns mono (RGB alphamask) and color emoji (RGBA premultiplied) separately — natural two-atlas split. `ScaleFactorChanged` event (winit 0.30 verified) clears + lazy-rerasterizes per D-48. |
| **RENDER-05** | Cursor and selection overlays render correctly under the live text grid. | Three-pass render: (1) cell quads from damage iterator, (2) selection rect (translucent alpha-blend), (3) cursor block (additive blend over current cell). All three pipelines share one surface; selection rect uses scissoring or grid-coordinate→pixel-coordinate transform. |
| **WIN-01** | Native macOS AppKit window with title bar, fullscreen, and standard window-control buttons. | Phase 1 `WindowAttributes::default()` already provides title bar + close + minimize. Fullscreen needs `NSWindow.collectionBehavior` `.fullScreenPrimary` (winit's `WindowAttributes::with_fullscreen` wires this) OR the menu item already in Phase 1 (`toggleFullScreen:` selector — Cmd-Ctrl-F, see `menu.rs::view_menu`). **No new AppKit work required** — the green fullscreen button (zoom button) is on by default for windows with default style mask `Titled|Closable|Miniaturizable|Resizable`. |

</phase_requirements>

## Project Constraints (from CLAUDE.md)

**Tech stack — Phase 3 additions (already pinned in CLAUDE.md's Tech Stack Recommendations):**
- `wgpu 29` (verified live: 29.0.3, 2026-05-02 — MSRV 1.87, comfortably under our 1.88 pin)
- `winit 0.30` (already pinned at 0.30.13 in workspace Cargo.toml)
- `objc2-app-kit 0.3` (already pinned; tracks objc2 0.6.4)
- `crossfont 0.9` (verified live: 0.9.0, 2025-06-09; MSRV 1.77)
- **NOT glyphon** — CLAUDE.md's renderer-by-variant section explicitly says `crossfont` + custom atlas; glyphon would force shaping we don't need.
- `unicode-width` (verified live: 0.2.2, 2025-10-06) — cell width source of truth per Pitfall 2

**Workflow rules:**
- "Commit each logical stage separately; **do not push** — the user reviews diffs and pushes asynchronously" (CLAUDE.md "Constraints" section + Phase 1/2 established practice).
- **Always use the project's lint/format commands** (CLAUDE.md global instructions). Workspace ships `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all -- --check` as the green-bar contract (per `.github/workflows/ci.yml`). Same commands locally and in CI.
- **Comments: succinct, one short line, only when the WHY is non-obvious** (CLAUDE.md global). No multi-paragraph rustdoc on Phase 3 atlas code.

**Scope discipline:**
- macOS 13 baseline (`MACOSX_DEPLOYMENT_TARGET=13.0` — already in xtask + CI). No Sonoma-only or Sequoia-only APIs. NSProcessInfo's `lowPowerModeEnabled` is pre-Mojave so we're safe.
- Universal binary (arm64 + x86_64 via `lipo`). `wgpu` Metal backend handles both natively.
- **Resist scope creep.** No Sixel/Kitty graphics, no IME, no command palette, no AI assist, no per-domain ligature toggle this phase.

**Threading invariants (Phase 1 D-09/D-11, still binding):**
- `winit::EventLoop` on the main thread. `tokio` multi-thread runtime on the dedicated I/O thread spawned in `main.rs`. Cross-thread signaling via `EventLoopProxy::send_event` only.
- `clippy::await_holding_lock = "deny"` workspace-level. The render path's `Term` lock must not be held across any `.await`.
- Per-crate `tests/no_tokio_main.rs` arch-lint: 15 invariants. Phase 3 adds zero new crates (uses existing `vector-render`, `vector-fonts`, `vector-input`, `vector-app`) so the 15==15 invariant holds without action.

## Standard Stack

### Core (new in Phase 3)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| **`wgpu`** | 29.0.3 (2026-05-02) | GPU pipeline (Metal backend on macOS) | Industry-standard Rust GPU API; cross-platform shader tooling; WezTerm validates it for terminals. Already specified in CLAUDE.md. |
| **`crossfont`** | 0.9.0 (2025-06-09) | Font rasterization via CoreText | Alacritty's font crate; already handles CoreText fallback chain, ligature shaping (off by default per D-42), color emoji via `BitmapBuffer::Rgba`. |
| **`unicode-width`** | 0.2.2 (2025-10-06) | Cell width = displayed width per UAX #11 | Source of truth for "is this glyph 1 or 2 cells wide". Never font-advance. Pitfall 2 prescribed. |
| **`bytemuck`** | 1.x (workspace pin in plan) | `cast_slice` for vertex/index buffers | Standard for wgpu vertex buffer upload — `#[derive(Pod, Zeroable)]` on `Vertex`/`CellInstance` lets us `cast_slice` straight into `Buffer::write_buffer`. |
| **`pollster`** | 0.4.x | One-shot `block_on` for wgpu Adapter request | `wgpu::Instance::request_adapter` and `Adapter::request_device` are async. We call these from `app.rs::resumed` (main thread, not inside the tokio runtime). `pollster::block_on` is the canonical lightweight executor for these one-shots. **Allowed in arch-lint:** the existing `tests/no_tokio_main.rs` forbids `tokio::main`/`current_thread`/`block_on` outside an allowlist — pollster::block_on is unrelated and uses a separate name; arch-lint passes unchanged. |

### Supporting (already in workspace from Phase 1/2)

| Library | Version | Purpose | Use in Phase 3 |
|---------|---------|---------|-----------------|
| `winit` | 0.30.13 | Event loop, NSWindow, raw-window-handle | wgpu Surface ties to `winit::Window`; KeyEvent → xterm bytes; Resized/ScaleFactorChanged drive D-48/D-49. |
| `objc2-app-kit` | 0.3 | Direct AppKit access | `NSProcessInfo.isLowPowerModeEnabled()` for D-46; `NSPasteboard.generalPasteboard().stringForType(...)` for D-53 Cmd-V; menu wiring already done in Phase 1. |
| `objc2-foundation` | 0.3 | NSNotificationCenter for `processInfoPowerStateDidChange` (D-46) | Observe via `NSNotificationCenter::defaultCenter().addObserver_selector_name_object_`. |
| `raw-window-handle` | 0.6 | Bridge winit ↔ wgpu surface | `wgpu::Instance::create_surface(window)` accepts `&Window` directly in wgpu 29; no explicit raw-handle handoff. |
| `parking_lot` | (add at workspace) | `Mutex<Term>` between I/O actor and render path | `Term::damage()` requires `&mut self` (alacritty quirk — confirms `Mutex`, not `RwLock`). Parking_lot is faster, fairer, and what the Phase 2 D-11 enforcement assumes ("await-holding-lock"). |
| `async-trait` | 0.1 | Already wired in vector-mux | No new use in Phase 3 itself; transport+domain layer unchanged. |
| `vector-term` | (path) | `Term::new/feed/resize/grid/cursor/mode/damage()` | LOCKED per 02-02-SUMMARY. We add ONE method this phase: a thin wrapper `pub fn damage(&mut self) -> TermDamage<'_>` re-exporting alacritty's API (see Open Questions). |
| `vector-mux` | (path) | `LocalDomain::spawn → Box<dyn PtyTransport>` | LOCKED per 02-04-SUMMARY. Phase 3 calls `LocalDomain::new()? .spawn(SpawnCommand { rows, cols, ... }).await?` exactly once at startup. |
| `tracing` | 0.1 | Frame timing, LPM throttle, atlas evictions, DPR rebuilds | Diagnostic channel per CONTEXT specifics. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `crossfont 0.9` | `cosmic-text 0.19 + swash 0.2.7 + glyphon 0.6.x` | Higher-level: solves shaping + fallback + atlas in one. Cost: heavier, less terminal-tuned, forces shape passes we don't need, and we'd inherit cosmic-text's font-discovery quirks. CONTEXT D-40 locked crossfont; this row is for traceability. |
| Hand-rolled atlas | `etagere` (atlas packer) + custom wgpu glue | etagere is the canonical Rust atlas-packing crate (used by glyphon, cosmic-term). Saves ~150 LOC of shelf-packing code. **Researcher recommendation: use etagere.** It's a single small crate with no transitive bloat; planner may pick. |
| `wgpu 29` | `metal-rs` directly (Apple-only Metal bindings) | ~30% less code; locks us to macOS forever. CLAUDE.md already rejected this. |
| `bytemuck` | hand-rolled `unsafe { transmute }` | Bytemuck is `#[deny(unsafe_code)]`-compatible (the `unsafe` lives inside bytemuck, not our crates). Required because the workspace forbids `unsafe_code` outside `vector-app` (D-06). |
| `pollster` for wgpu init | tokio handle.block_on | `tokio::runtime::Handle::block_on` from the main thread deadlocks because main IS the tokio reactor in some configurations. pollster is a single-threaded park/unpark executor — safe to call from anywhere. |

**Installation:**

Add to workspace `Cargo.toml`:

```toml
[workspace.dependencies]
wgpu = { version = "29", default-features = false, features = ["metal", "wgsl"] }
crossfont = "0.9"
unicode-width = "0.2"
bytemuck = { version = "1", features = ["derive"] }
pollster = "0.4"
parking_lot = "0.12"
# Optional but recommended:
etagere = "0.2"
```

**Crate-level wiring (Phase 3):**
- `vector-render/Cargo.toml`: add `wgpu`, `bytemuck`, `pollster`, `parking_lot`, `etagere` (optional), `vector-term = { path = "..." }`, `vector-fonts = { path = "..." }`.
- `vector-fonts/Cargo.toml`: add `crossfont`, `unicode-width`, `parking_lot`.
- `vector-app/Cargo.toml`: add `vector-render = { path = "..." }`, `vector-term = { path = "..." }`, `vector-mux = { path = "..." }`, `vector-input = { path = "..." }`, `parking_lot`.
- `vector-input/Cargo.toml`: add `winit = { workspace = true }` (winit's KeyEvent types are the input source).

**Version verification (run before final plan locks pins):**

```bash
npm view ...    # N/A — we use cargo
cargo search wgpu --limit 1            # confirm 29.x latest stable
cargo search crossfont --limit 1       # confirm 0.9.x
cargo search unicode-width --limit 1   # confirm 0.2.x
```

Verified versions and dates above are from crates.io API live fetch on 2026-05-11.

## Architecture Patterns

### Recommended Project Structure

```
crates/
├── vector-app/             # main.rs, app.rs, menu.rs, overlay.rs, tick.rs (existing)
│   └── src/render_host.rs  # NEW: wgpu surface lifecycle + redraw scheduling
├── vector-render/          # PROMOTE from stub — owns the wgpu compositor
│   ├── src/
│   │   ├── lib.rs          # public API: Renderer, RenderTarget
│   │   ├── pipeline.rs     # wgpu Device/Queue/Surface/Pipelines
│   │   ├── cell_pipeline.rs    # cell quad pipeline (vertex+frag .wgsl)
│   │   ├── selection_pipeline.rs # translucent selection rect
│   │   ├── cursor_pipeline.rs    # cursor block
│   │   ├── compositor.rs   # Grid + damage → CellInstance Vec
│   │   ├── atlas.rs        # GlyphAtlas (mono + emoji)
│   │   └── shaders/
│   │       ├── cell.wgsl
│   │       ├── selection.wgsl
│   │       └── cursor.wgsl
├── vector-fonts/           # PROMOTE from stub
│   ├── src/
│   │   ├── lib.rs          # public API: FontStack, GlyphKey, RasterizedGlyph
│   │   ├── loader.rs       # crossfont Rasterize::new + load_font from bundled path
│   │   └── width.rs        # unicode-width wrapper (D-43: cell width source)
├── vector-input/           # PROMOTE from stub
│   ├── src/
│   │   ├── lib.rs          # public API: encode_key, encode_mouse, encode_paste
│   │   ├── keymap.rs       # xterm key table — KeyEvent → Vec<u8>
│   │   ├── paste.rs        # bracketed paste wrapper
│   │   └── selection.rs    # grid-coord selection state machine
└── (others untouched: vector-term, vector-mux, vector-pty all Phase 2; vector-headless retained)
```

### Pattern 1: Single-actor `Term` ownership

**What:** `Arc<parking_lot::Mutex<Term>>` is owned by the `vector-app` event loop. A dedicated tokio "PTY reader actor" task on the I/O thread holds the only writable reference to the `Box<dyn PtyTransport>` and pumps reader bytes through to the main thread via `UserEvent::PtyOutput(BytesMut)`. The main thread locks `Term`, calls `feed()`, drops the lock, then calls `Window::request_redraw()`.

**When to use:** Always in Phase 3. This is exactly the pattern Phase 2's Plan 02-05 vector-headless proved (per 02-04-SUMMARY hand-off notes: "design your single-actor pattern so only one task holds the `&mut Box<dyn PtyTransport>` at a time").

**Why:** `Term: !Sync`. `Box<dyn PtyTransport>::write(&mut self)` requires exclusive borrow. Cross-thread mutable borrow → mutex. Lock + mutate + drop + `request_redraw` keeps `clippy::await_holding_lock = "deny"` (D-11) green.

**Example:**

```rust
// crates/vector-app/src/main.rs additions (sketch)
let term = Arc::new(parking_lot::Mutex::new(
    vector_term::Term::new(cols, rows, 10_000)
));
let term_for_io = Arc::clone(&term);

// I/O thread (existing tokio thread from Phase 1 main.rs:37-46)
rt.block_on(async move {
    let domain = vector_mux::LocalDomain::new()?;
    let mut transport = domain.spawn(SpawnCommand {
        argv: None, cwd: None, rows, cols, env: vec![]
    }).await?;
    let mut rx = transport.take_reader().expect("first call");

    // Actor: owns transport; receives writes from main via mpsc; pumps reader to main via proxy
    let (write_tx, mut write_rx) = mpsc::channel::<Vec<u8>>(64);
    // ... biased select! over rx + write_rx + resize_rx (Plan 02-05 pattern) ...
    while let Some(chunk) = rx.recv().await {
        // PTY-burst coalescing per D-47: accumulate to BytesMut, drain on tick or threshold
        proxy.send_event(UserEvent::PtyOutput(chunk))?;
    }
});

// Main thread (winit) — see app.rs::user_event below
```

```rust
// crates/vector-app/src/app.rs::user_event additions (sketch)
fn user_event(&mut self, _: &ActiveEventLoop, ev: UserEvent) {
    match ev {
        UserEvent::PtyOutput(chunk) => {
            {
                let mut t = self.term.lock();
                t.feed(&chunk);
            } // drop lock BEFORE request_redraw (and certainly before any .await — none in scope here)
            if let Some(w) = self.window.as_ref() { w.request_redraw(); }
        }
        UserEvent::Tick(n) => { /* legacy from Phase 1, keep briefly or delete */ }
        UserEvent::LpmChanged(enabled) => { self.lpm_throttle.set(enabled); /* D-46 */ }
    }
}
```

### Pattern 2: Render on `RedrawRequested`, never on a wall-clock timer

**What:** Hook `WindowEvent::RedrawRequested` (winit 0.30 — verified) to a render function that:
1. Locks the `Term`, calls `damage()` to get `TermDamage`, calls `reset_damage()`, drops the lock (read what we need under the lock, then release immediately — Pitfall 5 Anti-Pattern).
2. For each `LineDamageBounds { line, left, right }`, rebuilds that row's slice of the instance buffer.
3. Composites: cell pass → selection pass → cursor pass → submit.

**When to use:** Always. Render is event-driven from `request_redraw()` calls, which are themselves driven by (a) `UserEvent::PtyOutput` arriving, (b) `WindowEvent::Resized`, (c) `WindowEvent::ScaleFactorChanged`, (d) input that moves cursor/selection, (e) cursor blink timer (if implemented).

**Why:** Idle CPU < 1% (RENDER-03) requires zero work when nothing changed. Phase 1's `event_loop.set_control_flow(ControlFlow::Wait)` (main.rs:34) already cooperates — `Wait` blocks until an event arrives. ProMotion 120 Hz happens automatically because `PresentMode::Fifo` honors the display's vsync regardless of refresh rate.

### Pattern 3: Two atlases, RGBA8 + RGBA8

**What:** Two `wgpu::Texture` instances, both `Rgba8Unorm` (or `Rgba8UnormSrgb` for emoji), 2048×2048 each. One shelf-packed atlas for mono "RGB alphamask" glyphs (`crossfont::BitmapBuffer::Rgb`); one for color emoji (`BitmapBuffer::Rgba`). The cell shader has a branch on `attrs.is_emoji` to sample from the right atlas.

**When to use:** Always. CONTEXT D-40 + D-43.

**Why:** Mono and color glyphs have different shader paths (mono is multiplied by `fg_color`; emoji is sampled directly as RGBA — color comes from the bitmap). One pipeline per atlas is cleaner than a single texture with mixed formats.

**Source check:** Per crossfont docs (verified 2026-05-11), `BitmapBuffer::Rgb` is described as "RGB alphamask" — i.e., the 3 channels are anti-aliased coverage values, not RGB color. Mono atlas uploads these into the .rgb of an RGBA8 texture; alpha = max(r,g,b) or alpha = 1.0 (uniform). Fragment shader for mono: `out = vec4(fg_color.rgb * sample.rgb, fg_color.a)`. For emoji: `out = sample`.

**Atlas slot allocation:** `etagere::AtlasAllocator` is the recommended packer (used by glyphon/cosmic-term). On atlas-full → bump LRU eviction policy: a `VecDeque<GlyphKey>` access-ordered list, evict oldest until enough space frees; if a single glyph can't fit, log a warning and either skip it (fall back to `.` placeholder cell) or rebuild the atlas. Pitfall 2 explicitly mandates bounded eviction.

### Pattern 4: Damage iterator → vertex buffer update

**What:** `Term::damage()` returns `TermDamage::Partial(TermDamageIterator)` with items `LineDamageBounds { line, left, right }` (verified). For each bounds: rewrite the instance buffer's `(line * cols + left .. line * cols + right + 1)` slice from `Term::grid()`. For `TermDamage::Full`: rewrite the whole buffer. Use `queue.write_buffer(&instance_buf, offset, &cast_slice(&new_cells))` — wgpu's standard partial-buffer-write API.

**When to use:** Every frame that has non-empty damage.

**Why:** Rewriting only dirty rows is the difference between 60+ fps and stuttering at 30 fps on a `cat large.log`. Per-row writes also keep the GPU upload bandwidth bounded — even a 200-row terminal at 80 cols is 16,000 instances × ~32 bytes ≈ 512 KB worst case per frame, all-rows write. Per-row writes typically ship 1–5 rows.

### Pattern 5: Bundled font load path

**What:** `cargo-bundle` copies `crates/vector-app/resources/Fonts/JetBrainsMono-Regular.ttf` into `Vector.app/Contents/Resources/Fonts/` automatically (verified mechanism: `[package.metadata.bundle].resources` array in `Cargo.toml`). At runtime, locate the bundle via `[[NSBundle mainBundle] resourcePath]` (via objc2-foundation) or `std::env::current_exe()` walk-up — pick whichever is cleaner. Pass the path into `crossfont::FreeTypeRasterizer::new()` (or the CoreText-backed rasterizer on macOS — crossfont auto-selects per platform).

**When to use:** Once at app startup, before first frame.

**Why:** D-41 mandates bundling for snapshot-test determinism. CoreText shaping varies across macOS versions — we control the font bytes to keep PNG fixtures stable.

**Cargo-bundle resource wiring (confirmed in Phase 1 commit `4dd0c4e`):**

```toml
[package.metadata.bundle]
resources = ["resources/Fonts/JetBrainsMono-Regular.ttf"]
```

Phase 1's `xtask::dmg::finalize` post-process step (per STATE.md "Wave-0 cargo-bundle spike result") rebuilds the bundle; resources still ship correctly because they're copied during the per-arch `cargo bundle` invocations.

### Anti-Patterns to Avoid

- **Repainting the whole grid every frame.** Idle CPU jumps to 100% (Architecture.md Anti-Pattern 6). Always feed damage iterator into a partial vertex-buffer rewrite.
- **Allocating in the render hot loop.** Reuse `Vec<CellInstance>` capacity (e.g., `instances.clear(); instances.extend(...)` not `let mut v = Vec::new()`). Pre-allocate the instance buffer to `cols * rows` capacity at construction; resize only on grid resize.
- **Holding `Term` lock across an `await`.** Will trip `clippy::await_holding_lock = "deny"` at compile time. Lock → read damage + grid slices into owned Vec → drop lock → submit.
- **Re-creating wgpu pipelines on theme change.** Theme = uniform buffer update. Pipelines are stable per surface format + format combo (Performance Traps in PITFALLS.md).
- **`from_utf8_lossy` on a `Cell::c` field for rendering.** `cell.c` is `char` — already valid Unicode. No decoding needed. (Pitfall 4 is about PTY bytes, not grid cells; this distinction matters because the renderer's input is grid cells, not bytes.)
- **Subpixel AA branches per DPR.** Mojave disabled subpixel AA system-wide; grayscale only (D-50). Don't add `if dpr_fractional { … }` branches.
- **Bilinear filtering on mono glyph atlas.** Use `FilterMode::Nearest`. Bilinear blurs text at non-integer scales (Performance Traps).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| VT escape parsing | Custom state machine | `alacritty_terminal::Term` (already in Phase 2) | Pitfall 1. Settled in Phase 2. |
| Glyph rasterization | Custom CoreText FFI | `crossfont 0.9` | CoreText fallback chain, ligature shaping, color emoji, kerning all done. |
| Cell width measurement | Custom Unicode width tables | `unicode-width 0.2.2` | UAX #11 + emoji ZWJ + East Asian Width rules; ~1 KLOC of corner cases. Font advance LIES about CJK width. |
| Atlas packing | Custom shelf packer | `etagere 0.2` (or hand-rolled simple shelf if planner prefers — both are fine, etagere is cheaper to ship) | etagere is ~500 LOC dep with no transitive bloat; well-trodden. Hand-rolled simple shelf packing is ~150 LOC and also acceptable for v1. |
| Font fallback chain | Custom `NSFontDescriptor` walk | `crossfont::FontDesc` + CoreText | crossfont's CoreText path already runs the system font cascade; emoji falls through to Apple Color Emoji automatically. |
| LRU eviction | `lru` crate | Hand-rolled `VecDeque<GlyphKey>` access-ordered list | Optional — `lru` crate is ~50 LOC saved and adds a dep. **Researcher recommendation:** hand-roll with `VecDeque` + `HashMap` for the cache state; ~30 LOC, zero deps. Planner's call. |
| xterm key encoding | Custom ad-hoc table | Transcribe from `alacritty/src/input/keyboard.rs` (Apache-2.0; transcribed and attributed in a header comment) | The xterm key table is well-defined but voluminous (~80 keys × modifier combos). Alacritty's transcription is battle-tested. |
| Bracketed paste | Custom byte wrapping | 4-line constant: `format!("\x1b[200~{}\x1b[201~", paste_str.as_bytes())` | Trivial. Cite ECMA-48 + xterm extension. |
| Low Power Mode detection | Polling `pmset` subprocess | `NSProcessInfo::isLowPowerModeEnabled()` via objc2-foundation (verified API) | Apple's blessed primitive; observer-pattern for changes via NSNotificationCenter. |
| NSPasteboard read | Custom Carbon-era API | `NSPasteboard::generalPasteboard().stringForType(NSPasteboardTypeString)` (verified) | objc2-app-kit 0.3 exposes this directly. |
| Frame timing | Spin loop on CADisplayLink | `wgpu::PresentMode::Fifo` + winit `request_redraw` | CADisplayLink would be the lower-level primitive; wgpu's Fifo already hooks into Metal's CAMetalDisplayLink under the hood. (Confirmed in wgpu Metal backend source.) |

**Key insight:** Phase 3 is integration work. The hard things — VT parsing, font rasterization, cell width, OS power state, clipboard — all have one canonical Rust crate or one OS API. The only "from scratch" code Phase 3 owns is: (a) the cell→quad compositor, (b) the wgsl shaders, (c) the atlas-eviction policy, (d) the xterm key transcription (mechanical port from Alacritty source). Everything else is wiring.

## Runtime State Inventory

> Phase 3 is a greenfield code-add phase (no rename/refactor/migration). Section omitted.

## Common Pitfalls

### Pitfall 1: Glyph atlas churn across DPR change

**What goes wrong:** User drags the Vector window from an external 1080p monitor to a Retina internal display. Every glyph that was rasterized at 1×DPR now looks blurry because the texture coordinates point at low-res bitmaps stretched to 2× size. Or vice-versa: emoji rasterized at 2×DPR look chunky-sharp on a 1× display.

**Why it happens:** Glyph bitmaps are DPR-baked at rasterization time. CoreText returns different pixel counts at 1× vs 2×.

**How to avoid:** D-48's full-atlas invalidation on `WindowEvent::ScaleFactorChanged`. Both atlases clear; cell→slot maps clear; lazy re-rasterize on next reference. Pre-rasterizing for all DPRs was rejected — doesn't generalize to fractional DPRs (1.5×) and doubles memory.

**Warning signs:** Visible blur or jaggies after dragging the window between monitors. Look for a one-frame stutter as the atlas rebuilds (acceptable per success criterion #4: "no visible stutter beyond the first frame").

### Pitfall 2: Render path holds `Term` lock across an `.await`

**What goes wrong:** Code looks like:
```rust
let mut term = self.term.lock();
term.feed(&bytes);
some_async_op().await;  // BUG: lock held across await
```
Deadlocks when the I/O actor task tries to push the next chunk into `Term`, or when another input handler tries to write a key.

**Why it happens:** Subtle to spot in code review; easy to write by accident.

**How to avoid:** `clippy::await_holding_lock = "deny"` (already in workspace lints from Phase 1 D-11). Will fail the build. The fix is always the same shape: scope the lock guard tight, drop it before `.await`. Phase 2's Plan 02-05 vector-headless validated this in production.

**Warning signs:** Random UI hangs under heavy PTY output. `parking_lot::Mutex` doesn't deadlock-detect; symptom is just stalled.

### Pitfall 3: `Term::damage()` returns `&mut`, not `&`

**What goes wrong:** Plan assumes `RwLock<Term>` so render can read concurrently with input. But `damage()` is `&mut self` (alacritty quirk — confirmed). Render path needs a write lock, kills RwLock contention savings.

**Why it happens:** Reasonable design instinct ("rendering reads, input writes → RwLock") collides with alacritty's API choice.

**How to avoid:** Plain `parking_lot::Mutex<Term>`. The performance impact is negligible — the lock is held for ≤ 100 µs per frame (read damage + clone instance data → drop lock → render outside the lock).

**Warning signs:** Compile error: "cannot borrow `*self` as mutable" if anyone tries `term.read().damage()`.

### Pitfall 4: Cell character is `char`, not `&str` — emoji ZWJ assertion

**What goes wrong:** Renderer tries to look up an emoji ZWJ sequence (`U+1F468 ZWJ U+1F469 ZWJ U+1F467`, family-emoji = `man + ZWJ + woman + ZWJ + girl`) as a single glyph. alacritty's `Cell.c` is `char` (a single scalar), and ZWJ sequences span MULTIPLE cells with the `WIDE_CHAR_SPACER` and grapheme-cluster flags.

**Why it happens:** Newcomers assume one cell = one glyph.

**How to avoid:** Per 02-02 hand-off notes ("WIDE_CHAR cells"): when emitting/rendering, skip cells flagged `WIDE_CHAR_SPACER`; the glyph is drawn from the lead cell. For grapheme clusters that span more than 2 cells (some emoji), use `cell.zerowidth()` to get continuation characters. **Phase 3's compositor must skip WIDE_CHAR_SPACER cells in the cell pass.** Test fixture: family-emoji ZWJ sequence (man+ZWJ+woman+ZWJ+girl) + CJK U+4E2D row, assert pixel-correct rendering at 1× and 2× DPR.

**Warning signs:** Emoji renders as a single character followed by an empty cell (wrong); or every cell after an emoji shifts right by one column (worse).

### Pitfall 5: Phase-1 placeholder NSTextField overlapping the GPU surface

**What goes wrong:** The Phase-1 overlay (NSTextField anchored bottom-right via `overlay.rs`) renders on AppKit's compositor; the wgpu `CAMetalLayer` renders on Core Animation. If they're both on the same NSView, the overlay sits on top of the GPU surface — the "Vector v… (build …)" text shows over the terminal.

**Why it happens:** Layered AppKit subviews don't automatically cooperate with Metal layers.

**How to avoid:** D-51 dictates the overlay drops on first PTY byte. Mechanic: keep the NSTextField a subview of `contentView` (Phase 1 setup); on `UserEvent::PtyOutput(first_chunk)`, call `overlay._label.removeFromSuperview()` once and set `self.overlay = None`. The wgpu surface is configured directly on a Metal-backed sublayer — when the overlay is removed, the surface is alone on top.

**Warning signs:** "Vector v0.… (build abc1234)" text visible over running `vim`.

### Pitfall 6: Mouse/scroll wheel coords are pixel, not cell

**What goes wrong:** Scroll wheel delivers pixel deltas. User scrolls one notch; render shifts by 1 pixel instead of 1 row. Or worse: drag-selection in cell coordinates is computed by dividing `(pixel_x, pixel_y) / (cell_w, cell_h)` but `cell_w` is in physical pixels and the `MouseScrollDelta::PixelDelta` is in logical points → off by a DPR factor.

**Why it happens:** winit gives `PhysicalPosition<f64>` for `CursorMoved`, but `MouseScrollDelta::PixelDelta` is logical. `MouseScrollDelta::LineDelta` is in "lines" already (macOS native) — preferred.

**How to avoid:** For scroll: prefer `MouseScrollDelta::LineDelta(_, y)`; round to integer scrollback offset deltas. For drag-selection: store anchor and current in cell coordinates (computed from `PhysicalPosition / (cell_pixel_w, cell_pixel_h)`). Test at 1× and 2× DPR explicitly.

**Warning signs:** Scroll feels too fast or too slow; selection rectangle slides off the cells when dragging on Retina.

### Pitfall 7: `cargo-bundle` re-runs `cargo build` and clobbers the universal binary

**What goes wrong:** Already-known from Phase 1 (per STATE.md Wave-0 cargo-bundle spike). The fix is documented in ADR 0004 and lives in `xtask::dmg::finalize`. Phase 3 does NOT need to revisit, but the planner should be aware that resource bundling (the JetBrains Mono TTF) interacts with this code path — verify the TTF still ends up in `Vector.app/Contents/Resources/Fonts/` after the universal-binary post-process step.

**How to avoid:** After Phase 3's first DMG build, manually `unzip -l Vector-*.dmg` (or mount + `find`) and verify `Vector.app/Contents/Resources/Fonts/JetBrainsMono-Regular.ttf` exists. Add to Plan 03-02 acceptance criteria.

**Warning signs:** Vector launches and panics with "font not found at /…/Resources/Fonts/JetBrainsMono-Regular.ttf".

## Code Examples

Verified patterns from official sources.

### wgpu 29 Surface creation from winit 0.30 Window

```rust
// Source: https://docs.rs/wgpu/29.0.3/wgpu/struct.Instance.html#method.create_surface
// (verified 2026-05-11)
let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
    backends: wgpu::Backends::METAL,
    ..Default::default()
});
let surface = instance.create_surface(window)?;  // window: &winit::window::Window
let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
    power_preference: wgpu::PowerPreference::HighPerformance,
    compatible_surface: Some(&surface),
    force_fallback_adapter: false,
}))?;
let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
    required_features: wgpu::Features::empty(),
    required_limits: wgpu::Limits::default(),
    label: Some("vector-render-device"),
    memory_hints: wgpu::MemoryHints::Performance,
    trace: wgpu::Trace::Off,
}))?;
let caps = surface.get_capabilities(&adapter);
let format = caps.formats[0];  // first is sRGB-preferred on Metal
let size = window.inner_size();
surface.configure(&device, &wgpu::SurfaceConfiguration {
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    format,
    width: size.width.max(1),
    height: size.height.max(1),
    present_mode: wgpu::PresentMode::Fifo,   // D-45
    alpha_mode: wgpu::CompositeAlphaMode::Auto,
    view_formats: vec![],
    desired_maximum_frame_latency: 2,
});
```

### alacritty_terminal damage iteration

```rust
// Source: https://docs.rs/alacritty_terminal/0.26.0/alacritty_terminal/term/struct.Term.html#method.damage
// (verified 2026-05-11)
use alacritty_terminal::term::TermDamage;

let mut term = self.term.lock();
let damage_info: Vec<(usize, usize, usize)> = match term.damage() {
    TermDamage::Full => {
        let (cols, rows) = term.dims();
        (0..rows as usize).map(|r| (r, 0, cols as usize - 1)).collect()
    }
    TermDamage::Partial(iter) => {
        iter.map(|b| (b.line, b.left, b.right)).collect()
    }
};
term.reset_damage();
// Snapshot the cells we need under the lock too, then DROP the lock before render.
let cells_snapshot: Vec<(usize, Vec<alacritty_terminal::term::cell::Cell>)> = damage_info
    .iter()
    .map(|&(line, l, r)| {
        let row_cells = (l..=r).map(|col| {
            use alacritty_terminal::index::{Column, Line, Point};
            term.grid()[Point::new(Line(line as i32), Column(col))].clone()
        }).collect::<Vec<_>>();
        (line, row_cells)
    })
    .collect();
let cursor = term.cursor();
drop(term);  // CRITICAL: release lock before any potential await downstream.

// Now compose outside the lock.
renderer.update_rows(&cells_snapshot, cursor);
renderer.draw(&surface, &device, &queue)?;
```

### crossfont rasterizer construction

```rust
// Source: https://docs.rs/crossfont/0.9.0/crossfont/trait.Rasterize.html
// (verified 2026-05-11)
use crossfont::{Rasterize, Rasterizer, FontDesc, Slant, Style, Weight, Size, GlyphKey};

let mut rasterizer = Rasterizer::new(device_pixel_ratio)?;
let desc = FontDesc::new(
    "JetBrains Mono",
    Style::Description { slant: Slant::Normal, weight: Weight::Normal },
);
let font_key = rasterizer.load_font(&desc, Size::new(14.0))?;
let glyph = rasterizer.get_glyph(GlyphKey {
    character: 'A',
    font_key,
    size: Size::new(14.0),
})?;
// glyph.buffer is crossfont::BitmapBuffer::Rgb(Vec<u8>) for mono,
// or BitmapBuffer::Rgba(Vec<u8>) for emoji (verified docs.rs).
match glyph.buffer {
    crossfont::BitmapBuffer::Rgb(bytes) => { /* upload to mono atlas */ }
    crossfont::BitmapBuffer::Rgba(bytes) => { /* upload to emoji atlas */ }
}
```

### winit KeyEvent → xterm bytes (abbreviated)

```rust
// Pattern reference: alacritty/src/input/keyboard.rs (Apache-2.0) — transcribed into vector-input/src/keymap.rs
// winit KeyEvent struct fields (verified docs.rs 2026-05-11): physical_key, logical_key, text, location, state, repeat.
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};

fn encode_key(ev: &winit::event::KeyEvent, mods: ModState) -> Option<Vec<u8>> {
    if ev.state != winit::event::ElementState::Pressed { return None; }
    let mod_param = encode_mod_param(mods);  // 1=none, 2=Shift, 3=Alt, 5=Ctrl, etc.
    match ev.logical_key.as_ref() {
        Key::Named(NamedKey::ArrowUp) => Some(csi_seq(mod_param, b'A')),
        Key::Named(NamedKey::ArrowDown) => Some(csi_seq(mod_param, b'B')),
        Key::Named(NamedKey::ArrowRight) => Some(csi_seq(mod_param, b'C')),
        Key::Named(NamedKey::ArrowLeft) => Some(csi_seq(mod_param, b'D')),
        Key::Named(NamedKey::Home) => Some(csi_seq(mod_param, b'H')),
        Key::Named(NamedKey::End) => Some(csi_seq(mod_param, b'F')),
        Key::Named(NamedKey::F1) => Some(b"\x1bOP".to_vec()),  // sample
        // … full table per xterm spec; ~80 entries
        Key::Character(s) if mods.option => {
            // macOS Option-key default: ESC + char
            let mut buf = vec![0x1b]; buf.extend_from_slice(s.as_bytes()); Some(buf)
        }
        Key::Character(s) => {
            if let Some(text) = ev.text.as_deref() {
                Some(text.as_bytes().to_vec())
            } else { None }
        }
        _ => None,
    }
}

fn csi_seq(mod_param: u8, finalizer: u8) -> Vec<u8> {
    if mod_param == 1 {
        vec![0x1b, b'[', finalizer]   // ESC [ A
    } else {
        vec![0x1b, b'[', b'1', b';', b'0' + mod_param, finalizer]  // ESC [ 1;2 A
    }
}
```

### NSPasteboard read (Cmd-V) via objc2-app-kit

```rust
// Source: https://docs.rs/objc2-app-kit/0.3/objc2_app_kit/struct.NSPasteboard.html
// (verified 2026-05-11)
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::NSString;

fn read_clipboard_string(mtm: objc2::MainThreadMarker) -> Option<String> {
    let pb = unsafe { NSPasteboard::generalPasteboard() };
    let ns_str = unsafe { pb.stringForType(NSPasteboardTypeString) }?;
    Some(ns_str.to_string())
}

// Paste handler (Cmd-V):
let text = read_clipboard_string(mtm)?;
let mut bytes = b"\x1b[200~".to_vec();
bytes.extend_from_slice(text.as_bytes());
bytes.extend_from_slice(b"\x1b[201~");
write_to_pty.blocking_send(bytes).ok();
```

### NSProcessInfo low-power-mode observation (D-46)

```rust
// Source: objc2-foundation 0.3 NSProcessInfo (verified 2026-05-11)
use objc2_foundation::{NSProcessInfo, NSNotificationCenter};

fn is_low_power_mode() -> bool {
    unsafe { NSProcessInfo::processInfo().isLowPowerModeEnabled() }
}

// Observer: see objc2_foundation::NSNotificationCenter::defaultCenter
//   .addObserverForName_object_queue_usingBlock(...)
// Block needs to be a `RcBlock<...>`-wrapped Rust closure; the closure sends
// UserEvent::LpmChanged(now_enabled) via EventLoopProxy.
// Notification name constant: "NSProcessInfoPowerStateDidChangeNotification"
```

**Caveat (MEDIUM confidence):** I have NOT verified the exact objc2-foundation `addObserverForName_*` block API — there are multiple shapes across objc2 versions. Planner should spend ~30 min in Plan 03-05 verifying the block-based observer API before committing. Fallback: poll `isLowPowerModeEnabled()` once per second on a tokio timer — wasteful but simple. Pick the observer when the API is confirmed.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Subpixel anti-aliasing on macOS | Grayscale-only AA | Mojave (10.14), 2018 | One code path; no DPR-conditional rendering branches (D-50). |
| `cocoa-rs` + `objc` crates | `objc2` + typed `objc2-app-kit` 0.6/0.3 | 2024 | Already adopted in Phase 1. |
| OpenGL on macOS | Metal-only | macOS 10.14+ deprecated, removed in 14.0 | wgpu Metal backend is the only path. Alacritty has NOT migrated (still uses glutin/OpenGL); we leapfrog them. |
| `wgpu::SurfaceTexture::present()` polled in a render thread | `winit::Window::request_redraw()` event-driven on the main thread | wgpu 0.18+ / winit 0.30 | Pattern lock. (We adopt this.) |
| `harfbuzz_rs` | Crossfont (CoreText) for terminals OR cosmic-text/swash for rich text | 2023 | `harfbuzz_rs` is stale (Aug 2021). Avoid. |
| Single combined atlas | Two atlases (mono + color emoji) | ~2020 (Alacritty + iTerm both do this) | Different texture formats; different shader paths. Pitfall 2. |
| ProMotion 60-Hz hardcoded | wgpu `PresentMode::Fifo` honors display vsync regardless | wgpu since 0.10 | We get 120 Hz on ProMotion for free. |

**Deprecated/outdated:**

- `harfbuzz_rs 2.0.1` — stale (last release Aug 2021). Avoid (CLAUDE.md). Crossfont's CoreText path handles shaping internally.
- `tokio_pty_process` — Phase 2 already moved off this to `portable-pty`.
- `objc 0.2` and `cocoa-rs` — Phase 1 already on `objc2 0.6` + `objc2-app-kit 0.3`.
- `glutin` — Alacritty uses it for OpenGL on macOS, but Metal-only is our v1 baseline; no need.

## Open Questions

1. **Does `vector-term::Term` need a `pub fn damage(&mut self) -> TermDamage<'_>` method?**
   - What we know: `Term: alacritty_terminal::Term` has `damage()` and `reset_damage()` confirmed. `vector_term::Term::inner()` is `pub(crate)` (per `term.rs:73`) — not accessible to `vector-render`.
   - What's unclear: whether to expose `Term::damage` + `Term::reset_damage` as a `&mut self` pair on the wrapper, or expose a more idiomatic `Term::drain_damage(&mut self) -> Vec<LineDamageBounds>` that takes ownership of the data and resets internally.
   - Recommendation: **Plan 03-01 adds two `&mut self` methods on `vector_term::Term`:** `pub fn damage(&mut self) -> alacritty_terminal::term::TermDamage<'_>` and `pub fn reset_damage(&mut self)`. Re-export `TermDamage`, `TermDamageIterator`, `LineDamageBounds` from `vector_term` so `vector-render` doesn't need a direct dep on `alacritty_terminal`. Total addition: ~10 LOC; surface-area change in `vector-term` is small and reversible.

2. **Should `vector-render` depend on `vector-term`, or should the compositor be in `vector-app`?**
   - What we know: Architecture.md positions `vector-render` as "read-only access to grid; renderer never mutates terminal state". Damage iteration requires `&mut Term`, which contradicts that exactly. But the only mutation is `reset_damage()` — semantically still "render-only state".
   - Recommendation: **`vector-render` depends on `vector-term`.** The compositor takes `&mut Term` (via the locked Mutex's guard, passed by the caller). This keeps the rendering logic colocated with the wgsl shaders and atlas code, and matches WezTerm's split (`wezterm-gui` includes the compositor). `vector-app` is just the lifecycle host (winit handler + Surface creation + lock-acquire→pass-to-renderer).

3. **Does `cargo-bundle 0.10` ship resources from a `resources/Fonts/` subdirectory correctly?**
   - What we know: Phase 1's `[package.metadata.bundle].resources` already ships `resources/Info.plist.partial` (per cargo-bundle's `resources` array — files relative to `Cargo.toml`). The mechanism is "copy files into `Vector.app/Contents/Resources/`".
   - What's unclear: whether a path like `resources/Fonts/JetBrainsMono-Regular.ttf` ends up at `…/Contents/Resources/Fonts/JetBrainsMono-Regular.ttf` (preserves subpath) or `…/Contents/Resources/JetBrainsMono-Regular.ttf` (flattens). cargo-bundle docs suggest preservation, but the Wave-0 spike rebuilt the bundler logic so we should re-test.
   - Recommendation: **Plan 03-02 wave-0 spike (or Plan 03-02 Task 1) writes a 3-line shell test that builds the DMG, mounts it, and `find Vector.app/Contents/Resources -name "*.ttf"` to confirm the path.** If flattened, drop the `Fonts/` subdir and put the TTF at top-level `resources/`. Either layout is fine; pick what cargo-bundle gives us.

4. **What's the wgpu Metal backend's behavior on `SurfaceError::Outdated` after a DPR change?**
   - What we know: D-48 invalidates atlases on ScaleFactorChanged. `wgpu::Surface::get_current_texture()` can return `SurfaceError::Outdated` after surface resize.
   - What's unclear: ordering with `surface.configure()` calls — on Resized we want surface.configure(new size); on ScaleFactorChanged the inner_size CHANGES too (winit gives `inner_size_writer` to override).
   - Recommendation: **In Plan 03-05, handle `SurfaceError::{Lost, Outdated}` by calling `surface.configure(...)` once and retrying `get_current_texture()` once.** Standard wgpu pattern.

5. **Selection rendering: scissor rect vs. translucent quad?**
   - What we know: Selection state is grid-cell-range; selection rect needs to composite over the live grid with alpha.
   - What's unclear: whether to use `wgpu::RenderPass::set_scissor_rect` (rectangular clip) vs. a translucent quad in a separate pipeline.
   - Recommendation: **Plan 03-04: translucent quad in its own pipeline.** Scissor only handles rectangles aligned to viewport axes; arbitrary multi-row selections that wrap at end-of-line need either (a) one quad per visible selection row, or (b) a per-cell flag in the cell pipeline that brightens the cell's bg. **(b) is simpler:** in the cell pipeline, add a `selected: u32` bit to `CellInstance`; fragment shader blends the cell's bg toward selection-tint when set. Saves a draw call, simpler resize/scroll handling.

## Environment Availability

Phase 3 adds no new external CLI tools, services, or runtimes. All dependencies are crates.io packages (verified above) or already-bundled OS frameworks (Metal, AppKit, CoreText). The macOS development host (Apple Silicon per Phase 1 checkpoint) is the only required environment.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Apple Silicon or Intel Mac w/ Metal-capable GPU | wgpu Metal backend | ✓ (project constraint: macOS 13+) | — | — |
| macOS 13+ system fonts (Apple Color Emoji) | Emoji fallback in CoreText cascade | ✓ (system) | — | If user has removed system emoji font (unusual), emoji cells render as `?` — accept gracefully. |
| Xcode CLI tools (already required by Phase 1: `lipo`, `hdiutil`) | Universal binary + DMG | ✓ (Phase 1 dependency) | — | — |
| `cargo-bundle 0.10` | Bundle JetBrains Mono TTF in `Vector.app/Contents/Resources/Fonts/` | ✓ (Phase 1 installed) | 0.10.0 | — |

**Missing dependencies with no fallback:** None.
**Missing dependencies with fallback:** None.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (standard libtest) — already in use across Phases 1+2. |
| Config file | None — Cargo.toml plus per-crate `Cargo.toml`'s `[lints]` and `[[test]]` sections. |
| Quick run command | `cargo test -p vector-render -p vector-fonts -p vector-input -p vector-app --tests` |
| Full suite command | `cargo test --workspace --tests` (existing Phase 1+2 contract: 26 vector-term + 5 vector-pty + 8 vector-mux + arch-lints + Phase 3 additions; total ≈ 100 tests after phase close) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| RENDER-01 | wgpu Metal pipeline initializes; damage iter compiles | unit | `cargo test -p vector-render --test pipeline_init` | ❌ Wave 0 (Plan 03-01) |
| RENDER-01 | Damage iterator processed end-to-end (Term::feed → damage → atlas → vertex buffer) | integration (headless wgpu) | `cargo test -p vector-render --test damage_to_quads` | ❌ Wave 0 (Plan 03-03) |
| RENDER-02 | `cat large.log` sustains ≥ 60 fps on Apple Silicon | **manual-only** (smoke matrix; Activity Monitor + FPS overlay) | n/a — record in Plan 03-05 acceptance | n/a |
| RENDER-02 | PTY-burst coalescing (D-47): one feed per drain window | unit | `cargo test -p vector-render --test pty_coalesce` | ❌ Wave 0 (Plan 03-05) |
| RENDER-03 | Idle CPU < 1% (no redraw without dirty rows) | **manual-only** (Activity Monitor over 60s) | n/a — record in Plan 03-05 acceptance | n/a |
| RENDER-03 | `request_redraw` is never called when damage is empty AND no input fires | unit (counter assert) | `cargo test -p vector-render --test idle_no_redraw` | ❌ Wave 0 (Plan 03-05) |
| RENDER-04 | Two atlases (mono + emoji) populated separately | unit | `cargo test -p vector-fonts --test two_atlas_split` | ❌ Wave 0 (Plan 03-02) |
| RENDER-04 | LRU eviction kicks in at capacity | unit | `cargo test -p vector-fonts --test atlas_lru_eviction` | ❌ Wave 0 (Plan 03-02) |
| RENDER-04 | `ScaleFactorChanged` clears atlases | unit (mock event) | `cargo test -p vector-render --test dpr_change_atlas_reset` | ❌ Wave 0 (Plan 03-05) |
| RENDER-04 | Retina ↔ non-Retina visual smoke | **manual-only** (drag window between monitors) | n/a — record in Plan 03-05 acceptance | n/a |
| RENDER-05 | Cursor rendered as block over live grid | snapshot (PNG diff with tolerance) | `cargo test -p vector-render --test cursor_overlay_snapshot` | ❌ Wave 0 (Plan 03-03) |
| RENDER-05 | Selection rect composited (1-row, 3-row wrapping) | snapshot | `cargo test -p vector-render --test selection_overlay_snapshot` | ❌ Wave 0 (Plan 03-04) |
| WIN-01 | Native AppKit window opens with title bar + close + minimize + fullscreen | unit (existence check on NSWindow style mask) | `cargo test -p vector-app --test win_style_mask` | ❌ Wave 0 (Plan 03-01) |
| WIN-01 | Cmd-Ctrl-F (fullscreen) wired to `toggleFullScreen:` selector | manual smoke | n/a — already shipping in Phase 1 menu.rs:117 | ✓ Phase 1 |
| Cross-cutting | Workspace builds + clippy clean + fmt clean (arch invariants hold: 15 `no_tokio_main.rs` files) | gate | `cargo build --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check` | ✓ existing |
| D-52 | xterm key encoding ~80 cases | unit | `cargo test -p vector-input --test xterm_key_table` | ❌ Wave 0 (Plan 03-04) |
| D-53 | Bracketed paste wraps clipboard text | unit | `cargo test -p vector-input --test bracketed_paste_wrap` | ❌ Wave 0 (Plan 03-04) |

### Sampling Rate

- **Per task commit:** `cargo test -p {affected-crate} --tests && cargo clippy -p {affected-crate} --all-targets -- -D warnings && cargo fmt --all -- --check`
- **Per wave merge:** `cargo test --workspace --tests && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check && cargo build --workspace`
- **Phase gate:** Full suite green + manual smoke matrix (vim, `cat large.log`, drag window between monitors, Cmd-V paste, drag-select, arrow keys, Cmd-Ctrl-F fullscreen, idle CPU check) before `/gsd:verify-work`.

### Wave 0 Gaps

- [ ] `crates/vector-render/tests/pipeline_init.rs` — covers RENDER-01 (wgpu Metal initialization + pipeline compilation)
- [ ] `crates/vector-render/tests/damage_to_quads.rs` — covers RENDER-01 (damage iter → atlas hits → vertex buffer write end-to-end, headless wgpu)
- [ ] `crates/vector-render/tests/pty_coalesce.rs` — covers RENDER-02 (PTY burst coalescing throttle)
- [ ] `crates/vector-render/tests/idle_no_redraw.rs` — covers RENDER-03 (no redraw when nothing dirty)
- [ ] `crates/vector-fonts/tests/two_atlas_split.rs` — covers RENDER-04 (mono vs emoji routing per `BitmapBuffer` variant)
- [ ] `crates/vector-fonts/tests/atlas_lru_eviction.rs` — covers RENDER-04 (eviction at capacity)
- [ ] `crates/vector-render/tests/dpr_change_atlas_reset.rs` — covers RENDER-04 (DPR change clears atlases)
- [ ] `crates/vector-render/tests/cursor_overlay_snapshot.rs` — covers RENDER-05 (PNG snapshot under bundled JetBrains Mono)
- [ ] `crates/vector-render/tests/selection_overlay_snapshot.rs` — covers RENDER-05 (PNG snapshot with selection rect)
- [ ] `crates/vector-app/tests/win_style_mask.rs` — covers WIN-01 (NSWindow style mask contains close|miniaturizable|resizable|titled, fullscreen-via-menu)
- [ ] `crates/vector-input/tests/xterm_key_table.rs` — covers D-52 (~80 key cases)
- [ ] `crates/vector-input/tests/bracketed_paste_wrap.rs` — covers D-53
- [ ] `crates/vector-input/tests/no_tokio_main.rs` — already exists from Phase 1; verify it still holds after Phase 3 additions (no new tokio direct deps in vector-input)
- [ ] Test fixture: `crates/vector-render/tests/fixtures/` directory for PNG snapshots; pick a perceptual-tolerance comparator (HARDEN-01 mandates `insta`-style + perceptual diff, but Phase 10 owns the gate — Phase 3 ships fixtures + comparison helper, gate becomes mandatory in Phase 10)
- [ ] Framework install: none needed — `cargo test` already wired.

**Manual-only verifications (record in Plan 03-05 acceptance criteria, sign-off in the manual smoke matrix):**

- [ ] `vim` opens, edits a buffer, quits cleanly (alt-screen, cursor changes, selection, RENDER-05)
- [ ] `cat large.log` sustains ≥ 60 fps measured via Instruments or `tracing` frame-time logs (RENDER-02)
- [ ] Activity Monitor: idle CPU < 1% over 60s with cursor blink running (RENDER-03)
- [ ] Drag window between Retina internal display and non-Retina external monitor, observe no broken glyphs (RENDER-04)
- [ ] Cmd-V paste of multi-line text into `bash -c 'read -p "> " x; echo "got: $x"'` arrives bracketed (D-53)
- [ ] Drag-select 3 rows, selection rect visible against dark theme; arrow keys move cursor without selection clearing visibly (RENDER-05)
- [ ] Cmd-Ctrl-F fullscreen toggles cleanly (WIN-01)

## Sources

### Primary (HIGH confidence)

- **crates.io API:**
  - https://crates.io/api/v1/crates/wgpu → wgpu 29.0.3, published 2026-05-02, MSRV 1.87 (verified 2026-05-11)
  - https://crates.io/api/v1/crates/crossfont → crossfont 0.9.0, published 2025-06-09, MSRV 1.77 (verified 2026-05-11)
  - https://crates.io/api/v1/crates/unicode-width → unicode-width 0.2.2, published 2025-10-06 (verified 2026-05-11)
- **docs.rs (verified 2026-05-11):**
  - https://docs.rs/alacritty_terminal/0.26.0/alacritty_terminal/term/struct.Term.html — `damage()` + `reset_damage()` confirmed
  - https://docs.rs/alacritty_terminal/0.26.0/alacritty_terminal/term/enum.TermDamage.html — `Full | Partial(iter)` confirmed
  - https://docs.rs/alacritty_terminal/0.26.0/alacritty_terminal/term/struct.LineDamageBounds.html — `{ line, left, right }: usize` confirmed
  - https://docs.rs/winit/0.30.13/winit/event/enum.WindowEvent.html — `ScaleFactorChanged { scale_factor, inner_size_writer }`, `Resized(PhysicalSize<u32>)`, `KeyboardInput`, `MouseInput`, `MouseWheel`, `Ime`, `RedrawRequested` confirmed
  - https://docs.rs/winit/0.30.13/winit/event/struct.KeyEvent.html — fields `physical_key/logical_key/text/location/state/repeat` confirmed
  - https://docs.rs/crossfont/0.9.0/crossfont/enum.BitmapBuffer.html — `Rgb(Vec<u8>) | Rgba(Vec<u8>)` confirmed (no Grayscale variant)
  - https://docs.rs/crossfont/0.9.0/crossfont/trait.Rasterize.html — `new/metrics/load_font/get_glyph/kerning` confirmed
  - https://docs.rs/wgpu/29.0.3/wgpu/struct.Instance.html — `create_surface(impl Into<SurfaceTarget>)` confirmed
  - https://docs.rs/wgpu/29.0.3/wgpu/enum.PresentMode.html — `Fifo` is the default, supported everywhere; verified 6 variants
  - https://docs.rs/objc2-app-kit/0.3/objc2_app_kit/struct.NSPasteboard.html — `generalPasteboard()` + `stringForType(...)` confirmed
- **In-repo source (HIGH):**
  - `crates/vector-term/src/term.rs` — current public API surface
  - `crates/vector-mux/src/local_domain.rs` — LocalDomain.spawn → Box<dyn PtyTransport> shape
  - `crates/vector-app/src/{main.rs, app.rs, menu.rs, overlay.rs, tick.rs}` — Phase 1 skeleton, what we extend
  - `Cargo.toml` — current workspace pins
- **CLAUDE.md** — Tech Stack Recommendations (wgpu 29, crossfont 0.9 path)
- **STACK.md / PITFALLS.md / ARCHITECTURE.md** — `.planning/research/` curated context

### Secondary (MEDIUM confidence)

- **NSProcessInfo notification observer API** — `processInfoPowerStateDidChangeNotification` exists as a string; the exact objc2-foundation API for registering a block observer needs ~30 min of API exploration in Plan 03-05 (LOW→MEDIUM until verified). Fallback: 1-Hz poll.
- **`cargo-bundle 0.10` subdir resource preservation** — assumed to preserve subpaths from `resources` array; Wave-0 spike-test recommended (Open Question #3).
- **JetBrains Mono OFL license compatibility with redistribution in `.app` bundle** — OFL allows embedding; standard practice for Alacritty, ghostty, WezTerm. (No URL fetched; treat as MEDIUM until license file added to Plan 03-02 resources.)

### Tertiary (LOW confidence)

- **`etagere 0.2` API specifics** — not fetched in this research; CONTEXT says "researcher's call". Atlas packing is well-trodden; planner can use etagere or hand-roll a shelf packer. Either acceptable.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every pin verified against crates.io live API on 2026-05-11.
- Architecture: HIGH — patterns match Phase 1/2 established threading + ownership; damage iteration API confirmed against docs.rs.
- Pitfalls: HIGH — atlas churn, lock-across-await, DPR handling, mouse coords all sourced from PITFALLS.md + Architecture.md + cross-referenced with live docs.
- Code examples: HIGH for wgpu init, damage iteration, NSPasteboard read; MEDIUM for NSProcessInfo notification observer (block API needs verification spike).
- Validation architecture: HIGH — `cargo test` is the universal test runner; PNG snapshot fixtures + bundled font give us determinism; Phase 10 owns the perceptual gate.

**Research date:** 2026-05-11
**Valid until:** 2026-06-10 (30 days for the stable parts: crossfont/objc2/alacritty_terminal). wgpu 29 → wgpu 30 release would invalidate Surface creation code samples but not patterns; re-verify on each wgpu major bump.
