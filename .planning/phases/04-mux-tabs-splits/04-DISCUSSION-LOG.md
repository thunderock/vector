# Phase 4: Mux — Tabs & Splits — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-11
**Phase:** 04-mux-tabs-splits
**Areas discussed:** Tab bar style, Focus + split keymap + Cmd-W semantics, Split: cwd inheritance, Cmd-N (new window) + active-pane indicator

---

## Tab bar style

| Option | Description | Selected |
|--------|-------------|----------|
| Native NSWindowTabbingMode | AppKit-native: one NSWindow per tab, AppKit groups them. Matches Apple Terminal + ghostty. ~80% less code. CLAUDE.md recommends. | ✓ |
| Custom wgpu-drawn tab bar | WezTerm-style: one NSWindow, draw bar in wgpu. Full theme control. ~1 week of UI work. | |
| Native now, custom later | Ship native in Phase 4; revisit in Phase 7 if CS-06 needs more. | |

**User's choice:** Native NSWindowTabbingMode

### Tab title source

| Option | Description | Selected |
|--------|-------------|----------|
| Foreground process name | Track via PTY pgrp + proc_pidpath. Updates dynamically (zsh → vim → zsh). Matches Apple Terminal. | ✓ |
| Static 'Vector' | Every tab labeled 'Vector'. Zero code. Annoying for daily use. | |
| Domain.label() | 'Local' / 'codespace:my-repo'. Stable but less informative than process name. | |

**User's choice:** Foreground process name

### CS-06 remote-tab tint planning

| Option | Description | Selected |
|--------|-------------|----------|
| Unicode prefix + revisit | Phase 7 prefixes title with emoji/symbol (☁ codespace-name). Pure text. Zero Phase 4 cost. | ✓ |
| Pre-build hook for NSWindow accessoryView | Phase 4 lands a per-Tab field for Cocoa accessory view; Phase 7 fills with tinted badge. AppKit work now. | |
| Defer entirely | Phase 7 figures it out from scratch. Cleanest scope. | |

**User's choice:** Unicode prefix + revisit if insufficient

**Notes:** All three settled together; user moved on to next area without follow-up questions.

---

## Focus + split keymap + Cmd-W semantics

### Pane focus

| Option | Description | Selected |
|--------|-------------|----------|
| Cmd-Opt-Arrow directional | Spatial directional focus. Matches ghostty + iTerm2. | ✓ |
| Cmd-[/] cycle | Linear cycling, tree-traversal order. Apple Terminal-style. Loses spatial intuition. | |
| Both | Spatial + cycle. Doubles keymap, risks Cmd-[ conflict with vim. | |
| Cmd-h/j/k/l vim-style | Conflicts with Cmd-H ('Hide Vector'). | |

**User's choice:** Cmd-Opt-Arrow

### Pane resize

| Option | Description | Selected |
|--------|-------------|----------|
| Mouse drag + Cmd-Shift-Arrow | Visual drag for big moves, keyboard nudge for fine control. ~50 lines. | ✓ |
| Mouse drag only | No keyboard. Simpler. | |
| Cmd-Shift-Arrow only | Keyboard-only. Feels wrong on macOS. | |

**User's choice:** Both mouse drag + Cmd-Shift-Arrow

### Cmd-W semantics

| Option | Description | Selected |
|--------|-------------|----------|
| Close pane → fallback tab → fallback window | Ghostty cascade. Natural mental model. | ✓ |
| Close tab always | Apple Terminal-style. Loses split granularity. | |
| Cmd-W pane / Cmd-Shift-W tab | Explicit two-shortcut. iTerm2-style. | |

**User's choice:** Close pane with cascade fallback

### Tab cycling confirmation

User confirmed `Cmd-Shift-]/[` per ROADMAP — no Cmd-1..9 jump-to-tab in v1.

---

## Split: cwd inheritance

### Cmd-D split cwd source

| Option | Description | Selected |
|--------|-------------|----------|
| Inherit via proc_pidinfo | macOS libproc lookup of active pane's shell PID cwd. Matches tmux. Swap to OSC 7 in Phase 5. | ✓ |
| Always $HOME / shell default | Login-shell starts in $HOME. Zero code. Loses 'split here' workflow. | |
| Defer to Phase 5 (OSC 7) | Phase 4 ships login-shell cwd; Phase 5 retrofits. Worse v1 daily-driver experience. | |

**User's choice:** Inherit cwd via proc_pidinfo

### Cmd-T new tab cwd source

| Option | Description | Selected |
|--------|-------------|----------|
| Inherit from active pane | Consistency with Cmd-D split. | ✓ |
| Always $HOME | Differentiate tab vs split context. Apple Terminal default. | |
| Config-driven | Ship inherit; add TOML key in Phase 5. | |

**User's choice:** Inherit from active pane

**Notes:** Symbolic-link resolution, proc_pidinfo failure fallback, and SIP-protected dirs all left to Claude's discretion (recorded as D-64).

---

## Cmd-N (new window) + active-pane indicator

### Multi-window scope

| Option | Description | Selected |
|--------|-------------|----------|
| Defer to Phase 5 | Mux supports it; menu disabled. Smaller Phase 4. | ✓ |
| Enable in Phase 4 | Wire Cmd-N to spawn NSWindow with new Mux::Window. Modest extra plumbing. | |
| Enable as separate Mux instance | Independent tab groups per Cmd-N. More AppKit-y. | |

**User's choice:** Defer to Phase 5

### Active-pane indicator style

| Option | Description | Selected |
|--------|-------------|----------|
| Thin colored border | 1–2 px accent-color border. Reuse Phase 3 tint uniform with border mask. ghostty / iTerm2 default. | ✓ |
| Dim inactive panes | 50% opacity overlay on inactive. Strong cue. Extra render pass per pane. | |
| Cursor-only + subtle border | Inactive cursor = hollow outline; active = filled + thin border. Most subtle. | |
| Border + dim (both) | Strongest signal. Possibly overkill. | |

**User's choice:** Thin colored border

---

## Claude's Discretion

Decisions delegated to downstream agents (recorded in CONTEXT.md `<decisions>` block, "Claude's Discretion" subsection):

- `vector-ui` crate decision — land now or defer to Phase 6 (Codespaces picker)
- Tab close animation / drag-to-reorder — whatever NSWindowTabbingMode gives natively
- Maximum splits per tab — no hard limit; enforce minimum-pane-size during resize
- Pane minimum size — sensible floor (e.g., 20×4 cells)
- Per-pane process-exit policy — sentinel line + Cmd-W or restart
- Cursor visibility in inactive panes — hollow vs filled
- PaneId allocator — `AtomicU64` counter

---

## Deferred Ideas

- **Phase 5:** Cmd-N (new window), OSC 7 cwd tracking, Cmd-F search overlay, Cmd-C copy, mouse-reporting modes, per-pane ligature toggle
- **Phase 7:** Remote-tab tint/badge (CS-06)
- **Out of scope (Pitfall 21 / scope guard):** Layout save/restore, broadcast-input, leader-key chord modes, maximize-current-pane zoom, custom in-window tab bar
- **Backlog:** 999.1 AI autocomplete (orthogonal, needs Mux first)
- **Reviewed not folded:** `code-quality-hardening` todo (correctly target_phase: 5)
