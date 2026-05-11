---
phase: 03-gpu-renderer-first-paint
gathered: 2026-05-11
status: Ready for planning
discuss_mode: discuss
---

# Phase 3: GPU Renderer & First Paint — Context

**Gathered:** 2026-05-11
**Status:** Ready for planning

<domain>
## Phase Boundary

Replace Phase 1's text-overlay placeholder with a real `wgpu` pipeline that renders `alacritty_terminal::Term.grid()` into the existing winit/AppKit NSWindow. End-state: launching `Vector.app` opens a single-window-single-tab-single-pane GPU-rendered terminal where `vim` runs at sustained 60+ fps on Apple Silicon. Input → PTY plumbing for that one window is in scope; tabs, splits, mux, and search overlay are **not**.

**Covers requirements:** RENDER-01, RENDER-02, RENDER-03, RENDER-04, RENDER-05, WIN-01

**Explicitly out of phase (deferred):**
- Tabs, splits, focus routing → Phase 4 (mux)
- Cmd-F search overlay → Phase 5 (Polish), per D-39
- Cmd-C copy + selection-string semantics → Phase 5 (Polish) — Phase 3 has selection *rendering* only
- IME composition window UI → not in v1 (ROADMAP out-of-scope)
- Sixel / kitty graphics → not in v1
- Mouse-reporting modes (DEC 1006/1015/1016 → PTY) → Phase 5
- Per-domain ligature switching → Phase 5+ (single global toggle in v1)

</domain>

<decisions>
## Implementation Decisions

### Font + atlas stack

- **D-40:** **`crossfont 0.9` + hand-rolled two-atlas (mono SDF/grayscale + RGBA emoji), bounded LRU.** Terminal-tuned, proven in Alacritty, exact match for Pitfall 2's prescription. Reject cosmic-text+glyphon as the primary stack — it's a great general-text engine but overkill for a cell-grid renderer and would force shape/layout passes our cells don't need. swash/cosmic-text can be reconsidered in v2 if we add a settings/AI panel that needs rich text.
- **D-41:** **JetBrains Mono = bundled font (in `Vector.app/Contents/Resources/Fonts/`) AND default user-visible font.** Single artifact, OFL license. Bundling is mandatory for snapshot-test determinism — CoreText shaping is not version-stable across macOS releases, so reproducible glyph output requires we control the font bytes. The user can override via TOML config (`[font].family = "..."`); fallback chain still goes through CoreText.
- **D-42:** **Ligatures off by default; opt-in via `[font].ligatures = true`.** Matches Pitfall 2's guidance and keeps Phase 3 shaping-free (crossfont without GSUB). When a user turns it on later (or v2), we add `swash` for shaping. Single global toggle in v1 — no per-domain (local vs remote) switching.
- **D-43:** **Per-atlas bounded LRU eviction.** Each atlas (mono + emoji) caps at a fixed size (e.g., 2048×2048; final value is a planner-level call). On overflow, evict least-recently-used glyph slots. Width measured via the `unicode-width` crate, never via font advance.

### Frame pacing + dirty detection

- **D-44:** **Dirty detection uses `alacritty_terminal::Term::damage()` API + our own dirty-rows bitmap.** Alacritty already knows what rows changed since last clear — we wrap that into a bitmap consumed by the renderer to redraw only dirty rows. Per-row granularity (not per-cell, not full-screen) is the right size for cell terminals. If a future alacritty version changes the damage contract, this is the seam to refactor.
- **D-45:** **`wgpu::PresentMode::Fifo` everywhere; honor whatever vsync gives us.** Render-on-dirty + Fifo together mean idle costs zero and ProMotion 120Hz displays automatically get 120 fps on dirty frames. No FPS cap, no FPS floor, no special-casing for 60 vs 120 Hz displays. Satisfies success criterion #2 (ProMotion honored).
- **D-46:** **Low Power Mode cap = 30 fps; logged via `tracing`.** Detect via `NSProcessInfo.lowPowerModeEnabled` plus the `processInfoPowerStateDidChange` notification. Throttle by skipping render ticks; we do not suspend the renderer entirely (iTerm2's choice) because that desyncs logs that scroll invisibly. Trace event makes the cap visible in diagnostics.
- **D-47:** **PTY-burst coalescing: I/O thread accumulates reads into a `BytesMut`; drains to main once per ~8 ms tick (half-frame at 120 Hz) OR when buffer hits a size threshold.** Main calls `Term::feed()` exactly once per drain. Result: `cat large.log` produces one feed-and-render per vsync, not thousands. Coalesces naturally with D-44's dirty-row detection. Interactive keystroke latency stays ≤ one frame.

### Display change / DPR robustness

- **D-48:** **On `ScaleFactorChanged` event: clear both atlases + invalidate cell→slot map; lazy re-rasterize per glyph on first reference in next frame.** Matches Alacritty's behavior. Acceptable one-frame stutter (success criterion #4 explicitly allows "no visible stutter beyond the first frame"). Pre-rasterizing both DPRs upfront is rejected — doubles atlas memory, doesn't generalize to non-standard fractional DPRs (1.5x scaled externals), and Mojave-onward grayscale AA already makes lazy rasterization sharp enough.
- **D-49:** **Live window resize: debounce `Term::resize()` to ~50 ms quiescent / on resize-end; wgpu surface resizes on every event (cheap); repaint with current grid between events.** Reflow of scrollback is O(rows) so we don't want to fire it per Resized event during a drag. Surface resize is constant-cost and keeps the visual smooth.
- **D-50:** **Grayscale AA via CoreText, always.** Apple disabled subpixel AA system-wide in Mojave (10.14); modern macOS displays don't need it. crossfont via CoreText rasterizes to a single 8-bit channel, matches our mono atlas pixel format perfectly. One code path, no DPR-conditional rendering branches.
- **D-51:** **First real GPU frame paints after: shell spawned (`LocalDomain::spawn`) + first PTY read arrived + font loaded into atlas + first row marked dirty.** The Phase 1 placeholder ("Vector v{version} (build {sha})" `NSTextField`) may remain visible for one frame as the window opens; immediately replaced by the GPU surface once those four conditions are met. No empty-grid flash, no cursor-at-(0,0) before shell prompt.

### Input handling scope

- **D-52:** **Full xterm-compatible keyboard mapping.** Printable ASCII/Unicode + Esc + arrow keys + F1–F12 + navigation (Home, End, PgUp, PgDn) + modifier combinations (Shift / Option / Ctrl). Map per the xterm key table (e.g., Up = `ESC [ A`, Shift-Up = `ESC [ 1;2 A`, F5 = `ESC [ 15~`). Option key sends ESC + char by default (standard macOS terminal behavior). Ctrl chords are byte-value mappings (Ctrl-A = `0x01`, ... Ctrl-Z = `0x1A`, etc.) — same code path. Targeted ~80 unit tests against the xterm key table.
- **D-53:** **`Cmd-V` paste in Phase 3 (bracketed paste: ESC [ 200~ ... ESC [ 201~). `Cmd-C` copy and selection-string semantics deferred** to the Phase 5 (Polish) selection work. Phase 3 implements selection *rendering* (a translucent overlay rectangle composited over the live grid per success criterion #5) but does NOT implement string extraction from the selected region.
- **D-54:** **Mouse: scroll-wheel → scrollback viewport offset; left-click-drag → selection rectangle.** No mouse-reporting-to-PTY (DEC modes 1006/1015/1016) in Phase 3 — tmux/htop mouse-mode is Phase 5. Selection is grid-cell-range state + a translucent overlay composited each frame.
- **D-55:** **Phase 3/Phase 4 boundary: Phase 3 owns single-window, single-PTY input + rendering. Phase 4 owns Cmd-T/Cmd-W tabs, Cmd-D splits, and focus routing.** The Phase 1 menu bar (D-15) has tab/split menu items already wired but disabled; Phase 4 enables them. `Cmd-Q` (quit, already wired in D-15) and `Cmd-W` (close window) are the only window-lifecycle shortcuts active in Phase 3.

### Claude's Discretion

These are open for the planner / researcher / executor to resolve without further user input:

- **Renderer crate boundary** — where the compositor (Grid → draw calls) lives: `vector-render` exclusively, or split across `vector-render` (pipeline + atlas) + `vector-app` (Grid → primitive translation). Researcher picks based on what makes the wgpu code cleaner.
- **Default theme colors** — pick a sensible xterm-256-compatible default palette (Solarized Dark, Tomorrow Night, or a custom Vector palette). Surface in `vector-theme` with the trait shape but ship one default; user override via TOML in v2 if needed.
- **Cursor visuals** — block style is conventional for terminals; blink rate matches macOS default if simple, otherwise pick a fixed rate (e.g., 530 ms half-period) and move on.
- **Glyph atlas dimensions and slot allocation** — researcher's call, must fit in macOS Metal texture limits and respect bounded LRU.
- **Renderer panic policy** — log + clear render to a sentinel "renderer error" frame rather than terminating the app; restart the wgpu surface on next user input. Existing `tracing` infra (D-32) is the diagnostic channel.
- **Selection rectangle visual** — translucent rectangle composited over the live grid; exact alpha and color planner's call (must be visible against both dark and light theme backgrounds).
- **PTY-burst coalescing threshold values** — exact debounce window (5–10 ms) and buffer-size trigger; tune empirically against `cat large.log`.

### Folded Todos

None this phase. The one pending todo (Phase-5 code-quality hardening) was reviewed and is correctly scoped to Phase 5, not Phase 3.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents (researcher, planner, executor) MUST read these before research or implementation.**

### Phase 1 carryover (still binding)
- `.planning/phases/01-foundation-ci-dmg-pipeline/01-CONTEXT.md` — D-01..D-33; especially:
  - D-09 (tokio I/O thread + winit main thread split)
  - D-11 (`clippy::await_holding_lock = "deny"`)
  - D-12 (Phase 1's text overlay placeholder that Phase 3 replaces)
  - D-14 (window 1024×640 centered, title `Vector`)
  - D-15 (standard menu bar — Phase 3 may not extend it)

### Phase 2 carryover (still binding)
- `.planning/phases/02-headless-terminal-core/02-CONTEXT.md` — D-36..D-39; especially:
  - D-36 (vector-headless stays a separate pass-through, NOT the GPU render path)
  - D-38 (PtyTransport / Domain traits final shape; Phase 3 wires `LocalDomain::spawn` → `Box<dyn PtyTransport>` and `Term.grid()` into the GPU pipeline)
  - D-39 (no Cmd-F overlay this phase — that's Phase 5)
- `.planning/phases/02-headless-terminal-core/02-RESEARCH.md` — alacritty_terminal 0.26 API drift findings (Color::Spec(Rgb), Dimensions location, scrolling_history default)
- `.planning/phases/02-headless-terminal-core/02-02-SUMMARY.md` — vector-term public surface lock (`Term::new/feed/resize/grid/cursor/mode/dims/search`); Phase 3 consumes this without changing it
- `.planning/phases/02-headless-terminal-core/02-04-SUMMARY.md` — PtyTransport / Domain trait final shape and `LocalDomain` wiring

### Project-level
- `.planning/PROJECT.md` — Core value, v1 scope discipline, macOS 13 floor
- `.planning/REQUIREMENTS.md` §RENDER-01..05 + §WIN-01 — acceptance criteria for this phase
- `.planning/ROADMAP.md` Phase 3 section — goal + 5 success criteria + risks & notes
- `./CLAUDE.md` — Tech Stack tables (wgpu 29, winit 0.30, objc2-app-kit 0.3, crossfont 0.9, JetBrains Mono context), Stack Patterns by Variant ("Splits hand-rolled", "PTY I/O not real async")

### Architecture & Patterns
- `.planning/research/STACK.md` — wgpu/winit/crossfont/objc2 recommendations + version pins
- `.planning/research/PITFALLS.md` — especially:
  - Pitfall 2 (glyph atlas churn — two atlases, bounded LRU, unicode-width)
  - Pitfall 3 (frame pacing on macOS — Fifo vsync, render-on-dirty, ProMotion, LPM)
  - Pitfall 4 (any IME / dead-key related notes for D-52 Option-key handling)
- `.planning/research/ARCHITECTURE.md` — crate boundaries (vector-render owns wgpu pipeline; vector-fonts owns rasterization; vector-app owns window + event loop + glue)
- `.planning/research/FEATURES.md` — v1 feature inventory framing

### External references (not stored locally, planner/researcher may fetch)
- xterm key table (PC-Style / VT220 / xterm extensions) — canonical source for D-52 keyboard mapping. Multiple online versions; alacritty's `alacritty/src/input/keyboard.rs` is a trustworthy Rust transcription to cross-check against.
- `wgpu::PresentMode` docs.rs page — confirm Fifo semantics on Metal backend
- Apple NSPasteboard / objc2-app-kit `NSPasteboard` bindings — for D-53 paste
- macOS `NSProcessInfo` docs (lowPowerModeEnabled + processInfoPowerStateDidChange) — for D-46

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`crates/vector-app/src/app.rs`** — winit `ApplicationHandler` impl from Phase 1; already handles `resumed`, `window_event`, `user_event`. Phase 3 extends this with input event mapping and a wgpu surface.
- **`crates/vector-app/src/tick.rs`** — Phase 1's threading smoke test (`UserEvent::Tick(n)` every 500ms). Pattern reusable for the render-tick / PTY-drain timer.
- **`crates/vector-app/src/menu.rs`** — Phase 1's menu bar (Cmd-Q quit, Cmd-W close, Cmd-M minimize). D-55 says: leave Cmd-T / Cmd-D items in place but disabled until Phase 4 enables them.
- **`crates/vector-app/src/overlay.rs`** — Phase 1 NSTextField overlay; D-51 says: leave in place for one frame at startup, replace with GPU surface once shell prompt arrives.
- **`crates/vector-term::Term`** (Phase 2) — `feed(&[u8])`, `grid()`, `damage()` (need to confirm exact name in 02-RESEARCH), `cursor()`, `mode()`, `resize()`. Phase 3 wires these to the renderer.
- **`crates/vector-mux::LocalDomain`** (Phase 2) — `spawn(SpawnCommand) -> Box<dyn PtyTransport>`. Phase 3 calls this on app start.
- **`crates/vector-headless`** — Phase 2's 30Hz ANSI render is a USEFUL reference but NOT shared code. Two separate render paths (D-36).

### Established Patterns
- **Threading split (D-09):** main thread = winit + AppKit + wgpu surface ops; tokio I/O thread = PTY reads + future SSH/tunnel work. Cross-thread signaling via `EventLoopProxy::send_event` (proven in Phase 1 tick test).
- **Per-crate arch-lint (D-08):** every crate carries `tests/no_tokio_main.rs`. Phase 3 maintains the 15==15 invariant.
- **Actor over mutex (Phase 2 D-11 enforcement):** `Box<dyn PtyTransport>` is owned by a single actor task; PTY writes go through an mpsc channel. Phase 3's render path reads `Term.grid()` under a brief `parking_lot::Mutex` lock — must NOT cross any `.await` (already lint-enforced).
- **Bundled assets via `cargo-bundle`:** Phase 1 ships `icon.icns` in `Vector.app/Contents/Resources/`. Same mechanism for `JetBrainsMono-Regular.ttf` (D-41).
- **TOML config (workspace research):** D-41/D-42 ligature + font overrides land in `~/.config/vector/config.toml`. Schema may not exist yet — Phase 3 may stub the config crate to just hold these two keys, or planner may defer all config wiring to a later phase as long as defaults work.

### Integration Points
- **wgpu surface lifecycle** — created in `app.rs::resumed` after the NSWindow exists; tied to the same raw-window-handle winit already exposes.
- **Term construction** — happens on the I/O thread when LocalDomain spawns; the grid is read on main under the actor mutex. The exact ownership model is open for the planner.
- **Atlas memory** — lives in `vector-fonts` (rasterization) and `vector-render` (wgpu textures). Two atlases, two crates of code — planner decides exact layering.
- **Menu bar (D-15)** — Phase 1 already wired all menu items. Phase 3 only adds key handlers — no menu changes required.

</code_context>

<specifics>
## Specific Ideas

- **vim is the canonical smoke test.** Success criterion #1 ("running `vim` inside renders correctly with a visible cursor") and #2 ("`cat large.log` 60+ fps on Apple Silicon") together are the acceptance bar.
- **First impression must be quiet.** No empty-grid flash, no cursor-at-(0,0) flicker, no Phase 1 placeholder lingering past first frame (D-51).
- **Selection rectangle must be visible against both dark and light terminal backgrounds.** Implementation detail (alpha, color, blend mode) is Claude's discretion, but the user expectation is "obviously selected, never invisible."
- **The `tracing` infrastructure from Phase 1 (D-32) is the diagnostic channel for everything** — frame timing, LPM throttle activation, atlas evictions, DPR-change rebuilds. Help debug remote issues later.

</specifics>

<deferred>
## Deferred Ideas

### Phase 4 (mux)
- Tabs (Cmd-T, Cmd-W, Cmd-Shift-[/], NSWindowTabbingMode plumbing)
- Splits (Cmd-D horizontal, Cmd-Shift-D vertical)
- Focus routing — which Term receives a given key when multiple panes exist
- Enabling the Phase-1-stubbed menu items for tabs/splits

### Phase 5 (Polish)
- Cmd-F search overlay (per D-39)
- Cmd-C copy + selection-to-string semantics (D-53)
- Mouse-reporting modes (DEC 1006/1015/1016 → PTY) for tmux/htop mouse-mode (D-54)
- Per-domain (local vs remote) ligature toggle (D-42)
- swash or vendored harfbuzz when ligatures move beyond "off by default" (D-42)

### Backlog
- 999.1 AI autocomplete + history-aware Claude suggestions — orthogonal; needs render in Phase 3 + mux in Phase 4 to exist before suggestions can be composed over cells

### Reviewed Todos (not folded)
- **Code-quality hardening — workspace lints, arch-lint upgrade, pre-commit cargo-deny** (2026-05-11) — out of scope for Phase 3; correctly targeted at Phase 5 (Polish) in the todo's frontmatter. Will surface again when Phase 5 is discussed.

</deferred>

---

*Phase: 03-gpu-renderer-first-paint*
*Context gathered: 2026-05-11*
