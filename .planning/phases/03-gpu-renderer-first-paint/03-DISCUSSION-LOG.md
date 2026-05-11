# Phase 3: GPU Renderer & First Paint — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-11
**Phase:** 03-gpu-renderer-first-paint
**Areas discussed:** Font + atlas stack, Frame pacing + dirty detection, Display change / DPR robustness, Input handling scope

---

## Font + atlas stack

### Q1: Which font + atlas stack should we commit to for the renderer?

| Option | Description | Selected |
|--------|-------------|----------|
| crossfont + hand-rolled two-atlas | Alacritty's CoreText wrapper + mono SDF + RGBA emoji textures with bounded LRU. Terminal-tuned, matches Pitfall 2. | ✓ |
| cosmic-text + glyphon | Single API for shaping + atlas; ligatures + BiDi free. Heavier, less terminal-tuned. | |
| Vendor a font stack (WezTerm-style) | Vendor harfbuzz + freetype. Max control, massive scope creep. | |

**User's choice:** crossfont + hand-rolled two-atlas (recommended)
**Notes:** Captured as D-40 in CONTEXT.md.

### Q2: Bundled font for tests (snapshot determinism) and default user font

| Option | Description | Selected |
|--------|-------------|----------|
| JetBrains Mono bundled + default | One artifact, OFL license, ligatures available behind toggle. | ✓ |
| Bundled = JetBrains Mono, default = SF Mono | Bundled for tests; SF Mono for native feel. Two-font world. | |
| Bundled = JetBrains Mono, default = Menlo | Zero licensing risk; older font, no ligatures. | |

**User's choice:** JetBrains Mono bundled + default (recommended)
**Notes:** Captured as D-41 in CONTEXT.md. Bundling is mandatory for snapshot-test determinism.

### Q3: Ligature handling default for v1?

| Option | Description | Selected |
|--------|-------------|----------|
| Off by default, opt-in via config | Matches Pitfall 2 guidance; crossfont without GSUB; add swash later if user toggles on. | ✓ |
| On for local, off for remote | Per-domain toggle; needs swash NOW; per-domain branching. | |
| On globally | Single shaping path; contradicts PITFALLS; flicker risk in remote sessions. | |

**User's choice:** Off by default, opt-in via config (recommended)
**Notes:** Captured as D-42 in CONTEXT.md.

### Q4: Atlas eviction strategy?

| Option | Description | Selected |
|--------|-------------|----------|
| Bounded LRU with size cap per atlas | Each atlas (mono + emoji) capped; evict LRU on overflow. Matches Pitfall 2. | ✓ |
| Unbounded but compacted periodically | Never evicts hot glyphs; unpredictable VRAM; contradicts Pitfall 2. | |
| Hash-fingerprint cells, no atlas | Per-frame rasterize. Listed as anti-pattern in Pitfall 2. | |

**User's choice:** Bounded LRU with size cap per atlas (recommended)
**Notes:** Captured as D-43 in CONTEXT.md.

---

## Frame pacing + dirty detection

### Q1: How should we detect 'dirty' (when to request a redraw)?

| Option | Description | Selected |
|--------|-------------|----------|
| alacritty Term::damage() API + dirty rows bitmap | Per-row granularity; zero cost on our side; depends on alacritty damage contract. | ✓ |
| Per-cell content hash | Perfect granularity but O(cells) every tick. | |
| Always render on Term::feed() completion | Simplest; cascades to a redraw per PTY byte. | |

**User's choice:** alacritty Term::damage() API + dirty-rows bitmap (recommended)
**Notes:** Captured as D-44 in CONTEXT.md.

### Q2: ProMotion (120Hz) handling?

| Option | Description | Selected |
|--------|-------------|----------|
| Honor whatever vsync gives us | Fifo automatically syncs; ProMotion = 120 fps on dirty frames. | ✓ |
| Cap to 60fps always | Deterministic; contradicts success criterion #2. | |
| Adaptive 120Hz scrolling / 60Hz idle | Illusory optimization (Fifo + render-on-dirty already costs 0 idle). | |

**User's choice:** Honor whatever vsync gives us (recommended)
**Notes:** Captured as D-45 in CONTEXT.md.

### Q3: macOS Low Power Mode handling?

| Option | Description | Selected |
|--------|-------------|----------|
| Cap to 30fps in LPM, log via tracing | Honest with user about trade-off; doesn't hide invisible logs. | ✓ |
| No throttling | Visible battery cost users will file bugs about. | |
| Suspend rendering entirely (iTerm2 behavior) | Minimal battery; logs scroll invisibly. | |

**User's choice:** Cap to 30fps in LPM, log the cap (recommended)
**Notes:** Captured as D-46 in CONTEXT.md.

### Q4: PTY-burst coalescing strategy?

| Option | Description | Selected |
|--------|-------------|----------|
| Tokio task batches reads, sends as `Vec<u8>` chunks per frame budget | One feed-and-render per vsync, not thousands. | ✓ |
| Send every read() result as a separate event | Thousands of cross-thread sends under `cat`. | |
| Debounce until idle | Visible latency on slow-output programs. | |

**User's choice:** Tokio task batches reads (recommended)
**Notes:** Captured as D-47 in CONTEXT.md.

---

## Display change / DPR robustness

### Q1: Atlas behavior on DPR change?

| Option | Description | Selected |
|--------|-------------|----------|
| Clear atlas + lazy re-rasterize per glyph on miss | Simple, correct; one-frame stutter acceptable per success criterion #4. | ✓ |
| Pre-rasterize both DPRs upfront | Zero swap cost; 2× memory; doesn't generalize to fractional DPRs. | |
| Keep atlas + scale-blit | Zero churn; blurry glyphs at non-source DPR. | |

**User's choice:** Clear + lazy re-rasterize (recommended)
**Notes:** Captured as D-48 in CONTEXT.md.

### Q2: Live window resize handling?

| Option | Description | Selected |
|--------|-------------|----------|
| Resize Term grid on debounce + repaint per frame | Smooth visual; Term reflow only on quiescent / resize-end. | ✓ |
| Resize Term grid synchronously on every Resized | Always current; spikes CPU during fast drag. | |
| Pause rendering during drag | Empty/garbage window during drag. | |

**User's choice:** Debounced live resize (recommended)
**Notes:** Captured as D-49 in CONTEXT.md.

### Q3: Glyph AA mode?

| Option | Description | Selected |
|--------|-------------|----------|
| Grayscale AA via CoreText, always | One pixel format, one shader path; mirrors Alacritty. | ✓ |
| Subpixel non-Retina + grayscale Retina | Two atlas formats; complicates DPR-swap. | |
| Always subpixel AA | Blurry on Retina; fights the OS. | |

**User's choice:** Grayscale AA always (recommended)
**Notes:** Captured as D-50 in CONTEXT.md. Apple disabled subpixel AA system-wide in Mojave.

### Q4: First-frame timing?

| Option | Description | Selected |
|--------|-------------|----------|
| After Term spawn + first PTY read + font loaded | No empty-grid flash; quiet first impression. | ✓ |
| Paint empty grid immediately | Instant 'window is live'; visible empty-terminal flash. | |
| Block window-visible until first PTY byte | Clean reveal; window appears to hang on launch. | |

**User's choice:** After Term spawn + first PTY read + font loaded (recommended)
**Notes:** Captured as D-51 in CONTEXT.md.

---

## Input handling scope

### Q1: Minimum keyboard set?

| Option | Description | Selected |
|--------|-------------|----------|
| Full xterm-compatible: printable + Esc + arrows + F1-F12 + nav + modifiers | Covers vim, tmux, htop, less, fzf without further work. | ✓ |
| Bare minimum: printable + Esc + arrows + Enter + Backspace + Tab | vim users hit walls fast (no F-keys, no PgDn). | |
| All xterm + readline chord support | Largely a label — Ctrl chords already covered by recommended. | |

**User's choice:** Full xterm-compatible set (recommended)
**Notes:** Captured as D-52 in CONTEXT.md. Option-key sends ESC + char by default.

### Q2: Clipboard integration in Phase 3?

| Option | Description | Selected |
|--------|-------------|----------|
| Cmd-V paste in Phase 3, Cmd-C deferred | Paste is trivial + critical; copy depends on selection-to-string. | ✓ |
| Both Cmd-V and Cmd-C in Phase 3 | Complete clipboard story; clipboard format decisions creep scope. | |
| Defer both to Phase 5 | Smallest scope; misses 'daily-driver' spirit. | |

**User's choice:** Cmd-V paste only, Cmd-C deferred (recommended)
**Notes:** Captured as D-53 in CONTEXT.md. Bracketed paste (ESC [ 200~ ... ESC [ 201~).

### Q3: Mouse handling?

| Option | Description | Selected |
|--------|-------------|----------|
| Scroll-wheel scrollback + click-drag selection | Both in success criteria; no PTY mouse-reporting yet. | ✓ |
| Selection only | Smaller scope; awkward scrollback navigation. | |
| Selection + scroll-wheel + DEC mouse reporting | Complete; mouse-reporting logic is non-trivial. | |

**User's choice:** Scroll-wheel + click-drag selection (recommended)
**Notes:** Captured as D-54 in CONTEXT.md. DEC mouse-reporting deferred to Phase 5.

### Q4: Phase 3/Phase 4 boundary?

| Option | Description | Selected |
|--------|-------------|----------|
| Phase 3 = single window/single PTY; Phase 4 = tabs+splits+routing | Matches ROADMAP literally; clean phase boundary. | ✓ |
| Phase 3 also implements Cmd-T (new tab) | Smudges boundary; mux should land in one piece. | |
| Phase 3 = renderer only; input in 3.5 | Splits Phase 3 unnecessarily. | |

**User's choice:** Single-window single-PTY only in Phase 3 (recommended)
**Notes:** Captured as D-55 in CONTEXT.md. Cmd-Q (Phase 1 D-15) and Cmd-W are the only active window-lifecycle shortcuts in Phase 3.

---

## Claude's Discretion

Items deferred to planner/researcher/executor without further user input:
- Renderer crate boundary (compositor in `vector-render` exclusively, or split with `vector-app`)
- Default theme colors (xterm-256-compatible default palette)
- Cursor visuals (block + blink rate)
- Glyph atlas dimensions and slot allocation
- Renderer panic policy (log + sentinel frame, not app termination)
- Selection rectangle visual (alpha + color)
- PTY-burst coalescing threshold values (5–10 ms debounce; buffer-size trigger)

## Deferred Ideas

- Tabs / splits / focus routing → Phase 4
- Cmd-F search overlay → Phase 5 (already in Phase 2 D-39)
- Cmd-C copy + selection-string semantics → Phase 5
- DEC mouse-reporting → Phase 5
- Per-domain ligature switching → Phase 5+
- swash / harfbuzz when ligatures move past "off by default" → Phase 5+

## Reviewed Todos (not folded)

- Code-quality hardening — workspace lints, arch-lint upgrade, pre-commit cargo-deny (2026-05-11) — correctly targets Phase 5, not Phase 3.
