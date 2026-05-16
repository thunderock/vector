---
phase: 06
phase_name: GitHub Auth + Codespaces Picker
slug: github-auth-codespaces-picker
status: draft
design_system: none (native AppKit NSPanel + in-house wgpu compositor; inherits Phase 5 chrome tokens)
shadcn_initialized: false
preset: not applicable
component_library: AppKit (objc2-app-kit 0.3) for menu items + NSPanel modals; wgpu compositor (Phase-3 pipeline + Phase-5 chrome passes) for inline chrome
icon_library: SF Symbols via NSImage.symbolWithName (system-supplied); ASCII glyphs in wgpu surfaces
font: JetBrains Mono (grid) + macOS system font SF Pro via NSFont.systemFont (chrome)
created: 2026-05-14
sources:
  - .planning/REQUIREMENTS.md (AUTH-01..03, CS-01..03)
  - .planning/ROADMAP.md (§Phase 6)
  - .planning/phases/06-github-auth-codespaces-picker/06-CONTEXT.md (D-84..D-90)
  - .planning/phases/06-github-auth-codespaces-picker/06-RESEARCH.md (oauth2 5.0, octocrab 0.50, NSPanel patterns)
  - .planning/phases/05-polish-local-daily-driver/05-UI-SPEC.md (chrome tokens — inherited verbatim)
  - CLAUDE.md (Rust workspace, macOS-only, wgpu/winit/AppKit constraints)
---

# UI-SPEC — Phase 6: GitHub Auth + Codespaces Picker

## 0. Scope & Frame

Phase 6 adds two ephemeral modal surfaces on top of Vector's existing chrome:

| Surface | Technology | Purpose |
|---------|------------|---------|
| Device-flow modal (D-85) | **AppKit `NSPanel`** via `objc2-app-kit 0.3` | Show 8-char user code + verification URL during OAuth |
| Codespaces picker modal (D-86) | **AppKit `NSPanel`** | Live-fetched codespaces list with state badges + per-row actions |
| Menu items (D-84, D-86) | AppKit menu bar | `Vector → Sign in with GitHub`, `Vector → Codespaces…`, `Vector → Sign out` |
| Sign-in / connect-stub feedback | **wgpu Toast pass** (Phase 5 `ToastBanner` reused) | Status surface for sign-in success, connect placeholder, errors |

Phase 6 introduces **no new design system primitives**. It binds new components to the spacing, typography, color, and motion tokens locked by Phase 5 UI-SPEC §§2–4, 7. The new contracts in this document are:

1. NSPanel modal sizing + anatomy for two new surfaces.
2. Live-state badge color mapping for `CodespaceState`.
3. New copywriting for OAuth + Codespaces toasts.
4. Profile-name derivation algorithm for D-87 saves.

Where this spec is silent, **Phase 5 UI-SPEC governs**. The two specs compose without conflict.

---

## 1. Visual Mockups

ASCII mockups are normative. All dimensions are 1.0× device-pixel ratio; multiply by `NSScreen.backingScaleFactor` for retina.

### 1.1 Device-flow modal (AUTH-01, D-85)

```
   ┌──────────────────────────────────────────────────┐
   │  Sign in to GitHub                            ×  │ ← NSPanel titlebar (titled+closable)
   ├──────────────────────────────────────────────────┤
   │                                                  │
   │   Enter this code at github.com/device:          │   font.chrome.body 13pt
   │                                                  │
   │              ┌───────────────────┐               │
   │              │   W D J B - M J H │               │   user-code: JetBrains Mono 32pt semibold,
   │              └───────────────────┘               │   monospaced; 8 chars + dash → 9 glyphs
   │                                                  │
   │   Code copied to clipboard. Expires in 14:32.    │   font.chrome.small 11pt; countdown ticks
   │                                                  │
   │   ┌────────────────────────────────────┐         │
   │   │  Copy code and open github.com/…   │         │ ← primary button, 36px tall, full-width-ish
   │   └────────────────────────────────────┘         │
   │                                                  │
   │   [ Cancel sign-in ]                             │ ← secondary, plain, left-aligned
   │                                                  │
   └──────────────────────────────────────────────────┘
```

**Layout invariants**
- Fixed size: **440 × 280 px** (does not resize). Centered over the active NSWindow content rect.
- NSPanel style: `NSWindowStyleMask::Titled | NSWindowStyleMask::Closable`. Window level: `NSFloatingWindowLevel`. **Never** `.modalPanel` (Pitfall 3 — strands key-window status).
- Closing the titlebar `×` is equivalent to Cancel sign-in.
- The user-code field is **selectable, not editable**; click selects the whole code (re-copies to clipboard, info toast `code copied`).
- Token is **never** displayed. Only the 8-char user code (which is harmless — it pairs the device, not the account).
- Countdown updates every second; at 0, the modal closes itself and emits info toast `sign-in code expired — try again`.

### 1.2 Codespaces picker modal (CS-01, D-86)

```
   ┌────────────────────────────────────────────────────────────────────┐
   │  Codespaces                                              ⟳     ×   │ ← titlebar; refresh icon + close
   ├────────────────────────────────────────────────────────────────────┤
   │   ⚪ search…                                                       │ ← filter input, 32px (chrome.picker.input.height)
   ├────────────────────────────────────────────────────────────────────┤
   │  ● Available  octocat/hello-world      main           2 hours ago  │ ← row 1, selected
   │               [ Connect to codespace ]  [ Save as profile ]        │   action row inset, fades in on select
   │  ◌ Starting   colligo/vector           phase3         4 minutes ago│   yellow dot, animated ring
   │  ○ Shutdown   adobe/design-system      v2.0           yesterday    │   gray dot
   │               [ Start codespace ]  [ Save as profile ]             │
   │  ⊘ Failed     myorg/legacy-app         hotfix         3 days ago   │   red dot (semantic: danger)
   │  ?  Unknown   anonymous-repo           ???            2 weeks ago  │   placeholder for Unrecognized state
   ├────────────────────────────────────────────────────────────────────┤
   │   5 codespaces · last refreshed just now                           │ ← footer, font.chrome.small 11pt
   └────────────────────────────────────────────────────────────────────┘
```

**Layout invariants**
- Fixed width: **640 px** (wider than Phase 5 profile picker because of 4-column row). Min height 320 px, max height = `min(NSWindow content height − 80 px, 560 px)`. Scroll on overflow.
- Centered horizontally; vertically anchored 15 % from top (slightly higher than Phase 5 picker because content is taller).
- NSPanel style: `NSWindowStyleMask::Titled | NSWindowStyleMask::Closable`. Window level: `NSFloatingWindowLevel`. Stays above the main window but does not steal key from Cmd-Q.
- Filter input is focused on open; fuzzy-matches against `repository.full_name + " " + git_status.ref + " " + display_name` (concatenated haystack).
- Row height: **44 px collapsed, 76 px expanded** when selected. Action buttons appear in the expanded row only — keeps unselected rows scan-friendly.
- 4 columns per row: state badge (16 px + 8 px label = 88 px fixed), repo name (flex 2), branch (flex 1, monospace), last-used (96 px fixed, right-aligned).
- Header row contains refresh icon (24×24 hit target, SF Symbol `arrow.clockwise`) and titlebar `×`.
- Empty-list state: replaces the row area with a single centered string `no codespaces found` in `font.chrome.body`, `color.fg.muted`. Footer counter then reads `0 codespaces`.
- Loading state (during fetch): row area shows centered spinner (SF Symbol `circle.dotted` rotating 360° / 1.2 s) + label `loading codespaces…`.
- Error state: row area shows centered `could not fetch codespaces — check your connection` + button `[ Try again ]`.

### 1.3 OAuth flow — info-toast lifecycle (Phase 5 ToastBanner reused)

```
   ┌────────────────────────────────────────────────────────────────────┐
   │ ⓘ signed in as @octocat                                         ×  │ ← info toast, 36px, dismisses 5s
   ├────────────────────────────────────────────────────────────────────┤
```

```
   ┌────────────────────────────────────────────────────────────────────┐
   │ ⓘ codespace ssh transport not yet wired — phase 7              ×  │ ← shown when user clicks Connect to codespace
   ├────────────────────────────────────────────────────────────────────┤
```

```
   ┌────────────────────────────────────────────────────────────────────┐
   │ ⚠ sign-in code expired — try again                             ×  │ ← shown when device flow times out
   ├────────────────────────────────────────────────────────────────────┤
```

### 1.4 Profile-picker integration (Phase 5 Cmd-Shift-P picker — unchanged appearance)

```
                    ┌──────────────────────────────────────┐
                    │ ▸ filter…                            │
                    ├──────────────────────────────────────┤
                    │ ● default                            │
                    │ ● hello-world                        │ ← NEW: saved via D-87
                    │ ● work-codespace                     │ ← previously Phase 6+ disabled;
                    └──────────────────────────────────────┘   now ENABLED (clicking auto-triggers
                                                                sign-in if no token, then shows the
                                                                connect-stub toast — D-84)
```

**Behavior change vs Phase 5**: `Kind::Codespace` profile rows in the Cmd-Shift-P picker are **enabled** in Phase 6 (Phase 5 dimmed them with `Phase 6+`). Clicking such a row:
1. If no valid token → opens device-flow modal (D-84 second trigger path).
2. Once token valid → emits info toast `codespace ssh transport not yet wired — phase 7` (placeholder; Phase 7 replaces with real connect).

`Kind::DevTunnel` rows remain dimmed with `Phase 8+` suffix.

---

## 2. Spacing Tokens

**Inherits Phase 5 UI-SPEC §2 verbatim.** All Phase 6 chrome uses the 4 px grid.

New surface heights added for the two modals:

| Token | Value | Used For |
|-------|-------|----------|
| `chrome.auth_modal.width` | 440 px | device-flow modal width |
| `chrome.auth_modal.height` | 280 px | device-flow modal height |
| `chrome.auth_modal.code_field.height` | 56 px | user-code display field |
| `chrome.auth_modal.button.primary.height` | 36 px | "Copy code and open…" button |
| `chrome.auth_modal.button.secondary.height` | 28 px | "Cancel sign-in" button |
| `chrome.cs_picker.width` | 640 px | codespaces picker modal width |
| `chrome.cs_picker.height.min` | 320 px | floor |
| `chrome.cs_picker.height.max_offset` | 80 px | subtracted from window height for ceiling |
| `chrome.cs_picker.row.height.collapsed` | 44 px | unselected row (state + repo + branch + time) |
| `chrome.cs_picker.row.height.expanded` | 76 px | selected row (adds action button strip) |
| `chrome.cs_picker.header.height` | 32 px | inherits chrome.picker.input.height |
| `chrome.cs_picker.footer.height` | 24 px | counter + last-refreshed |
| `chrome.cs_picker.col.state.width` | 88 px | badge (16 px dot + 8 px gap + label) |
| `chrome.cs_picker.col.time.width` | 96 px | last-used relative time (right-aligned) |
| `chrome.cs_picker.col.repo.weight` | 2 | flex weight |
| `chrome.cs_picker.col.branch.weight` | 1 | flex weight |
| `chrome.cs_picker.button.action.height` | 28 px | row-level action buttons (Connect to codespace, Start codespace, Save as profile) |

**Exceptions:** none. Every value is a multiple of 4.

---

## 3. Typography

**Inherits Phase 5 UI-SPEC §3 verbatim.** Two fonts (`font.grid` = JetBrains Mono, `font.chrome` = SF Pro). Three sizes, two weights.

New role added for the device-flow modal:

| Token | Family | Size | Weight | Line Height | Used For |
|-------|--------|------|--------|-------------|----------|
| `font.auth.code` | JetBrains Mono | 32 pt | semibold (600) | 1.0 | user-code display (`WDJB-MJHT`) |

The 32 pt code display does **not** violate the "3 sizes, 2 weights" discipline because `font.grid` is the same family as Phase 3 — it's a re-use, not a new chrome typeface. Weight (semibold/600) is already on the Phase 5 register (`font.chrome.button`). Effective new typography contribution: **zero new font families, zero new weights, one new render context** (large code display).

| Role | Size | Weight | Line Height |
|------|------|--------|-------------|
| Body | 13 pt | 400 | 1.4 |
| Label / counter | 11 pt | 400 | 1.4 |
| Button | 13 pt | 600 | 1.3 |
| Display (code) | 32 pt | 600 | 1.0 |

---

## 4. Color Contract

**Inherits Phase 5 UI-SPEC §4 verbatim.** 60/30/10 split unchanged. Accent reserved-for list unchanged (Phase 6 adds zero new accent uses).

### 4.1 New tokens added — state badges only

| Token | Vector Dark | Vector Light | Alpha | Surface |
|-------|-------------|--------------|-------|---------|
| `color.state.available` | `#30d158` | `#34c759` | 1.0 | filled circle for `Available` |
| `color.state.starting` | `#ffd60a` | `#ff9500` | 1.0 | open ring + spinner for `Starting`, `Provisioning`, `Queued`, `Updating`, `Rebuilding` |
| `color.state.shutdown` | `#8e8e93` | `#8e8e93` | 1.0 | hollow circle for `Shutdown`, `ShuttingDown`, `Archived` |
| `color.state.failed` | `#ff453a` | `#ff3b30` | 1.0 | `⊘` glyph for `Failed` |
| `color.state.unknown` | `#8e8e93` | `#8e8e93` | 1.0 | `?` glyph for `Unrecognized`, `Created`, `Unknown` |

**Semantic anchoring:** `color.state.available` is **NOT** the same token family as `accent`. It is a semantic-status color, comparable to Phase 5's `color.warning` and `color.danger`. Accent (10 %) remains reserved for the three uses listed in Phase 5 §4.2.

| Token | Vector Dark | Vector Light | Alpha | Surface |
|-------|-------------|--------------|-------|---------|
| `color.auth.code.bg` | `theme.chrome.surface` | `theme.chrome.surface` | 1.0 | user-code field background |
| `color.auth.code.border` | `theme.chrome.divider` | `theme.chrome.divider` | 1.0 | 1 px border around user-code field |
| `color.cs_picker.row.bg.hover` | `theme.chrome.button.hover` | `theme.chrome.button.hover` | 1.0 | row hover background |

**Accent reserved-for list (re-stated, unchanged from Phase 5):**
1. Tint stripe fill.
2. Active-search-match 1 px border.
3. Profile-picker selected-row 2 px left-edge bar.

Codespaces-picker selected-row uses `theme.chrome.selection` (same as profile picker row bg), **not** accent.

### 4.2 Destructive color usage

Phase 6 has **no destructive actions in chrome.** The `Sign out` menu item is destructive in concept but uses a standard menu item (AppKit-drawn). No red button surface. Token `color.danger` remains declared-but-unused in chrome.

`color.state.failed` is a **status** indicator, not a destructive action; it never appears on a button.

---

## 5. Component Inventory

### 5.1 `AuthDeviceFlowModal`

| Field | Value |
|-------|-------|
| Surface | AppKit `NSPanel` (Titled + Closable; `NSFloatingWindowLevel`) |
| Size | Fixed 440 × 280 px |
| Anchor | Centered over active NSWindow content rect |
| Anatomy | (1) heading "Sign in to GitHub" (titlebar), (2) prompt label, (3) user-code field, (4) countdown line, (5) primary button, (6) secondary button |
| Modes | `displaying_code` (steady state) → `submitting` (button pressed, briefly disables UI ~250 ms) → `dismissing` |
| Primary button | Label: `Copy code and open github.com/device`. Action: `NSPasteboard.generalPasteboard.setString(user_code)` + `NSWorkspace.shared.open(URL("https://github.com/login/device"))`. Repeated clicks reset clipboard + re-open URL. |
| Secondary button | Label: `Cancel sign-in`. Action: cancel device-flow poll task, restore previous clipboard contents (Pitfall 7), dismiss modal. Same effect as titlebar `×`. |
| Auto-copy | On modal mount, current `NSPasteboard.generalPasteboard.stringForType(NSPasteboardTypeString)` is captured and held in modal state; user-code is written; on dismiss (success OR cancel), previous contents are restored. |
| Countdown | Re-renders every 1 s. Format: `Code copied to clipboard. Expires in MM:SS.`. At 00:00, modal closes itself and emits info toast `sign-in code expired — try again`. |
| Token redaction | The full access token is **never** displayed (Pitfall 14). Only the 8-char user-code (device-pairing, not account-bearing) is shown. |
| State machine | (a) opened → (b) device-code requested (UI shows spinner until response, max 500 ms loop guard) → (c) displaying_code → (d) polling (no UI change; modal stays) → (e) success → dismiss + toast `signed in as @{login}` / (e') cancel → dismiss / (e'') expired → dismiss + warn toast |
| Accessibility | Root = `NSAccessibilityRoleGroup` label `Sign in to GitHub`. User-code field = `NSAccessibilityRoleStaticText` value = user-code with hyphens spelled out (`W-D-J-B dash M-J-H-T`) for VoiceOver clarity. Primary button = `NSAccessibilityRoleButton` label `Copy code and open device page`. Countdown line = `NSAccessibilityRoleStaticText` updates via `accessibilityValueChanged`. |
| Focus | NSPanel becomes key; primary button is default-focused (Return triggers it). Tab cycles primary → secondary → user-code-field (selectable) → primary. Esc = cancel sign-in. |
| Motion | Open: AppKit-default modal slide (NSPanel built-in); we do not override. Close: AppKit-default. Countdown re-render is text-replace, no animation. |
| Reduce Motion | Honored by AppKit; we add no animations on top. |

### 5.2 `CodespacesPickerModal`

| Field | Value |
|-------|-------|
| Surface | AppKit `NSPanel` (Titled + Closable; `NSFloatingWindowLevel`) |
| Size | Width 640 px fixed; height `clamp(320, NSWindow.content.height - 80, 560)` |
| Anchor | Centered horizontally; 15 % from top of NSWindow content rect |
| Anatomy | Filter input row (32 px) + scrollable row list + footer (24 px) |
| Refresh icon | SF Symbol `arrow.clockwise`, 16 × 16 px glyph in a 24 × 24 px hit target, top-right of titlebar (before `×`). Click triggers re-fetch. Disabled (dimmed via `color.fg.muted`) while fetch is in flight. |
| Row anatomy | `[state badge 16px] [spacing.2] [state label, 64px max] [spacing.4] [repo name, flex 2] [spacing.2] [branch, flex 1] [spacing.2] [last-used, 96px right-aligned]` |
| State badge | A 16 × 16 px glyph in `color.state.{state}`. `Available` = filled disc. `Starting` family = open ring + animated 360° rotation 1.2 s linear (CSS-equivalent; wgpu pass updates transform every frame). `Shutdown` family = hollow disc. `Failed` = `⊘` glyph. `Unknown/Unrecognized` = `?` glyph. Reduce Motion: spinner becomes static open ring. |
| Selected row | Background `color.picker.row.bg.selected` (Phase 5 token, alpha 1.0). **No accent bar** — accent is reserved (§4.1). Selection rendered by background alone. Expands row to 76 px to show action-button strip. |
| Action buttons | Row-level: `[ Connect to codespace ]` (always), `[ Start codespace ]` (only on `Shutdown`-family states), `[ Save as profile ]` (always). Buttons use Phase 5 `chrome.toast.button.*` token family for color + 13 pt semibold label. Height = 28 px (`chrome.cs_picker.button.action.height`). Spacing between buttons = `spacing.2` (8 px). Buttons left-aligned, inset by `spacing.6` (24 px) from row left edge to align under repo column. |
| Connect button click | Always emits info toast `codespace ssh transport not yet wired — phase 7` and keeps the modal open. (Phase 7 replaces this stub with real connect + modal-dismiss.) |
| Start button click | (1) Disables Start codespace button briefly (~250 ms), (2) `POST /user/codespaces/{name}/start` (200/202/409 = success per Pitfall 5), (3) state badge transitions to `Starting`, (4) row begins live polling (Pattern 3, RESEARCH §Architecture). Polling updates badge to `Available` or fails gracefully. |
| Save as profile click | (1) Computes profile name per §5.3 algorithm, (2) calls `vector-config::writer::append_codespace_profile`, (3) emits info toast `profile saved as "{name}"`, (4) keeps modal open. Following the toast, Cmd-Shift-P picker now shows the new profile. |
| Filter | Fuzzy match (Phase 5 SkimMatcherV2 reused) over `repo_full_name + " " + branch + " " + display_name`. Match-rank order. Filter clear → restore full list. |
| Empty list | Row area = centered string `no codespaces found` in `font.chrome.body` muted. |
| Loading | Row area = centered spinner glyph (SF Symbol `circle.dotted` rotating 1.2 s linear) + label `loading codespaces…`. Reduce Motion: static `circle.dotted` glyph. |
| Error | Row area = centered string `could not fetch codespaces — check your connection` + 28-px-tall button `[ Try again ]` directly underneath, both centered. |
| Footer | Left-aligned: `{n} codespace{s} · last refreshed {relative-time}` in `font.chrome.small`. Right-aligned: invisible until polling — then `polling {name} ({state})…` so the user knows what's live. |
| Open trigger | (a) Menu `Vector → Codespaces…`. (b) Keyboard shortcut `Cmd-Shift-G` (D-86). (c) Click on a `Kind::Codespace` row in Cmd-Shift-P picker (D-84 second path) IF user wants to discover not-yet-saved codespaces. |
| Close triggers | (a) Titlebar `×`. (b) Click outside panel (NSPanel `becomesKey = false` handler). (c) `Esc`. (d) Selecting `Save as profile` does NOT close (so user can save multiple). |
| Cancellation | On close, every in-flight `poll_until_available` task is cancelled via shared `CancellationToken` (RESEARCH Pattern 5). The HTTP `list` request, if in flight, is also cancelled. |
| 401 handling | If any HTTP call returns 401 (RESEARCH Pattern 2): silently attempt refresh (or re-run device flow if no refresh token); on retry success, transparent to user. On retry failure: modal closes itself, device-flow modal opens. |
| Accessibility | Panel root = `NSAccessibilityRoleGroup` label `Codespaces`. Filter input = `NSAccessibilityRoleTextField` label `filter codespaces`. Rows = `NSAccessibilityRoleMenuItem`. Each row's `accessibilityLabel` is the full sentence `{repo}, branch {branch}, {state}, last used {relative}` so VoiceOver users get a complete row read in one announcement. State badges are decorative (no separate a11y node). Action buttons have explicit labels: `connect to {repo}`, `start {repo}`, `save {repo} as profile`. Refresh = `button` label `refresh list`. |
| Keyboard | Open lands focus in filter input. `↓` moves focus into row list. `↑/↓` navigate rows. `Enter` on a row activates the **primary button** for that state (`Connect to codespace` for Available/Starting, `Start codespace` for Shutdown). `Cmd-S` while a row is selected = `Save as profile`. `Tab` from row list moves to action buttons within the expanded row. `Esc` closes the modal. |
| Motion | Selection background transition: instant (Phase 5 norm). Row expansion (44 → 76 px) on select: 80 ms ease-out height interpolation. Reduce Motion: instant snap. Spinner rotation: 1.2 s linear loop; Reduce Motion = static frame. State-badge color transition on poll update (e.g. Starting → Available): cross-fade 200 ms ease-out; Reduce Motion = instant. |

### 5.3 Profile-name derivation algorithm (D-87)

When the user clicks **Save as profile** on a codespace named `octocat/hello-world-abc123`:

1. Split on first `/`: `owner = "octocat"`, `rest = "hello-world-abc123"`.
2. Strip a trailing randomized suffix: regex `-[a-z0-9]{4,}$` (case-insensitive). If matched, drop it: `rest → "hello-world"`. If unmatched (e.g. user named it `myproject`), keep `rest` as-is.
3. If `rest` is empty after stripping, fall back to `owner-{codespace_name first 6 chars}`.
4. If `profile.{rest}` already exists in `config.toml`, append `-2`, `-3`, … and re-check until unique.
5. Default `tint = "#7a3aaf"` (D-87) unless a prior `Kind::Codespace` profile in the same config uses a non-default tint — then reuse that.
6. Write the block:
   ```toml
   [profile.{rest}]
   kind = "codespace"
   codespace_name = "{full original codespace name}"
   tint = "#7a3aaf"
   ```

Examples (from RESEARCH fixtures):
- `octocat/hello-world-abc123` → `[profile.hello-world]`
- `colligo/vector-x7k2m1n8` → `[profile.vector]`
- `adobe/design-system-v2` → `[profile.design-system-v2]` (no trailing random suffix matched)
- Collision: second save of `colligo/vector-{different-suffix}` → `[profile.vector-2]`

### 5.4 New menu items (AppKit-drawn — no design tokens)

| Location | Item | Modifier | State / Notes |
|----------|------|----------|---------------|
| `Vector` menu | `Sign in with GitHub` | — | **new**; hidden when token present, replaced by `Sign out (@{login})` |
| `Vector` menu | `Sign out (@{login})` | — | **new**; visible only when signed in; click deletes Keychain entries + emits info toast `signed out` |
| `Vector` menu | `Codespaces…` | `Cmd-Shift-G` | **new**; opens `CodespacesPickerModal`. If no token, opens `AuthDeviceFlowModal` first; on completion, automatically opens the picker. |
| `Vector` menu | separator | — | between auth items and existing Phase 5 items |

Menu titles use **Title Case** (AppKit convention). Toast text uses **lowercase sentence** (Phase 5 convention). The two voices coexist intentionally — menus are macOS-native chrome; toasts are app voice.

### 5.5 Inherited components (Phase 5, used unchanged)

| Component | Inherited from | Phase 6 usage |
|-----------|----------------|---------------|
| `ToastBanner` (info mode) | Phase 5 §5.4 | Sign-in success, sign-out, connect stub, error feedback, code-expired warning |
| `ProfilePicker` (Cmd-Shift-P) | Phase 5 §5.3 | `Kind::Codespace` rows enabled (Phase 5 dimmed them); `Phase 6+` suffix removed for codespace kind, retained for `Kind::DevTunnel` (now reads `Phase 8+`) |
| `TintStripe` | Phase 5 §5.1 | Saved codespace profiles' `#7a3aaf` tint renders the stripe in purple when active |

---

## 6. Copywriting Contract

Voice: terse, lowercase sentence-case for prose; Title Case for menu items only; no exclamation marks; no "please." Token strings never appear in any string in this section (Pitfall 14).

### 6.1 Toast strings (Phase 6 additions)

| Trigger | Mode | Exact string |
|---------|------|--------------|
| Device flow — code expired before user authenticated | info | `sign-in code expired — try again` |
| Device flow — user cancelled at github.com | info | `sign-in cancelled` |
| Device flow — success | info | `signed in as @{login}` |
| Sign-out from menu | info | `signed out` |
| Connect-stub (Phase 7 not yet) | info | `codespace ssh transport not yet wired — phase 7` |
| Save as profile — success | info | `profile saved as "{name}"` |
| Profile name collision auto-suffix | info | `profile saved as "{name}" (renamed to avoid collision)` |
| Codespace start — already starting (409) | info | `codespace is already starting` |
| Codespace start — request failed (non-409 4xx/5xx) | info | `could not start codespace — try again` |
| Network error during fetch | info | `could not fetch codespaces — check your connection` (also rendered as inline empty state, see §5.2) |
| Auth required (401 chain failed) | info | `sign in to github required` (then auto-opens AuthDeviceFlowModal) |
| OSC-8 / clipboard restored on auth-modal close | (none — silent) | clipboard restoration is silent; no toast |

All toasts use info mode (auto-dismiss 5 s). **No action toasts** in Phase 6 — every Phase-6 affordance has an explicit modal or menu item for confirmation. The clipboard-write prompt from Phase 5 is unrelated; it remains action-mode.

### 6.2 Modal copy — device-flow

| Element | Exact string |
|---------|--------------|
| NSPanel titlebar | `Sign in to GitHub` |
| Prompt label | `Enter this code at github.com/device:` |
| Code field caption | `Code copied to clipboard. Expires in {MM:SS}.` |
| Primary button | `Copy code and open github.com/device` |
| Secondary button | `Cancel sign-in` |
| VoiceOver — user-code | spelled out with `-` read as `dash` (handled by NSAccessibility on the StaticText) |

### 6.3 Modal copy — codespaces picker

| Element | Exact string |
|---------|--------------|
| NSPanel titlebar | `Codespaces` |
| Filter placeholder | `search…` |
| State labels (right of badge) | `Available` / `Starting` / `Shutdown` / `Failed` / `Unknown` (mapped from `CodespaceState` variants per §6.4) |
| Empty list | `no codespaces found` |
| Loading | `loading codespaces…` |
| Error | `could not fetch codespaces — check your connection` |
| Retry button | `Try again` |
| Connect button | `Connect to codespace` |
| Start button | `Start codespace` |
| Save button | `Save as profile` |
| Footer counter (singular) | `1 codespace · last refreshed {relative-time}` |
| Footer counter (plural / zero) | `{n} codespaces · last refreshed {relative-time}` |
| Footer counter (never refreshed yet) | `{n} codespaces` |
| Footer poll annotation | `polling {name} ({state})…` |
| Refresh icon a11y label | `refresh list` |
| Close (`×`) a11y label | `close codespaces` |

### 6.4 `CodespaceState` → display label mapping

| API value | Display label | Badge color token |
|-----------|---------------|-------------------|
| `Available` | `Available` | `color.state.available` |
| `Starting` | `Starting` | `color.state.starting` |
| `Provisioning` | `Starting` (subsumed) | `color.state.starting` |
| `Queued` | `Starting` (subsumed) | `color.state.starting` |
| `Updating` | `Starting` (subsumed) | `color.state.starting` |
| `Rebuilding` | `Starting` (subsumed) | `color.state.starting` |
| `Created` | `Starting` (subsumed) | `color.state.starting` |
| `Shutdown` | `Shutdown` | `color.state.shutdown` |
| `ShuttingDown` | `Shutdown` (subsumed) | `color.state.shutdown` |
| `Archived` | `Shutdown` (subsumed) | `color.state.shutdown` |
| `Failed` | `Failed` | `color.state.failed` |
| `Unrecognized` | `Unknown` | `color.state.unknown` |

Subsuming intermediate states under `Starting` / `Shutdown` matches user mental model. The full API state is preserved in `tracing` logs for debugging (per RESEARCH Pitfall 9: state names only, never tokens).

### 6.5 Relative-time formatter (`last_used_at`)

Implemented inline (RESEARCH §"Don't Hand-Roll" — ~25 LOC, no extra crate). Output format:

| Elapsed | Format |
|---------|--------|
| `< 60 s` | `just now` |
| `< 60 min` | `{n} minute(s) ago` |
| `< 24 hr` | `{n} hour(s) ago` |
| `< 7 days` | `{n} day(s) ago` |
| `< 30 days` | `{n} week(s) ago` |
| `< 365 days` | `{n} month(s) ago` |
| `≥ 365 days` | `{n} year(s) ago` |
| `null / missing` | `never` |

Singular vs plural: drop the `s` when `n == 1`. Footer "last refreshed" uses the same formatter against the timestamp of the last successful `GET /user/codespaces`.

### 6.6 Destructive actions

There are **no destructive actions in Phase 6 chrome**. `Sign out` is the closest, and it is an AppKit menu item (no custom button surface). It does not present a confirmation — clicking it deletes the Keychain entries and emits the `signed out` info toast. Rationale: sign-out is reversible (sign in again) and frequent in dev workflows; a confirmation modal would add friction without protecting data (the Keychain entry stays valid on the server side anyway; only its local cache is cleared).

`Cancel sign-in` in the device-flow modal is not destructive — it aborts an in-progress sign-in attempt with no state change.

---

## 7. Motion Contract

**Inherits Phase 5 UI-SPEC §7 verbatim** for all reused components.

### 7.1 Phase 6 additions

| Element | Trigger | Duration | Curve | Reduce-Motion behavior |
|---------|---------|----------|-------|------------------------|
| `AuthDeviceFlowModal` open/close | mount / dismiss | (AppKit default ~150 ms) | (AppKit default) | (AppKit honors automatically) |
| `CodespacesPickerModal` open/close | mount / dismiss | (AppKit default) | (AppKit default) | (AppKit honors automatically) |
| Codespaces row selection expand (44 → 76 px) | row select | 80 ms | ease-out | instant snap |
| Codespaces row selection collapse | row deselect | 80 ms | ease-out | instant snap |
| State badge color cross-fade (e.g. Starting → Available) | poll update | 200 ms | ease-out | instant |
| Starting-state badge spin | continuous | 1.2 s / 360° | linear | static (no rotation) |
| Loading spinner (modal-level) | during fetch | 1.2 s / 360° | linear | static (single frame) |
| Device-flow countdown text update | every 1 s | instant (text re-render) | — | instant |

**Reduce Motion** is read once on modal mount from `NSWorkspace.shared.accessibilityDisplayShouldReduceMotion`. When `true`, every animated motion collapses to its terminal state (no transition).

**Frame budget:** Modals render outside the wgpu compositor (NSPanel is AppKit-native). The state-badge spinner and row-expand animation are inside the picker's wgpu pass (panel uses a wgpu-backed content view) and stay well under the 16.67 ms / 60 fps budget — both are single quads with simple transforms.

---

## 8. Accessibility Contract

**Inherits Phase 5 UI-SPEC §8 verbatim.** All Phase 6 additions are listed in §5 per-component a11y rows.

### 8.1 New WCAG-AA contrast pairs verified

| Foreground | Background | Min ratio achieved | Theme |
|------------|------------|--------------------|-------|
| `font.auth.code` (32 pt fg=theme.fg) | `color.auth.code.bg` (theme.chrome.surface) | 4.5:1+ | Vector Dark |
| `font.auth.code` | `color.auth.code.bg` | 4.5:1+ | Vector Light |
| State label text (`font.chrome.body` muted) | row bg / selected row bg | 4.5:1 | both |
| Action button labels (`font.chrome.button`) | `color.toast.button.bg` | 4.5:1+ | both (inherited from Phase 5 validation) |
| Code-expired warning toast text | toast bg | 4.5:1 | both |

State badge colors (`color.state.*`) are decorative — the state label text adjacent to the badge carries the meaning for low-vision and color-blind users. Color-blind users are explicitly considered: pairing color + glyph (filled disc / open ring / hollow disc / `⊘` / `?`) ensures state distinguishability without relying on hue alone.

### 8.2 Keyboard reachability — Phase 6 surfaces

| Surface | Open shortcut | Internal nav | Close |
|---------|---------------|--------------|-------|
| AuthDeviceFlowModal | menu `Sign in with GitHub` (no global shortcut) | Tab cycles primary/secondary/code-field; Return = primary; Esc = cancel sign-in | titlebar `×`, Esc, or Cancel sign-in button |
| CodespacesPickerModal | menu `Codespaces…` OR `Cmd-Shift-G` | filter input → `↓` enters list → arrows navigate → Tab into row actions → Enter activates primary action | titlebar `×`, Esc, click-outside |
| Sign out | menu `Sign out (@{login})` | n/a | n/a (action item) |
| Connect-stub toast | (auto-emitted) | inherits Phase 5 toast nav | inherits Phase 5 toast nav |

### 8.3 Focus return

| Surface closed | Focus returns to |
|----------------|------------------|
| AuthDeviceFlowModal (any path) | active pane grid in the main NSWindow |
| CodespacesPickerModal | active pane grid |
| AuthDeviceFlow → success → CodespacesPicker chain | filter input of the picker (because user came in via `Cmd-Shift-G` or `Codespaces…` and the auth was a pre-step) |

### 8.4 Token redaction (Pitfall 14 — UI-level)

In addition to the Pitfall-14 code-level discipline (manual `Debug` impls), the UI contract is:

- **Never render the OAuth access token or refresh token as text in any surface.** Modals, toasts, menus, tooltips, status bars, tracing display strings — all blocked.
- The 8-char user-code IS safe to render (it pairs a device, not an account; it expires in 15 min).
- The GitHub `login` (e.g. `@octocat`) IS safe to render (public username).
- Codespace names (`octocat/hello-world-abc123`) ARE safe to render — they are not secrets.
- A grep test in `vector-arch-tests/tests/no_token_in_debug_or_log.rs` enforces this at compile time via fail-on-`token`-in-display-impl pattern (RESEARCH §"Validation Architecture" → arch-lint).

---

## 9. Theme Integration

**Inherits Phase 5 UI-SPEC §9 verbatim.** Phase 6 adds the state-color tokens listed in §4.1 to the `vector-theme` palette (additive, both light and dark variants).

`profile.tint = "#7a3aaf"` is the new default for codespace profiles saved via D-87. The user can edit the TOML to override per-profile. The hex is fixed by D-87, not theme-driven — it identifies "codespace" as a *kind* visually, not a theme choice.

`.itermcolors` overlays continue to override **grid** colors only (Phase 5 §9.2). They do **not** override `color.state.*` — state badges must remain unambiguously colored regardless of user theme.

---

## 10. Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| shadcn official | — (not applicable; Rust + AppKit project) | not required |
| Third-party UI registries | none | not required |

Phase 6 introduces **no third-party UI registry dependencies.** All visual primitives come from:

1. AppKit (Apple, via `objc2-app-kit`) — system-trusted.
2. SF Symbols (Apple) — system-trusted.
3. wgpu compositor passes already in the workspace.
4. Phase 5 component inventory.

External Rust crates that touch UI surface area (none for Phase 6 — `oauth2`, `octocrab`, `keyring-core`, `chrome 0.4` are pure transport/parse/storage).

---

## 11. Component <-> Requirement Cross-Reference

| Requirement | Component(s) | UI tokens |
|-------------|--------------|-----------|
| AUTH-01 (Device Flow sign-in from inside the app) | `AuthDeviceFlowModal`, menu `Sign in with GitHub`, second trigger via Cmd-Shift-P codespace-row click | §5.1, copywriting §6.2, `chrome.auth_modal.*` |
| AUTH-02 (Keychain storage, never disk-plaintext, never logged) | Token redaction contract §8.4; no UI surface renders tokens | none — entirely a non-UI requirement enforced by arch-lint + manual `Debug` |
| AUTH-03 (Silent refresh; expired token triggers re-auth) | Transparent 401 → refresh chain in `CodespacesPickerModal` (§5.2 "401 handling"); on failure, auto-opens `AuthDeviceFlowModal` | Reuses §5.1; copywriting §6.1 line `sign in to github required` |
| CS-01 (List codespaces with state, repo, branch, last-used) | `CodespacesPickerModal` rows | §5.2, copywriting §6.3, §6.4 state mapping, §6.5 time formatter, `color.state.*`, `chrome.cs_picker.*` |
| CS-02 (Start Shutdown codespace; poll; swallow 409) | `CodespacesPickerModal` Start codespace button + row polling state machine | §5.2 "Start button click", copywriting §6.1 `codespace start` lines |
| CS-03 (Save codespace as one-click profile) | `CodespacesPickerModal` "Save as profile" button + §5.3 name-derivation algorithm + Cmd-Shift-P picker integration | §5.2 "Save as profile click", §5.3, copywriting §6.1 `profile saved` lines, §1.4 picker integration |

---

## 12. Layout Boundary Rules

Phase 6 modals are NSPanels — they layer on top of the main NSWindow content rect. Interaction with Phase 5 chrome:

1. **AuthDeviceFlowModal** sits above everything (NSFloatingWindowLevel). Tint stripe, search bar, toasts in the main window remain visible behind it but are not interactive while modal is open.
2. **CodespacesPickerModal** same as above. Cmd-Shift-P profile picker cannot be opened while CodespacesPickerModal is open (mutually exclusive — NSPanel becomes key window).
3. **Toasts** emitted from Phase-6 flows render in the main NSWindow (not in the NSPanel). They are visible behind the modal. When the modal dismisses, the toast remains for its 5 s auto-dismiss. This is intentional — the user reads the toast after dismiss.
4. **Cmd-Shift-P picker** integration: when user clicks a `Kind::Codespace` row that requires auth, the profile picker dismisses first, then AuthDeviceFlowModal opens (chained, not stacked).
5. **Modal-over-modal is disallowed.** If AuthDeviceFlowModal needs to open while CodespacesPickerModal is showing (e.g. 401 chain failed), the picker dismisses first, then the auth modal opens, and on auth success the picker re-opens with the original filter state preserved.

---

## 13. Pre-Populated From Upstream — Source Audit

| Field | Source | Decision id / reference |
|-------|--------|-------------------------|
| Sign-in menu item as primary trigger | CONTEXT.md D-84 | locked |
| Auto-prompt sign-in on codespace profile click | CONTEXT.md D-84 (second trigger path) | locked |
| Device-flow as NSPanel modal, stays on top, auto-copies code, primary button, cancel sign-in | CONTEXT.md D-85 | locked |
| Token never displayed in modal or tooltip | CONTEXT.md D-85 + Pitfall 14 | locked |
| Dedicated Codespaces modal (not extending Cmd-Shift-P) | CONTEXT.md D-86 | locked |
| Cmd-Shift-G keyboard shortcut | CONTEXT.md D-86 (verified Phase 5 keymap has no collision) | locked |
| Modal columns: state badge, repo, branch, last-used | CONTEXT.md D-86 | locked |
| Per-row actions: Connect to codespace / Save as profile / Start codespace | CONTEXT.md D-86 | locked |
| Profile name derivation algorithm | CONTEXT.md D-87 | locked |
| Default tint `#7a3aaf` for codespace profiles | CONTEXT.md D-87 | locked |
| On-demand fetch + active-transition poll (no background) | CONTEXT.md D-88 | locked |
| Refresh icon in picker header | CONTEXT.md D-88 | locked |
| 401 → silent refresh → re-auth prompt flow | CONTEXT.md D-88 + RESEARCH Pattern 2 | locked |
| State color mapping (Available / Starting / Shutdown / Failed) | new this UI-SPEC | researcher choice — semantic system colors, color+glyph for accessibility |
| 16-px state-badge glyphs (disc / ring / hollow / ⊘ / ?) | new this UI-SPEC | researcher choice — color-blind support |
| Subsuming intermediate states into Starting/Shutdown families | new this UI-SPEC | researcher choice — user mental model |
| Spinner motion 1.2 s linear, Reduce Motion = static | new this UI-SPEC | follows Phase 5 motion discipline |
| Modal sizes (440×280 auth; 640×min320..max body-80) | new this UI-SPEC | researcher choice — fits content + 4 px grid |
| Action buttons in expanded-row (44 → 76 px) | new this UI-SPEC | researcher choice — keep unselected rows scan-friendly |
| Voice (lowercase toast, Title Case menu) | inherited from Phase 5 §6 | locked |
| Reused: ToastBanner info mode | Phase 5 §5.4 | locked |
| Reused: ProfilePicker (Cmd-Shift-P) | Phase 5 §5.3 | extended (Codespace kind enabled) |
| Reused: TintStripe with profile tint | Phase 5 §5.1 | locked |
| Reused: typography tokens, color tokens, motion contract | Phase 5 §§3, 4, 7 | locked |
| Relative-time formatter inline (~25 LOC) | RESEARCH §"Don't Hand-Roll" | locked |
| `chrono 0.4` for timestamp parsing | RESEARCH §Standard Stack | locked |
| Profile-name derivation regex `-[a-z0-9]{4,}$` | new this UI-SPEC | researcher choice — derived from D-87 worked examples |

---

## 14. Open Questions for Checker / Auditor

Items the checker should pay particular attention to:

1. **`Phase 6+` suffix is REMOVED from `Kind::Codespace` rows in Cmd-Shift-P** (§1.4). Phase 5 dimmed them; Phase 6 enables them. Auditor must verify the Phase 5 contract has been correctly updated, not duplicated. `Kind::DevTunnel` rows still dimmed with `Phase 8+` suffix (renamed from Phase 5's `Phase 6+`).
2. **Accent reservation list is NOT extended** (§4.1 note). State colors are NOT accent. Codespaces-picker selected row uses `theme.chrome.selection`, not `theme.accent`. If executor adds an accent bar to the codespaces picker row, that's a contract violation.
3. **Token never appears in any UI string** (§8.4). Every Phase 6 string in §6 has been audited; the user-code and `@login` are the only externally-derived strings allowed in chrome. Auditor must verify `tracing` spans + log lines also obey this.
4. **Save-as-profile keeps modal open** (§5.2 "Save as profile click"). This is intentional — users may save multiple. If executor makes it auto-close, that breaks the contract.
5. **No action toasts in Phase 6** (§6.1 final line). Every Phase 6 affordance is in a modal or menu; toasts are observational only. If executor adds an action-mode toast (with buttons) to a Phase 6 flow, escalate.
6. **Modal-over-modal is disallowed** (§12). Chained dismiss-then-open is the correct pattern; if executor stacks NSPanels, Pitfall 3 fires (key-window starvation).
7. **`Connect to codespace` button always emits placeholder toast in Phase 6** (§5.2). Phase 7 will replace the body with real connect logic, but the toast surface + copy lock in here. Phase 6 should NOT implement any SSH/transport logic.

---

*End of UI-SPEC for Phase 6.*
