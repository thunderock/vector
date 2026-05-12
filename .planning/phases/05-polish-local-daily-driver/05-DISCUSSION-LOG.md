# Phase 5: Polish (Local Daily-Driver) - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-12
**Phase:** 05-polish-local-daily-driver
**Areas discussed:** Config + hot-reload, Clipboard + tmux passthrough, Theme strategy + .itermcolors, Profile model + switcher, Scrollback search UX, OSC 7/8/133 behaviors, Secure Keyboard Entry + IME, Cmd-N + code-quality hardening

---

## Area 1: Config + hot-reload model (POLISH-01)

### Q: How should the config file be structured?

| Option | Description | Selected |
|--------|-------------|----------|
| Single config.toml, [default] + [profile.X] | One file, inherit-by-overlay, simpler hot-reload | ✓ |
| config.toml + profiles/*.toml | Top-level + per-profile dir; ambiguous precedence | |
| Single config.toml, deep-merge cascade | Recursive deep-merge over nested tables; harder to reason about | |

**User's choice:** Single config.toml, [default] + [profile.X]
**Notes:** Recorded as D-68.

### Q: What should hot-reload do, and what about invalid configs?

| Option | Description | Selected |
|--------|-------------|----------|
| Live-apply theme/keybinds/font-size; restart for font-family + GPU; keep-last-good on invalid | Apple-Terminal-esque | ✓ |
| Live-apply everything possible, restart on invalid | Power-user friendly | |
| All changes require Cmd-Shift-R explicit reload | Predictable but breaks edit-save loop | |

**User's choice:** Live-apply most; restart for font-family + GPU; keep-last-good on invalid + toast
**Notes:** Recorded as D-69.

---

## Area 2: Clipboard + tmux passthrough policy (POLISH-05)

### Q: Default state for OSC 52 clipboard writes?

| Option | Description | Selected |
|--------|-------------|----------|
| Prompt-once per origin, then remember | macOS-style permission UX, per-profile | ✓ |
| Always-on (no prompt) | Best UX, worst security (CVE-class) | |
| Off by default; enable per-profile | Most conservative, friction for Codespaces | |
| Always-on for write, never for read | iTerm2's default | |

**User's choice:** Prompt-once per origin, then remember
**Notes:** Recorded as D-70. Reads are always denied in v1 (combined the "never read" guarantee with the prompt-on-write approach).

### Q: How should we handle the tmux DCS-wrapping pitfall?

| Option | Description | Selected |
|--------|-------------|----------|
| Accept both raw + DCS inbound, never re-wrap outbound; chunk at 58 bytes; document allow-passthrough | Pitfall 8 prescription | ✓ |
| Auto-detect $TMUX and wrap outbound writes | Aggressive, ambiguous with nested tmux | |
| Raw-only; document "don't use OSC 52 inside tmux without passthrough" | Breaks common workflow | |

**User's choice:** Accept both inbound, never re-wrap outbound, document allow-passthrough
**Notes:** Recorded as D-71. 58-byte chunking dodges the ~60-char tmux passthrough bug (tmux issue #4377).

---

## Area 3: Theme strategy + .itermcolors UX (POLISH-02 / POLISH-03)

### Q: Built-in themes + macOS appearance follow?

| Option | Description | Selected |
|--------|-------------|----------|
| Vector Light + Vector Dark only; appearance="system" default | Curated single identity, users bring rest via .itermcolors | ✓ |
| Vector + 3 classics (Solarized Dark, Tomorrow Night, Gruvbox Dark) | Wider appeal, more to maintain | |
| Vector only (single dark theme); no macOS follow | Smallest surface, doesn't honor light-mode users | |

**User's choice:** Vector Light + Vector Dark + auto-follow opt-in via appearance = "system"
**Notes:** Recorded as D-72.

### Q: How should users import .itermcolors palettes?

| Option | Description | Selected |
|--------|-------------|----------|
| Drop file in ~/.config/vector/themes/; reference by stem | Zero UI surface, fits file-first config story | ✓ |
| Drag-onto-app + import dialog | Friendlier but needs AppKit Open handler | |
| CLI: vector theme import <file> | Scriptable, no GUI | |

**User's choice:** Drop file in themes dir, reference by stem
**Notes:** Recorded as D-73. CLI + drag-onto-app explicitly deferred.

---

## Area 4: Profile model + switcher UX (POLISH-07)

### Q: Profile model: fixed kinds or user-extensible?

| Option | Description | Selected |
|--------|-------------|----------|
| Fixed kind {local, codespace, dev_tunnel}, unlimited named profiles per kind | Type-system honest, future-proof | ✓ |
| Free-form string kind | Flexible, loses exhaustiveness | |
| Three fixed profiles by name; no extensibility | Hard-codes, won't survive multiple codespaces | |

**User's choice:** Fixed kind enum + unlimited named profiles
**Notes:** Recorded as D-74. Phase 5 only wires kind=local; codespace/dev_tunnel transports land in Phase 6/7.

### Q: What does the "tint" look like, and how do users switch profiles?

| Option | Description | Selected |
|--------|-------------|----------|
| Title-bar tint stripe + Cmd-Shift-P palette switcher | Instant visual ID + fast keyboard switch | ✓ |
| Full per-profile theme override + menu-only switcher | Powerful but heavyweight | |
| Subtle window edge accent + dedicated keyboard shortcut per profile | Collides with future Cmd-1 tab-jump | |

**User's choice:** Title-bar tint stripe + Cmd-Shift-P picker

### Follow-up Q: Cmd-Shift-P conflicts with ROADMAP's "no command palette" note — how to reconcile?

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal profile switcher only, NOT a general palette | Narrow scope exception, honors roadmap's spirit | ✓ |
| Drop the palette; menu-only switcher | Honors roadmap literally | |
| Treat as roadmap-update; full command palette in scope | Significant new surface | |

**User's choice:** Narrow profile switcher only — explicit scope exception
**Notes:** Recorded as D-75. The roadmap line "no command palette" is preserved for Lua/plugins/general action picker; the D-75 picker is a fuzzy-name profile switcher only.

---

## Area 5: Scrollback search UX (POLISH-06)

### Q: Where does the search bar live and how does it activate?

| Option | Description | Selected |
|--------|-------------|----------|
| Inline bottom bar, Cmd-F open / Esc close | Familiar (Safari/VS Code), per-pane | ✓ |
| Top bar overlaid on title | Saves bottom space, fights NSWindow tabs | |
| Floating centered modal | Steals focus, heavy visual weight | |

**User's choice:** Inline bottom bar, Cmd-F / Esc
**Notes:** Recorded as D-76.

### Q: Search behavior: regex toggle, case, Enter semantics?

| Option | Description | Selected |
|--------|-------------|----------|
| Smart-case + regex always-on + Enter next / Shift-Enter prev | vim/rg standard, reuses D-39 Regex API | ✓ |
| Literal default + [.*] regex toggle + case toggle | More discoverable, more clutter | |
| Pure literal substring | Throws away D-39 Regex capability | |

**User's choice:** Smart-case + always-regex + Enter/Shift-Enter
**Notes:** Recorded as D-77.

---

## Area 6: OSC 7 / OSC 8 / OSC 133 visible behaviors (POLISH-04)

### Q: OSC 8 hyperlink UX and sanitization?

| Option | Description | Selected |
|--------|-------------|----------|
| Hover-underline + Cmd-click; allow https/http/mailto/file:// | Default-clean visuals, discoverable on hover | ✓ |
| Always-underlined + Cmd-click | Web-like, busier visuals | |
| Cmd-click only, no visual indicator | Plain-terminal feel, hardest discovery | |

**User's choice:** Hover-underline + Cmd-click, sanitize schemes
**Notes:** Recorded as D-78. Anything outside the allowlist logged + ignored (CVE-class per Pitfalls security row).

### Q: OSC 7 cwd handoff + OSC 133 prompt marks?

| Option | Description | Selected |
|--------|-------------|----------|
| OSC 7 replaces proc_pidinfo (when present); OSC 133 silent collection + nav-hook stub | Captures data now, defers UI | ✓ |
| OSC 7 + visible gutter chevrons for OSC 133 | Adds render pass, useful for long sessions | |
| OSC 7 only; defer OSC 133 entirely | Descopes from POLISH-04 | |

**User's choice:** OSC 7 preferred for cwd; OSC 133 silent collection + stub
**Notes:** Recorded as D-79. OSC 133 navigation UI deferred.

---

## Area 7: Secure Keyboard Entry + IME (POLISH-08)

### Q: Secure Keyboard Entry surface area?

| Option | Description | Selected |
|--------|-------------|----------|
| Menu item only, global app state, persisted | Apple-Terminal-esque, honest about API scope | ✓ |
| Menu + Cmd-Shift-K, per-window state | Apple's API is process-level — per-window illusory | |
| Menu + Cmd-Shift-K, global state | Easy to fat-finger | |

**User's choice:** Menu-only, global, persisted
**Notes:** Recorded as D-80.

### Q: Basic IME scope — confirm v1/v2 boundary?

| Option | Description | Selected |
|--------|-------------|----------|
| Display marked text under cursor; no candidate window; full IME = v2 | Pitfall 16's exact prescription | ✓ |
| No IME at all in v1; document in README | Stricter Pitfall 16 reading, breaks dead-keys | |
| Full IME with candidate window | Major scope creep, contradicts Pitfall 16 | |

**User's choice:** Basic preedit only, full IME = v2
**Notes:** Recorded as D-81.

---

## Area 8: Cmd-N + code-quality hardening

### Q: Cmd-N (new window) behavior?

| Option | Description | Selected |
|--------|-------------|----------|
| Spawn fresh local profile in new NSWindow | "Clean slate" semantics, predictable | ✓ |
| Duplicate focused window's profile + cwd | Conflates new-window with new-pane | |
| Cmd-N opens minimal profile picker first | Most clicks for common case | |

**User's choice:** Spawn fresh local profile
**Notes:** Recorded as D-82. Closes D-65 deferral.

### Q: Code-quality hardening scope?

| Option | Description | Selected |
|--------|-------------|----------|
| All four sub-items (lints inheritance + path-dep arch-lint + cargo-deny pre-commit + cargo-machete) | Closes the d652c8b regression class | ✓ |
| Just workspace lints + path-dep arch-lint | Catches immediate regression, leaves hygiene later | |
| Defer all to a dedicated tooling phase | Risk: todo pending since Phase 2 | |

**User's choice:** All four sub-items
**Notes:** Recorded as D-83. Folded todo `code-quality-hardening` retired into Phase 5.

---

## Claude's Discretion

- Cmd-C / selection-string extraction (carries from D-53/D-54)
- OSC 10/11/12 fg/bg/cursor color query responses (mechanical parser response)
- Keybind override TOML syntax
- Font fallback chain (CoreText system default + emoji + CJK)
- vector-config / vector-theme / vector-secrets crate boundaries
- notify debounce strategy (150ms quiescent + atomic-rename handling)
- Toast surface implementation
- Profile picker fuzzy match library
- OSC 8 hyperlink span detection during hover
- Search highlight color (theme-aware yellow/orange)
- OSC 133 PromptMark struct shape
- SKE menu item position
- Profile scope = per-pane

## Deferred Ideas

- General command palette (FEATURES.md row 68)
- OSC 133 prompt-mark navigation UI (Cmd-PageUp jump-to-prev-prompt)
- OSC 9;4 progress reporting
- Drag-and-drop + CLI .itermcolors import surfaces
- Cmd-1..9 jump-to-tab / jump-to-profile
- Sparkle auto-updater
- Full IME with candidate window
- Per-window Secure Keyboard Entry
- OSC 52 read (clipboard query from terminal)
- Backlog 999.1 AI autocomplete

## Reviewed Todos (not folded)

None.
