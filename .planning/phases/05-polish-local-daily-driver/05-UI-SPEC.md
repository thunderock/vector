---
phase: 05
phase_name: Polish (Local Daily-Driver)
status: draft
design_system: none (native AppKit + in-house wgpu compositor)
created: 2026-05-12
sources:
  - .planning/REQUIREMENTS.md (POLISH-01..08)
  - .planning/ROADMAP.md (§Phase 5)
  - .planning/phases/05-polish-local-daily-driver/05-CONTEXT.md (D-68..D-83)
  - .planning/phases/05-polish-local-daily-driver/05-RESEARCH.md (R-Phase5-01..16)
  - CLAUDE.md (Rust workspace, macOS-only, wgpu/winit/AppKit constraints)
---

# UI-SPEC — Phase 5: Polish (Local Daily-Driver)

## 0. Scope & Frame

Phase 5 adds the chrome that turns the Phase-1–4 renderer + PTY core into a daily-driver local terminal. Every visual surface in this spec is implemented with one of two technologies — there is no third:

| Surface | Technology |
|---------|------------|
| Window chrome (titlebar, traffic lights, native tab bar, menu bar, secure-input checkmark, system services) | **AppKit** via `objc2-app-kit 0.6` |
| Tint stripe, search bar, profile picker, toast banner, IME preedit, OSC-8 hover affordance, search highlights, prompt-mark capture | **wgpu compositor** (reuses Phase-3 pipeline + Phase-4 active-pane border pass) |

There is **no HTML/CSS, no SwiftUI, no third-party UI framework**. Tab bar is system-supplied via `NSWindow.tabbingMode = .preferred` (not drawn by us). Per-pane chrome (tint stripe, search bar, toast) lives **inside the wgpu render graph as additional passes** that share the Phase-3 glyph atlas and quad pipeline.

This UI-SPEC is the design contract. Every downstream plan task **must** reference the named tokens (e.g. `chrome.toast.height`, `color.search.highlight.dark`) by symbol, not by re-deriving values.

---

## 1. Visual Mockups

ASCII mockups are normative. They show pixel-level relationships at default 1.0× device-pixel ratio; multiply all dimensions by `NSScreen.backingScaleFactor` for retina.

### 1.1 Window with tint stripe + active pane + search bar

```
┌────────────────────────────────────────────────────────────────────┐
│ ● ● ●     Vector                                                   │ ← NSWindow titlebar (28 px, AppKit, untouched)
├────────────────────────────────────────────────────────────────────┤
│████████████████████████████████████████████████████████████████████│ ← tint stripe (28 px, wgpu, profile.tint colour, alpha 1.0)
├────────────────────────────────────────────────────────────────────┤
│  default ×    work-codespace ×    [+]                              │ ← NSWindow native tab bar (system-drawn, ~28 px)
├──┬─────────────────────────────────┬───────────────────────────────┤
│  │                                 │                               │
│  │   $ ls -la                      │   $ cargo build               │   (panes — Phase-3 grid)
│  │   total 24                      │                               │
│  │                                 │                               │
│  │                                 │                               │   active pane has Phase-4 1 px border (D-66)
│  │                                 │                               │
│  └─────────────────────────────────┘                               │
│  ╔═════════════════════════════════════════════════════════════╗   │ ← search bar (32 px, wgpu, inside active pane viewport)
│  ║ / cargo /▢   aA    ↑   ↓   3/142    ×                       ║   │
│  ╚═════════════════════════════════════════════════════════════╝   │
└────────────────────────────────────────────────────────────────────┘
```

**Layout invariants**
- Tint stripe sits **below** the AppKit titlebar (traffic lights stay visible at top, untouched) and **above** the system tab bar. Stripe never overlaps traffic lights.
- Default profile renders **no stripe**; the row collapses to 0 px (tab bar slides up).
- Search bar is **inside** the active pane's content rectangle, anchored to its bottom edge, **inset by the Phase-4 active-pane border** (1 px) so the border continues unbroken around it.
- Search bar is part of the active pane only; non-active panes have no search bar even if their search state exists (it's hidden, not destroyed).

### 1.2 Profile picker (Cmd-Shift-P)

```
                    ┌──────────────────────────────────────┐
                    │ ▸ filter…                            │ ← input row (32 px)
                    ├──────────────────────────────────────┤
                    │ ● default                            │ ← selected (highlight bg)
                    │ ● work-laptop                        │
                    │ ● adobe-vpn                          │
                    │ ⚠ rust-codespace        Phase 6+     │ ← dimmed, suffix label
                    │ ⚠ devtunnel-prod        Phase 6+     │
                    └──────────────────────────────────────┘
```

**Layout invariants**
- Centered horizontally in the active NSWindow content rect, vertically anchored 25 % from top.
- Width = `max(longest profile name in px, 280) + 48 px breathing room`, clamped to `[280, 480]`.
- Per-row height: 28 px. Input row: 32 px. Max visible rows: 8 (scroll on overflow). No row separators.
- Modal — dims the rest of the window with a 40 % black overlay; Esc / click-out closes without switching profile.
- Codespace / DevTunnel `kind` profiles are listed but render dimmed (`color.picker.row.disabled`) with the suffix `Phase 6+` in `chrome.font.micro`. Enter on a dimmed row is a no-op (and emits a `restart required`-style toast with the message `profile kind not available until phase 6`).

### 1.3 Toast banner — informational (auto-dismiss)

```
┌────────────────────────────────────────────────────────────────────┐
│ ● ● ●     Vector                                                   │
├────────────────────────────────────────────────────────────────────┤
│ ⓘ config error at line 12: invalid key "foo"                    ×  │ ← toast (36 px, wgpu, anchored top of content)
├────────────────────────────────────────────────────────────────────┤
│ ▒▒▒▒▒▒▒▒▒▒  tint stripe…                                           │
```

### 1.4 Toast banner — action prompt (clipboard authorization)

```
┌────────────────────────────────────────────────────────────────────┐
│ ● ● ●     Vector                                                   │
├────────────────────────────────────────────────────────────────────┤
│ ⚠ allow “work-laptop : node” to write to your clipboard?           │
│   [ allow once ]  [ always ]  [ block ]                         ×  │ ← 56 px when two-line action
├────────────────────────────────────────────────────────────────────┤
```

### 1.5 OSC-8 hyperlink hover

```
   ┌──────────────────────────────────────────┐
   │  see https://example.com/very-long-url    │ ← cursor: Cmd-arrow when modifier held
   │      ‥‥‥‥‥‥‥‥‥‥‥‥‥‥‥‥‥‥‥‥‥‥‥‥           │ ← dotted underline (2 px dash, 2 px gap), color.link.hover
   └──────────────────────────────────────────┘
```

### 1.6 IME preedit

```
   $ echo こん▁  ← preedit "こん" rendered at active cell with underline attribute,
                   cursor advanced past it; Enter commits, Esc cancels.
```

No candidate window in Phase 5 (D-81). The OS may render its own candidate strip in a floating window — that's an AppKit concern, not ours.

---

## 2. Spacing Tokens

Base grid: **4 px**. Every dimension is `4n`. No exceptions.

| Token | Value | Used For |
|-------|-------|----------|
| `spacing.0` | 0 | flush edges |
| `spacing.1` | 4 px | tight (icon ↔ label inside search bar) |
| `spacing.2` | 8 px | standard internal padding (toast left margin, picker row x-padding) |
| `spacing.3` | 12 px | between chrome groups (search-bar buttons cluster) |
| `spacing.4` | 16 px | toast button row inset |
| `spacing.6` | 24 px | picker max horizontal breathing room |
| `spacing.8` | 32 px | minimum row width safeguard |
| `spacing.12` | 48 px | picker outer breathing-room budget |

**Surface heights (snapped to the 4 px grid):**

| Token | Value | Used For |
|-------|-------|----------|
| `chrome.titlebar.height` | 28 px | AppKit-owned; we observe but do not draw |
| `chrome.tintstripe.height` | 28 px | wgpu pass; 0 px when profile.tint absent |
| `chrome.tabbar.height` | ~28 px | system-supplied; we do not own this number |
| `chrome.searchbar.height` | 32 px | wgpu, per active pane |
| `chrome.toast.height.info` | 36 px | single-line informational toast |
| `chrome.toast.height.action` | 56 px | two-line action toast (text + button row) |
| `chrome.picker.row.height` | 28 px | profile picker rows |
| `chrome.picker.input.height` | 32 px | profile picker filter input row |
| `chrome.picker.max_rows_visible` | 8 | overflow → scroll |
| `chrome.picker.width.min` | 280 px | floor |
| `chrome.picker.width.max` | 480 px | ceiling |

**Phase-5 exceptions:** none. All chrome lives on the 4 px grid.

---

## 3. Typography

**Two fonts. No more.**

| Token | Family | Where | Source |
|-------|--------|-------|--------|
| `font.grid` | JetBrains Mono (bundled) | terminal cell grid (existing from Phase 3 / D-41) | not new this phase |
| `font.chrome` | macOS system font (`NSFont.systemFont(ofSize:)`) | toast text, picker rows, search bar counter, button labels | new in Phase 5 |

**Sizes — chrome only (the grid font is not re-specified):**

| Token | Size | Weight | Line Height | Used For |
|-------|------|--------|-------------|----------|
| `font.chrome.body` | 13 pt | regular (400) | 1.4 | toast text, picker row label, search-bar query echo |
| `font.chrome.small` | 11 pt | regular (400) | 1.4 | search-bar match counter (`3/142`), toggle icons (`aA`), picker `Phase 6+` suffix |
| `font.chrome.button` | 13 pt | semibold (600) | 1.3 | toast action buttons (`allow once`, `always`, `block`) |

Three sizes, two weights total. This satisfies the "3–4 sizes, 2 weights" discipline.

**Why system font, not JetBrains Mono, for chrome:** chrome text is prose, not code; using the system font (SF Pro) keeps Vector visually aligned with macOS Terminal, Xcode, and Console — the apps Vector competes with for muscle memory.

---

## 4. Color Contract

All colors resolve through `vector-theme`. **No hardcoded hex in chrome code.** The contract below specifies which palette key each chrome surface uses; the actual hex values come from `Vector Light` / `Vector Dark` themes (R-Phase5-13) and any per-profile `.itermcolors` overlay.

### 4.1 60 / 30 / 10 split

| Role | % | Source | Examples |
|------|---|--------|----------|
| Dominant (60 %) | Terminal background (`theme.bg`) | the pane grid behind everything | always |
| Secondary (30 %) | Chrome surfaces (`theme.chrome.surface`) | toast, picker bg, search-bar bg | Phase 5 net-new |
| Accent (10 %) | **Profile tint** (`profile.tint`) **only** | tint stripe; active-search-match border tinting; picker selected-row hairline | reserved-for list below |

### 4.2 Accent is reserved for, and ONLY for

1. The tint stripe (filled rectangle, alpha 1.0).
2. The 1 px hairline border around the **active-search-match** highlight (alpha 1.0).
3. The 2 px left-edge bar on the **selected row** of the profile picker.

Accent is **never** used for: toast backgrounds, search highlight fills, toast button text, picker row text, OSC-8 hover underline. Those have their own dedicated tokens below.

### 4.3 Second semantic color

| Token | Use | Source |
|-------|-----|--------|
| `color.warning` | toast info icon background (parse error, restart-required, kind-mismatch); picker `Phase 6+` suffix tint; clipboard-prompt left-edge bar | `theme.warning` (yellow/amber family) |
| `color.danger` | reserved — Phase 5 has no destructive action in chrome (clipboard prompt's `block` button is neutral, not danger) | unused in Phase 5; declared for future |

### 4.4 Full chrome color token table

| Token | Light theme source | Dark theme source | Alpha | Surface |
|-------|-------------------|-------------------|-------|---------|
| `color.tintstripe` | `profile.tint` | `profile.tint` | 1.0 | tint stripe fill |
| `color.search.bar.bg` | `theme.chrome.surface` | `theme.chrome.surface` | 0.92 | search bar background |
| `color.search.bar.border` | `theme.chrome.divider` | `theme.chrome.divider` | 1.0 | 1 px hairline top of search bar |
| `color.search.highlight` | `theme.search.highlight.light` (orange family) | `theme.search.highlight.dark` (yellow family) | 0.40 | all non-active match cell overlays |
| `color.search.highlight.active.border` | same hue, full saturation | same hue, full saturation | 1.0 | 1 px border around the *active* match |
| `color.search.no_match.bg` | `theme.danger.subtle` | `theme.danger.subtle` | 0.20 | search bar bg when 0 matches found |
| `color.toast.info.bg` | `theme.chrome.surface` | `theme.chrome.surface` | 0.95 | informational toast |
| `color.toast.info.icon` | `theme.warning` | `theme.warning` | 1.0 | ⓘ glyph fill |
| `color.toast.action.bg` | `theme.chrome.surface` | `theme.chrome.surface` | 0.95 | action toast |
| `color.toast.action.icon` | `theme.warning` | `theme.warning` | 1.0 | ⚠ glyph fill |
| `color.toast.button.bg` | `theme.chrome.button` | `theme.chrome.button` | 1.0 | toast button background |
| `color.toast.button.bg.hover` | `theme.chrome.button.hover` | `theme.chrome.button.hover` | 1.0 | toast button hover |
| `color.toast.text` | `theme.fg` | `theme.fg` | 1.0 | toast body text |
| `color.picker.bg` | `theme.chrome.surface` | `theme.chrome.surface` | 0.96 | picker modal background |
| `color.picker.scrim` | `#000000` | `#000000` | 0.40 | modal dimming overlay |
| `color.picker.row.bg` | transparent | transparent | 0 | unselected rows |
| `color.picker.row.bg.selected` | `theme.chrome.selection` | `theme.chrome.selection` | 1.0 | selected row background |
| `color.picker.row.accent_bar` | `profile.tint`, or `theme.accent` if profile has no tint | same | 1.0 | 2 px left-edge bar on selected row |
| `color.picker.row.text` | `theme.fg` | `theme.fg` | 1.0 | row label |
| `color.picker.row.disabled` | `theme.fg.muted` | `theme.fg.muted` | 0.55 | Codespace/DevTunnel rows |
| `color.picker.row.suffix` | `theme.warning` | `theme.warning` | 1.0 | `Phase 6+` label |
| `color.link.hover` | `theme.link` | `theme.link` | 1.0 | OSC-8 dotted underline |
| `color.ime.preedit.underline` | `theme.fg.muted` | `theme.fg.muted` | 1.0 | IME preedit underline |

**Dotted-underline dash pattern** (`color.link.hover`): **2 px dash, 2 px gap**, drawn 1 px below the cell baseline at 1 px thickness.

**Tint-stripe transparency:** stripe is opaque (alpha 1.0). The profile color is the whole point — do not dilute it.

---

## 5. Component Inventory

Every component in Phase 5 is listed below with its anatomy, sizing, state, accessibility contract, and motion. Plan tasks reference these by name.

### 5.1 `TintStripe`

| Field | Value |
|-------|-------|
| Surface | Single wgpu quad, 1 pipeline (passthrough fragment), 1 vertex buffer |
| Size | Window content width × `chrome.tintstripe.height` (28 px), or 0 × 0 when no `profile.tint` |
| Fill | `color.tintstripe` (= `profile.tint` resolved from active pane's profile) |
| Anchors | Top-left of NSWindow content rect, **below** AppKit titlebar, **above** system tab bar |
| Per-window vs per-pane | **Per-window**, reflecting the **active pane's** profile (since panes can have different profiles) |
| State changes | On active-pane change → re-paint stripe with new profile's tint (instant; no animation). On profile reload → repaint instant. |
| Accessibility | Decorative. **No** VoiceOver label. Stripe is not focusable. |
| Motion | None. Color change is instantaneous to keep profile identity unambiguous. |

### 5.2 `SearchBar`

| Field | Value |
|-------|-------|
| Surface | wgpu rect + 6 child quads + glyph runs |
| Size | (active pane width − 2 × `spacing.2`) × `chrome.searchbar.height` (32 px) |
| Anchors | Bottom-inside of active pane content rect, inset by Phase-4 1 px border |
| Anatomy | `[ / {query} /▢ ]  [aA]  [↑]  [↓]  [{i}/{n}]  [ × ]` |
| Layout | Query field flex-grows; right-side cluster has fixed widths: toggle icon 16 px, arrow buttons 24 px each, counter min 48 px, close 24 px. Each gap = `spacing.2`. |
| Toggles visible? | **No.** Smart-case + always-regex are silent (D-77). The `aA` is a **status indicator**, not a button — dims when query has no upper-case characters. |
| State machine | `idle → matching → has_matches → active_match` + `→ no_match` + `→ overflow_1000_plus` |
| Counter format | `{active}/{total}` when total ≤ 1000; `{active}/1000+` when overflow (D-76 lazy step) |
| Open trigger | `Cmd-F` while pane has focus |
| Close triggers | `Esc`, click on `×`, click outside the bar (focus loss). Closing restores focus to the pane grid. |
| Empty query state | Hide arrows + counter; show only `[ /  /▢ ]` and `[ × ]` |
| No-match state | Bar tints `color.search.no_match.bg`; counter shows `0/0` |
| Accessibility | `NSAccessibilityRole = textField` for input; arrows = `button` with labels `previous match` / `next match`; counter = `staticText` with label `match {i} of {n}` (or `match {i} of more than 1000`); close = `button` label `close find` |
| Focus capture | Bar **captures** keyboard focus while open. Terminal grid does **not** receive keypresses. PTY continues to receive output but no input. Tab key cycles: query → ↑ → ↓ → × → query. |
| Motion | Open: instant (no fade — D-83 minimal motion). Close: instant. Match step: **no pulse**; the active match's 1 px border is the only indicator (R-Phase5-13). |

### 5.3 `ProfilePicker`

| Field | Value |
|-------|-------|
| Surface | Centered modal — scrim quad + panel quad + per-row quads + glyph runs |
| Size | `clamp(longest_label_px + spacing.12, chrome.picker.width.min, chrome.picker.width.max)` × `chrome.picker.input.height + N × chrome.picker.row.height` where `N = min(matching_profile_count, chrome.picker.max_rows_visible)` |
| Anchors | Horizontal: center of NSWindow content rect. Vertical: 25 % from top. |
| Anatomy | Filter input (32 px) + filtered row list. Each row: `[●/⚠ icon, 12 px] [spacing.2] [profile label, flex] [spacing.2] [suffix label, optional]` |
| State machine | `closed → open → filtered → selected → closed` |
| Fuzzy match | Filters as the user types (D-75). No visible match highlighting on matched chars in Phase 5 — too noisy at 11 pt. |
| Selected row | First match after filter; arrow keys move; Enter commits. |
| Kind handling | `local` profiles enabled. `codespace` / `devtunnel` profiles dim + show `Phase 6+` suffix; Enter on those emits an info toast `profile kind not available until phase 6` and keeps the picker open. |
| Open trigger | `Cmd-Shift-P` while window has focus |
| Close triggers | `Esc`, click outside panel, `Enter` on enabled row, `Enter` on disabled row (with toast, see above). On close (successful switch), restore focus to the pane grid. |
| Scroll | Mouse wheel + arrow keys when row count > `max_rows_visible`. No visible scrollbar; small fade-out top/bottom gradient (2 px) hints overflow. |
| Accessibility | Picker root = `group` with label `switch profile`. Input = `textField` label `filter profiles`. Rows = `menuItem` with name = profile label; disabled rows are announced as `dimmed, available in phase 6 or later`. |
| Focus | Input field is focused on open. Tab moves focus into the row list; Shift-Tab back to input. |
| Motion | Open: instant. Close: instant. (Respects Reduce Motion trivially.) Scrim fades in/out 80 ms only when Reduce Motion is **off** — the fade is the one motion in the picker. |

### 5.4 `ToastBanner`

| Field | Value |
|-------|-------|
| Surface | wgpu rect spanning window content width, anchored top of content area (below tab bar) |
| Modes | `info` (height 36 px, auto-dismiss 5 s) and `action` (height 56 px, until-dismissed) |
| Anatomy — info | `[ ⓘ icon, 16 px ]  [ text ]  [ × close, 16 px ]` |
| Anatomy — action | Line 1: `[ ⚠ icon, 16 px ]  [ prompt text ]` Line 2: `[ button ]  [ button ]  [ button ]  ←spacer→  [ × close ]` |
| Stack behavior | Max **1** toast visible at a time. New toast while one is showing → newer replaces older immediately (older logged to tracing for debug). |
| State machine | `idle → showing → dismissed` (with `auto_dismiss_timer` substate for info mode) |
| Accessibility | Root = `alert` (`NSAccessibilityRoleAlert`). Action buttons have explicit labels (see copywriting §6). Close button label `dismiss notification`. |
| Focus | `info` mode: does **not** steal focus. `action` mode: takes focus, first button is default-focused; Tab cycles buttons → close → first button. |
| Motion | Fade-in 120 ms ease-out, fade-out 200 ms ease-out. **Reduce Motion: both instant.** |

### 5.5 `IMEPreedit`

| Field | Value |
|-------|-------|
| Surface | Existing Phase-3 cell pipeline + underline attribute (D-81) |
| Size | Variable (length of preedit string × cell width) × 1 cell height |
| Anchor | At the cursor position in the active pane |
| Style | Underline color `color.ime.preedit.underline`; preedit glyphs use the pane's foreground color but **dimmed by 0.7** (multiply alpha) |
| Commit | Enter → write committed string to PTY, clear preedit. |
| Cancel | Esc → drop preedit silently. |
| Accessibility | The IME owns VoiceOver announcement for preedit; we expose preedit state via `NSTextInputClient` callbacks correctly so VoiceOver can read it. |
| Motion | None. |

### 5.6 `OSC8HyperlinkHover`

| Field | Value |
|-------|-------|
| Surface | wgpu line segments under the contiguous run of cells sharing one OSC-8 hyperlink id |
| Trigger | Mouse hover with cursor **over** a cell whose attributes have `hyperlink_id != null` |
| Visual | Dotted underline `color.link.hover`, dash pattern 2 px / 2 px, thickness 1 px, baseline + 1 px below cell |
| Modifier indicator | When `Cmd` is held while hovering, the cursor becomes the macOS hand cursor (via `NSCursor.pointingHand`). Without Cmd, normal text I-beam. |
| Click | `Cmd-Click` → `NSWorkspace.shared.open(URL)`; rejects non-http(s) and `file://` (D-78). On rejection, info toast: `vector only opens http and https links`. |
| Hit target | The full contiguous run of cells sharing the hyperlink id (not just one cell). Minimum hit width: 1 cell. |
| Accessibility | Hyperlinks expose `NSAccessibilityRoleLink` via the grid's `NSTextInputClient` shim with the URL as `AXURL`. |
| Motion | None. Underline appears/disappears instantly on hover enter/leave. |

### 5.7 `OSC133PromptMark` (data-only in Phase 5)

| Field | Value |
|-------|-------|
| Visual affordance | **None in Phase 5.** Prompt marks are captured into the cell-attribute stream and the scrollback index; no chevrons, no gutter glyph, no margin column. |
| Note for reviewers | If you expected a visible prompt indicator: it is intentionally deferred. Navigation UI (jump-to-prev-prompt) and any visible marker land in a later phase. The CONTEXT decision is explicit (D-79). |

### 5.8 AppKit menu items (system chrome — not wgpu)

| Location | Item | Modifier | State |
|----------|------|----------|-------|
| `Vector` menu | `About Vector` | — | existing |
| `Vector` menu | `Secure Keyboard Entry` | — | **new**; checkmark when active (`NSMenuItem.state = .on`) |
| `Vector` menu | `Switch Profile ▸` | — | **new** submenu; lists all profiles; selecting one performs the same swap as the picker; Codespace/DevTunnel items disabled with `(phase 6+)` suffix in title |
| `Vector` menu | separator | — | existing |
| `Vector` menu | `Quit Vector` | `Cmd-Q` | existing |
| `File` menu | `New Window` | `Cmd-N` | **new**, ungrouped from any existing items |
| `File` menu | `New Tab` | `Cmd-T` | existing (from Phase 2) |

Menu items are drawn by AppKit — no design tokens apply.

---

## 6. Copywriting Contract

Voice: terse, lowercase-friendly, macOS-native (Apple HIG). No emoji. No exclamation marks. No "please." Sentence case for prose; lowercase for buttons.

### 6.1 Toast strings

| Trigger | Mode | Exact string |
|---------|------|--------------|
| Config TOML parse error | info | `config error at line {n}: {reason}` — e.g. `config error at line 12: invalid key "foo"` |
| Config field requires restart (e.g. `gpu_backend`) | info | `restart required for: {field}` |
| Config field warning (e.g. font-family change on running session) | info | `{field} change applies to new panes` |
| Clipboard write request | action | `allow "{profile_name} : {foreground_process}" to write to your clipboard?` |
| Clipboard write — buttons | action | `allow once` • `always` • `block` |
| Clipboard read request (if reached in P5) | action | `allow "{profile_name} : {foreground_process}" to read your clipboard?` |
| Profile kind not available | info | `profile kind not available until phase 6` |
| Kind-mismatched spawn attempt | info | `{profile_name} requires phase 6+ — opened with default profile` |
| OSC-8 unsupported scheme | info | `vector only opens http and https links` |
| OSC-8 click failed (system rejected) | info | `could not open link` |
| Generic close button label | both | `dismiss notification` (VoiceOver only; visual is `×`) |

### 6.2 Profile picker strings

| Element | Exact string |
|---------|--------------|
| Empty filter placeholder | `filter…` |
| Suffix on Codespace/DevTunnel rows | `Phase 6+` |
| Empty result (no profiles match filter) | `no matches` (rendered as a single dim row, not interactive) |

### 6.3 Search bar strings

| Element | Exact string |
|---------|--------------|
| Match counter — normal | `{i}/{n}` (e.g. `3/142`) |
| Match counter — overflow | `{i}/1000+` |
| Match counter — none | `0/0` |
| Toggle status — smart-case status indicator label (VoiceOver) | `case sensitive` (when active) / `case insensitive` (when inactive) |
| VoiceOver — empty query | `find in pane` |
| VoiceOver — non-empty query | `finding "{query}", {i} of {n}` |

### 6.4 Menu strings

| Menu item | Exact title |
|-----------|-------------|
| Vector → Secure Keyboard Entry | `Secure Keyboard Entry` |
| Vector → Switch Profile ▸ | `Switch Profile` |
| File → New Window | `New Window` |
| (Submenu disabled item suffix) | `{profile name} (phase 6+)` |

### 6.5 Destructive actions in this phase

There are **no destructive actions** in Phase 5 chrome. `block` in the clipboard prompt is a default-state choice, not destruction. `Quit Vector` is existing AppKit standard.

---

## 7. Motion Contract

| Element | Trigger | Duration | Curve | Reduce-Motion behavior |
|---------|---------|----------|-------|------------------------|
| Toast (info or action) — fade-in | mount | 120 ms | ease-out | instant |
| Toast — fade-out | dismiss or 5 s timeout | 200 ms | ease-out | instant |
| Picker scrim | open/close | 80 ms | linear | instant |
| Picker panel | open/close | instant | — | instant |
| Tint-stripe color change | active pane switch / profile reload | instant | — | instant |
| Active-search-match border | step | instant (no pulse) | — | instant |
| OSC-8 hover underline | mouse enter / leave | instant | — | instant |
| IME preedit | character add/remove | instant | — | instant |

**Reduce Motion** is honored globally by reading `NSWorkspace.shared.accessibilityDisplayShouldReduceMotion`. When `true`, every duration above collapses to 0 ms. The toast and picker still appear/disappear — they just snap.

**Frame budget:** Every animated transition must fit within Phase-3's 16.67 ms render budget. Fade animations are alpha lerps on a single quad — well under budget.

---

## 8. Accessibility Contract

### 8.1 VoiceOver

Every chrome surface must expose itself through `NSAccessibility` correctly. The component table in §5 lists the per-component contract; the consolidated requirements:

- All buttons (search arrows, search close, toast buttons, toast close, picker rows) have `accessibilityLabel` set to a human string from §6 — **not** an icon character.
- The toast banner is `NSAccessibilityRoleAlert`, which VoiceOver announces automatically on mount.
- The profile picker is `NSAccessibilityRoleGroup` with `accessibilityLabel = "switch profile"`; rows are `NSAccessibilityRoleMenuItem` so VoiceOver enters list-navigation mode.
- The search bar input is `NSAccessibilityRoleTextField`; the active-match counter is `NSAccessibilityRoleStaticText` and updates `accessibilityValueChanged` on each step.
- OSC-8 hyperlinks expose `AXURL` so the rotor can list them; this is the only data-driven a11y surface in Phase 5.

### 8.2 Keyboard

- Every action reachable by mouse is reachable by keyboard. Specifically:
  - Search bar arrows: `↑` / `↓` (or `Cmd-G` / `Cmd-Shift-G` standard macOS).
  - Picker selection: arrow keys + Enter.
  - Toast buttons: Tab to cycle, Space/Enter to activate.
- `Esc` consistently means "close the current ephemeral surface" (search bar, picker, action toast).
- Tab order is documented per-component in §5.

### 8.3 Hit-target minimums

Mouse targets are at least **24 × 24 px** (4 px more generous than HIG's 20 px minimum):

- Search bar arrow / close buttons: 24 × 24 px (16 px icon + 4 px padding all sides).
- Toast close: 24 × 24 px.
- Picker row: full row width × 28 px (always ≥ 24).
- OSC-8 hyperlink: 1 cell minimum, but always the full contiguous run (typically much larger).

### 8.4 Focus return

| Surface closed | Focus returns to |
|----------------|------------------|
| Search bar | active pane grid |
| Profile picker (any path) | active pane grid (possibly with new profile applied) |
| Action toast (button click or dismiss) | active pane grid |
| Info toast (auto-dismiss or close) | wherever focus was — info toasts never stole focus to begin with |

### 8.5 Color contrast

All chrome text on chrome surfaces must meet **WCAG AA contrast (4.5:1)** in both `Vector Light` and `Vector Dark`. The `theme.chrome.surface` and `theme.fg` token values shipped with the two bundled themes are validated to meet this. Per-profile `.itermcolors` overlays do **not** override chrome tokens (they only override grid colors), so user themes cannot break chrome contrast.

Tint-stripe color is decorative; no contrast requirement applies to it.

### 8.6 Secure Keyboard Entry indicator

When Secure Keyboard Entry is active (D-80), the menu item shows a checkmark **and** the AppKit traffic-light area gets the system-provided lock glyph (this is OS-supplied; we just enable the mode). No additional chrome needed.

---

## 9. Theme Integration

### 9.1 Theme palette extensions (additive to whatever Phase 4 left)

The `vector-theme` palette must add the following keys (default values shown for `Vector Dark` / `Vector Light`):

| Palette key | Vector Dark | Vector Light | Notes |
|-------------|-------------|--------------|-------|
| `theme.chrome.surface` | `#1c1c1ee6` | `#f4f4f5e6` | translucent neutral; 90 % opaque |
| `theme.chrome.divider` | `#3a3a3c` | `#d1d1d6` | 1 px hairline |
| `theme.chrome.button` | `#2c2c2e` | `#ffffff` | toast button bg |
| `theme.chrome.button.hover` | `#3a3a3c` | `#e5e5ea` | toast button hover bg |
| `theme.chrome.selection` | `#0a84ff33` | `#007aff22` | picker selected-row bg |
| `theme.search.highlight.dark` | `#ffd60a` | n/a | yellow family, used at alpha 0.40 |
| `theme.search.highlight.light` | n/a | `#ff9500` | orange family, used at alpha 0.40 |
| `theme.warning` | `#ffd60a` | `#ff9500` | toast info icon, picker `Phase 6+` |
| `theme.danger.subtle` | `#ff453a` | `#ff3b30` | search no-match bar tint (alpha 0.20) |
| `theme.link` | `#0a84ff` | `#007aff` | OSC-8 hover underline |
| `theme.fg.muted` | `#8e8e93` | `#8e8e93` | disabled picker rows, IME preedit underline |

### 9.2 Appearance resolution

`[default].appearance = "system" | "light" | "dark"` (D-72). On `system`, we observe `NSWindow.effectiveAppearance` and re-resolve all chrome tokens when it flips. The flip is **instant** for chrome (we are not animating across themes).

Per-profile `.itermcolors` files override **grid** colors only (`theme.bg`, ANSI 0–15, cursor, selection). They do **not** override the `theme.chrome.*` family. Rationale: the chrome must remain readable regardless of how exotic the user's profile theme is.

### 9.3 Tint resolution

`profile.tint = "#RRGGBB"` (optional, D-74). Resolves to `color.tintstripe`. If absent, `chrome.tintstripe.height` collapses to 0 and the stripe pass is skipped entirely (perf: no draw call).

---

## 10. Component <-> Requirement Cross-Reference

| Requirement | Component(s) | UI tokens |
|-------------|-------------|-----------|
| POLISH-01 (config hot-reload + errors) | ToastBanner (info) | `chrome.toast.height.info`, `color.toast.info.bg`, copywriting §6.1 lines 1–3 |
| POLISH-02 (clipboard authorization) | ToastBanner (action) | `chrome.toast.height.action`, `color.toast.action.bg`, copywriting §6.1 lines 4–6 |
| POLISH-03 (OSC 7 cwd tracking) | none — data only, no UI affordance | n/a |
| POLISH-04 (OSC 8 hyperlinks) | OSC8HyperlinkHover | `color.link.hover`, dotted-underline pattern §4.4, copywriting §6.1 lines 9–10 |
| POLISH-05 (OSC 9 / system bell) | none — system notification API + audio cue (AppKit) | n/a |
| POLISH-06 (find-in-pane) | SearchBar | all `chrome.searchbar.*` + `color.search.*`, copywriting §6.3 |
| POLISH-07 (profile picker + tint + menu) | ProfilePicker, TintStripe, AppKit menu | `chrome.picker.*`, `color.picker.*`, `chrome.tintstripe.height`, `color.tintstripe`, copywriting §6.2 + §6.4 |
| POLISH-08 (Secure Input, New Window, IME) | AppKit menu items, IMEPreedit | §5.5, §5.8, copywriting §6.4 |
| OSC 133 (prompt marks) — D-79 | none in Phase 5 (deferred); §5.7 note | n/a |

---

## 11. Layout Boundary Rules (Active-Pane Border Interaction)

The Phase-4 active-pane border (D-66) is a 1 px hairline drawn just inside the active pane's content rect. Phase-5 chrome interacts with it as follows:

1. **Search bar** is drawn **inside** the border. The border continues underneath the search bar's bottom edge. The search bar's own top hairline (`color.search.bar.border`) sits 1 px above the active-pane border's bottom segment, creating a visible 2-line separator (intentional — keeps the search bar visually distinct from the grid).
2. **Tint stripe** is drawn **outside** all panes — it is window chrome, not pane chrome. It never interacts with the active-pane border.
3. **Profile picker** is modal over the whole window — covers the active-pane border without disturbing it (border is drawn underneath; picker scrim sits on top).
4. **Toast banner** is window chrome — sits above the active-pane border, anchored to the window content top.
5. **OSC-8 hover** lives inside the grid — the hover underline is drawn inside the pane content rect, never crosses the active-pane border.
6. **IME preedit** is inline in the grid, same containment as OSC-8.

---

## 12. Pre-Populated From Upstream — Source Audit

| Field | Source | Decision id / reference |
|-------|--------|-------------------------|
| Tint stripe location, color, height | CONTEXT.md D-75 + RESEARCH.md R-Phase5-13 | locked |
| Profile picker layout, fuzzy behavior, Codespace dimming | CONTEXT.md D-75 + REQUIREMENTS POLISH-07 | locked |
| Search bar layout `[/{q}/▢] [aA] [↑] [↓] [{i}/{n}] [×]` | CONTEXT.md D-76, D-77 | locked |
| Search smart-case + always-regex, no toggles | CONTEXT.md D-77 | locked |
| Search highlight colors (yellow/orange, alpha 0.40) + active-match 1 px border | CONTEXT.md D-77 + RESEARCH R-Phase5-13 | locked |
| Toast — two modes (auto-dismiss vs until-dismissed) | REQUIREMENTS POLISH-01, POLISH-02 + CONTEXT D-69, D-70 | locked |
| Clipboard prompt copy `Allow … to write to your clipboard? [Allow once] [Always] [Block]` | CONTEXT D-70 | adapted to lowercase macOS-native voice in §6.1 |
| IME inline preedit, no candidate window | CONTEXT D-81 | locked |
| OSC-8 Cmd-click, http/https only | CONTEXT D-78 | locked |
| OSC 133 — capture only, no UI | CONTEXT D-79 | locked |
| Menu items (Secure Input, Switch Profile, New Window) | CONTEXT D-75, D-80, D-82 | locked |
| Light/Dark via macOS effectiveAppearance | CONTEXT D-72 | locked |
| JetBrains Mono for grid | CONTEXT D-41 (Phase 3) | inherited |
| System font for chrome | new this UI-SPEC | researcher choice (HIG-aligned) |
| 4 px grid + 28/32/36/56 surface heights | new this UI-SPEC | researcher choice (HIG-aligned) |
| 3 chrome font sizes, 2 weights | new this UI-SPEC | researcher choice (discipline) |

---

## 13. Open Questions for Checker / Auditor

None — every required field is filled. Items the checker should pay particular attention to:

1. **Accent reservation list** (§4.2) is strict. If any plan task introduces accent into toast or search highlight fills, the auditor must flag it.
2. **No motion on tint stripe** (§7) is deliberate. Do not let the executor add a "smooth transition" — instant color change is the contract.
3. **Search active-match indicator is a border, not a pulse** (§5.2). Static, full alpha, no animation.
4. **Per-profile `.itermcolors` does NOT override chrome tokens** (§9.2). If the executor wires the overlay into `theme.chrome.*`, that is a contract violation.
5. **`Phase 6+` items are dimmed and announced as such** (§5.3) — not hidden. Hiding them would lose the discoverability win of the picker.

---

*End of UI-SPEC for Phase 5.*
