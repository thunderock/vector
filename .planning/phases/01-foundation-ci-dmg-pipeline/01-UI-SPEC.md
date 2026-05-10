---
phase: 1
slug: foundation-ci-dmg-pipeline
status: approved
shadcn_initialized: false
preset: none
created: 2026-05-10
reviewed_at: 2026-05-10
---

# Phase 1 — UI Design Contract

> Visual and interaction contract for the foundation phase. This is a **native macOS AppKit phase** — no HTML/CSS/web framework, no component library. The contract scope is: window chrome, menu bar, version overlay, app icon, DMG packaging surfaces, and the README + Release body. The standard token taxonomy (spacing scale, typography ladder, 60/30/10 color split) is adapted to those native surfaces; categories that do not apply at this phase (component library, copywriting empty/error states, third-party registries) are explicitly marked **N/A — Phase 1**.

---

## Scope Note

| Surface | In Phase 1? | Notes |
|---------|-------------|-------|
| Native AppKit `NSWindow` chrome | Yes | D-14 locked size + title; chrome style decided here |
| `NSTextField` version overlay | Yes | D-12 locked element; typography + layout decided here |
| Standard macOS menu bar | Yes | D-15 locked structure; copy + accelerators audited here |
| App icon (`.icns`) | Yes | D-16 locked motif; concrete art direction decided here |
| DMG window background + layout | Yes | D-25/D-26 locked content; visual style decided here |
| README install block | Yes | D-26 locked content; layout/typography decided here |
| GitHub Release body template | Yes | D-26 locked content; layout decided here |
| Terminal grid / cell typography | **No** | Phase 3 |
| Tab bar / pane chrome | **No** | Phase 4 |
| Theme system (light/dark, `.itermcolors`) | **No** | Phase 5 |
| Codespaces picker UI | **No** | Phase 6 |
| Reconnect overlay | **No** | Phase 9 |

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none (native AppKit — no shadcn, no Tailwind, no component library) |
| Preset | not applicable |
| Component library | none — `objc2-app-kit 0.6.4` for `NSWindow`, `NSMenu`, `NSTextField` directly |
| Icon library | system SF Symbols where AppKit menus accept them; otherwise none in Phase 1 |
| Font (UI chrome) | macOS system font (`NSFont.systemFont(ofSize:)` / SF Pro) — inherited from AppKit defaults |
| Font (version overlay) | SF Mono (`NSFont.monospacedSystemFont(ofSize: 11, weight: .regular)`) |
| Font (terminal grid) | **deferred to Phase 3** (locked: bundled JetBrains Mono per ROADMAP §"Phase 3 risks") |

Rationale: Anything we don't render ourselves must look identical to a Cocoa-native app. We only depart from system defaults where Phase 1 has a deliberate surface (overlay, icon, DMG, README).

---

## Spacing Scale

Declared values (8-point grid; multiples of 4 only):

| Token | Value | Usage in Phase 1 |
|-------|-------|------------------|
| xs | 4px | Inset between version overlay text and its container edge |
| sm | 8px | Inner padding inside the version overlay rectangle |
| md | 16px | Distance from window edge to overlay; menu separator inset |
| lg | 24px | DMG icon-to-Applications-arrow horizontal gap (visual breathing room) |
| xl | 32px | DMG margin between window edge and the `Vector.app` / `Applications` icons |
| 2xl | 48px | DMG vertical space reserved for the `xattr` instruction strip |
| 3xl | 64px | README major section breaks (between H2 sections in markdown render) |

Exceptions:
- macOS standard menu accelerators are not "spacing" — they follow AppKit defaults.
- `NSWindow` traffic-light buttons are AppKit-positioned; we do not specify their inset.
- DMG window dimensions use the `create-dmg` natural defaults (640×400) but icon positions are quantized to the table above.

---

## Typography

### UI / chrome typography (the only typography Phase 1 emits)

| Role | Size | Weight | Line Height | Where |
|------|------|--------|-------------|-------|
| Version overlay (mono) | 11pt | Regular (400) | 1.2 | `NSTextField` over the `NSWindow` content view |
| Menu bar items | system default | system default | system default | `NSMenu` / `NSMenuItem` (do not override) |
| Window title | system default | system default | n/a (single line) | `NSWindow.title` |
| DMG title strip (rasterized into background PNG) | 14pt | Semibold (600) | 1.3 | "Drag Vector.app to Applications" headline |
| DMG instruction strip (rasterized into background PNG) | 11pt | Regular (400) | 1.4 | The `xattr` command + one-line explainer |
| README H1 (`# Vector`) | rendered by GitHub markdown | n/a | n/a | governed by GitHub's stylesheet, not us |
| README code block (the `xattr` line) | rendered by GitHub markdown | n/a | n/a | governed by GitHub's stylesheet |

Rules:
- **No more than 3 type sizes ship in Phase 1** (11pt, 14pt — overlay + DMG headline; everything else is system default that we do not override). The 4th "Display" slot in the standard template is intentionally unused.
- **No more than 2 weights ship in Phase 1** (Regular 400, Semibold 600). No Bold, no Light, no Black.
- The version overlay uses mono so the SHA portion looks dependable / not like marketing copy.
- The DMG background uses the macOS system font (SF Pro) rasterized at the asset-creation stage — we do not embed a font file.

---

## Color

Vector v1 has no terminal palette in Phase 1 (themes land in Phase 5). The 60/30/10 split below covers only the surfaces this phase ships.

| Role | Value | Usage |
|------|-------|-------|
| Dominant (60%) | `#1A1A1A` (near-black, not pure black) | `NSWindow` content view background; DMG background base color |
| Secondary (30%) | `#2A2A2A` | Version overlay rectangle fill (slightly lighter than the window content view, gives the overlay a subtle plate so the text reads on the dark canvas) |
| Accent (10%) | `#7B61FF` (electric violet) | Icon vector/tensor motif primary stroke; DMG drag-arrow stroke; nothing else |
| Destructive | not applicable in Phase 1 | no destructive UI lands until Phase 7 (disconnect/quit-with-active-session) |

**Accent reserved for:** the app icon's vector/tensor strokes, and the DMG drag-to-Applications arrow. **Nothing else.** The version overlay uses neutral text on neutral plate. The menu bar uses system defaults. There is no "primary button" in Phase 1.

Rationale for `#1A1A1A` over `#000000`: pure black on macOS reads as "uninitialized framebuffer" and triggers the "is this app actually running" reflex from teammates who land on the unsigned build. A near-black surface signals "this is intentional and styled."

Rationale for the violet accent: it sits in the same neighborhood as the GitHub Codespaces brand purple (which we will adopt as a tab tint in Phase 7 per CS-06), establishing a visual through-line from "the app launches" → "you're in a Codespace." A v1 palette decision now saves a re-skin later. Override at planning time if the user prefers a different accent within the speed/vector/tensor motif (acceptable alternatives: cyan `#00D4FF`, electric green `#39FF14`).

### Window chrome decisions (native AppKit)

| Property | Value | Rationale |
|----------|-------|-----------|
| `NSWindow.titlebarAppearsTransparent` | `false` | Standard titlebar — Phase 1 is not the place for unified-titlebar tricks; Phase 3+ may revisit. |
| `NSWindow.styleMask` | `.titled \| .closable \| .miniaturizable \| .resizable` | Standard window — D-15 menu bar's `Window → Minimize/Zoom/Close` items expect these capabilities. |
| Traffic-light buttons | default position (top-left) | Native macOS feel; do not hide or relocate. |
| `NSVisualEffectView` / vibrancy | **not used in Phase 1** | Vibrancy looks great on a real terminal background but is wasted on a flat overlay. Defer to Phase 3 when a real surface exists. |
| Window background color | `#1A1A1A` set via `NSWindow.backgroundColor` | Matches the dominant-60% surface. |
| Tabbing mode | `.disallowed` | Phase 1 has no tabs; Phase 4 will switch to `.preferred`. Locking it down explicitly prevents AppKit from auto-grouping windows during dev. |
| Initial size | 1024 × 640 (D-14) | Locked. |
| Window position | centered (`NSWindow.center()`) (D-14) | Locked. |

---

## Copywriting Contract

The standard CTA / empty-state / error-state taxonomy doesn't fit Phase 1 (no buttons, no data, no error UI). The copy that DOES ship is enumerated below — every string the user can see in Phase 1.

### App-shell copy

| Element | Copy | Source |
|---------|------|--------|
| Initial window title (pre-tick) | `Vector` | D-14 locked |
| Window title after first tick | `Vector — tick {n}` | D-10 locked |
| Version overlay text | `Vector v{version} (build {sha})` — example: `Vector v2026.05.10 (build a1b2c3d)` | D-12 locked, format confirmed here |
| Menu bar app name | `Vector` | D-15 locked |
| Menu bar top-level items (left → right) | `Vector` / `File` / `Edit` / `View` / `Window` / `Help` | D-15 locked |

### Menu bar items (Phase 1)

Every menu and item is enumerated. Items marked **disabled** render greyed out and produce no action. Items marked **functional** wire to AppKit defaults or to Phase 1 code paths.

#### `Vector` menu

| Item | Accelerator | State | Action |
|------|-------------|-------|--------|
| About Vector | — | functional | Standard `NSApp.orderFrontStandardAboutPanel(_:)` — uses Info.plist values; no custom panel. |
| (separator) | — | — | — |
| Preferences… | Cmd-, | **disabled** in Phase 1 | Stub for Phase 5 (config). |
| (separator) | — | — | — |
| Services | — | functional (system) | Standard AppKit services submenu. |
| (separator) | — | — | — |
| Hide Vector | Cmd-H | functional | `NSApp.hide(_:)`. |
| Hide Others | Cmd-Option-H | functional | `NSApp.hideOtherApplications(_:)`. |
| Show All | — | functional | `NSApp.unhideAllApplications(_:)`. |
| (separator) | — | — | — |
| Quit Vector | Cmd-Q | functional | `NSApp.terminate(_:)` (D-15 locked). |

#### `File` menu

| Item | Accelerator | State | Action |
|------|-------------|-------|--------|
| New Window | Cmd-N | **disabled** in Phase 1 | Stub for Phase 4. |
| New Tab | Cmd-T | **disabled** in Phase 1 | Stub for Phase 4. |
| (separator) | — | — | — |
| Close | Cmd-W | functional | `NSWindow.performClose(_:)` (D-15 locked). |

#### `Edit` menu

| Item | Accelerator | State | Action |
|------|-------------|-------|--------|
| Undo | Cmd-Z | **disabled** in Phase 1 | First responder has no editable surface. |
| Redo | Cmd-Shift-Z | **disabled** in Phase 1 | — |
| (separator) | — | — | — |
| Cut | Cmd-X | **disabled** in Phase 1 | — |
| Copy | Cmd-C | **disabled** in Phase 1 | Phase 3+ wires this to selection. |
| Paste | Cmd-V | **disabled** in Phase 1 | Phase 2 wires this to PTY input. |
| Select All | Cmd-A | **disabled** in Phase 1 | — |

#### `View` menu

| Item | Accelerator | State | Action |
|------|-------------|-------|--------|
| Enter Full Screen | Cmd-Ctrl-F | functional | `NSWindow.toggleFullScreen(_:)`. |

#### `Window` menu

| Item | Accelerator | State | Action |
|------|-------------|-------|--------|
| Minimize | Cmd-M | functional | `NSWindow.performMiniaturize(_:)` (D-15 locked). |
| Zoom | — | functional | `NSWindow.performZoom(_:)` (D-15 locked). |
| (separator) | — | — | — |
| Bring All to Front | — | functional | `NSApp.arrangeInFront(_:)`. |

#### `Help` menu

| Item | Accelerator | State | Action |
|------|-------------|-------|--------|
| Vector Help | — | **disabled** in Phase 1 | Stub for documentation site (post-v1). |

**Naming rules:**
- Sentence case, not Title Case (matches macOS HIG).
- No emoji in menu items.
- Disabled items still render — they are not hidden — so the structure is visible to teammates from day one.

### Version overlay placement

| Property | Value |
|----------|-------|
| Anchor | bottom-right of window content view |
| Margin from right edge | 16px (md) |
| Margin from bottom edge | 16px (md) |
| Inner padding | 8px horizontal × 4px vertical (sm × xs) |
| Background | `#2A2A2A` rectangle, 4px corner radius |
| Text color | `#9A9A9A` (subdued grey, not full white — overlay is informational, not chrome) |
| Drag region | overlay must NOT be a drag-region; clicks on it should pass through to the window content |
| Pointer cursor | default arrow — overlay is non-interactive in Phase 1 |

### DMG background image content

Surface dimensions: 640 × 400 (`create-dmg` default window size).

Vertical layout (top → bottom):
1. **Top strip (0 – 80px):** Vector wordmark + app icon thumbnail at 32×32, left-anchored at 32px (xl) inset. Wordmark in 14pt Semibold (`#FFFFFF`).
2. **Middle band (80 – 280px):** Two icon slots positioned by `create-dmg` (`Vector.app` on the left at ~160 px center, `Applications` symlink on the right at ~480 px center). Drag arrow rasterized into background between them, stroke `#7B61FF`.
3. **Bottom strip (280 – 400px):** Two-line instruction block.
   - Line 1 (14pt Semibold, `#FFFFFF`): `If macOS blocks the app, run this in Terminal:`
   - Line 2 (11pt Regular Mono, `#7B61FF` background plate at 12% alpha, `#FFFFFF` text, monospace): `xattr -dr com.apple.quarantine /Applications/Vector.app`

Asset path (placeholder until rasterized): `crates/vector-app/resources/dmg-background.png` (1280 × 800 @2x for Retina).

### README install block

The README is a markdown surface. It must contain — in this order, before any other content beyond the H1:

````markdown
# Vector

Fast native macOS terminal with first-class GitHub Codespaces and Dev Tunnels support.

## Install

1. Download the latest `Vector-{version}-universal.dmg` from [GitHub Releases](https://github.com/<owner>/vector/releases/latest).
2. Open the DMG and drag `Vector.app` to `/Applications`.
3. The first time you launch, macOS will block the unsigned app. Run this once in Terminal:

```sh
xattr -dr com.apple.quarantine /Applications/Vector.app
```

4. Open Vector from `/Applications` (or Launchpad). You should see a window titled `Vector` with a small build identifier in the bottom-right corner.
````

Rules:
- The `xattr` line is in a fenced code block, language hint `sh`, on its own line. Teammates will copy-paste verbatim.
- No badges (build status, license, etc.) above the H1 in Phase 1. Badges add noise and we have nothing material to badge yet. Revisit when CI is green and stable for one full week.
- The phrase "unsigned app" appears explicitly so the `xattr` step does not feel arbitrary.

### GitHub Release body template

Every tagged release uses this body (auto-generated by `cargo xtask release`, populated from `git-cliff`):

````markdown
## Vector {version}

[CHANGELOG section for this version, generated by git-cliff]

### Install

Download `Vector-{version}-universal.dmg` from this release page, open it, and drag `Vector.app` to `/Applications`.

The first time you launch, macOS will block the unsigned app. Run this once:

```sh
xattr -dr com.apple.quarantine /Applications/Vector.app
```

Built from commit `{sha}` on macOS 13+ (Ventura). Apple Silicon and Intel both supported (Universal binary).
````

The same `xattr` line, byte-identical, appears in three places (D-26): README, DMG background, Release body.

### Standard template fields (N/A justification)

| Element (template default) | Phase 1 status | Reason |
|---------|------|--------|
| Primary CTA | **N/A — Phase 1** | No buttons in Phase 1. The first CTA-shaped element ships in Phase 6 ("Sign in with GitHub"). |
| Empty state heading | **N/A — Phase 1** | No data surfaces in Phase 1. The first empty state ships in Phase 6 (empty Codespaces list). |
| Empty state body | **N/A — Phase 1** | — |
| Error state | **N/A — Phase 1** | No error-producing user flows in Phase 1. The first error UI ships in Phase 6 (auth failure) and Phase 7 (connect failure). |
| Destructive confirmation | **N/A — Phase 1** | No destructive actions in Phase 1. Cmd-Q quits without confirmation by design (no in-flight state to lose). |

---

## App Icon Direction

D-16 locks the **motif** (speed + vector/tensor) and the **mechanism** (`crates/vector-app/resources/icon.svg` → `xtask` → `iconutil` → `.icns`). This contract bounds the **art direction** so the planner / executor can produce a placeholder without a second design round.

### Required properties

| Property | Value |
|----------|-------|
| Canvas | macOS app-icon canvas: 1024×1024 master, with iconutil-generated set covering 16, 32, 64, 128, 256, 512, 1024 (each at @1x and @2x where applicable). |
| Shape language | Geometric. A directional element (arrow / vector head / motion line) over a flat or subtly-gradient base. No skeuomorphism, no glyph-on-document, no glossy reflection. |
| Primary stroke / fill | Accent `#7B61FF` |
| Secondary stroke / fill | White `#FFFFFF` at 90% alpha |
| Background | Rounded-square plate, dominant `#1A1A1A`. Plate corner radius follows macOS Big Sur+ icon proportions (≈22.4% of canvas — `iconutil` does not enforce this; the SVG must respect it manually). |
| Composition | Single focal element, centered. The vector/tensor motif reads at 16×16 (i.e. survives Finder list-view rendering) — no fine detail that disappears below 32px. |
| Negative space | Minimum 8% inset from the rounded-square plate edge to any vector/tensor stroke. macOS draws shadow + light effects assuming this margin. |
| Text in icon | None. No "V", no wordmark, no version. The icon is a glyph-shape, not a logo. |

### Concrete first-cut direction (placeholder; replaceable in one commit per D-16)

A right-leaning chevron (`>`) made of three stacked motion lines, each line a tighter copy of the previous, suggesting both an arrow head and a tensor's directional eigenvector. The longest line spans the full inner width; the next two are 75% and 50%. All three start from the same left x-coordinate; their right tips converge at a single point on the right side. Stroke `#7B61FF`, stroke-width 64 on the 1024 canvas (scales down cleanly to 16×16 as a single anti-aliased pixel). Plate `#1A1A1A` rounded square.

The planner may substitute any other concrete shape that satisfies the **Required properties** table above without re-running this contract.

### Out of scope for Phase 1

- Adaptive light/dark icon variants (macOS 14 supports them; Phase 1 ships a single icon).
- Document-type icons (Vector has no document types in v1).
- Menu bar status icon (Vector is not a menu-bar app).

---

## Threading-visible surface (D-10 smoke test)

The tick smoke test is a UI-visible surface: it mutates the window title every 500ms. The contract for what the user sees:

| Property | Value |
|----------|-------|
| Pre-tick title | `Vector` |
| Title after first tick | `Vector — tick 1` |
| Title pattern | `Vector — tick {n}` where `{n}` is a monotonically incrementing decimal integer (no leading zeros, no padding). |
| Tick interval | 500ms (D-10 locked) |
| Em-dash | U+2014 EM DASH (not hyphen `-`, not en-dash `–`) — matches macOS title conventions. |
| Visibility | The tick is visible in the window title bar AND in the macOS Dock right-click menu's window list. Both update together because `NSWindow.title` is the source of truth. |

Phase-2-and-later note: when the terminal grid lands, the tick test is removed (it stops being a useful proof of life once real PTY output is rendering). The title format will revert to a Phase-3-decided convention. Phase 1 ships only the format above.

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| shadcn official | none — N/A (no web framework in Phase 1) | not applicable |
| third-party (any) | none | not applicable |

No component registries, no third-party UI blocks, no `shadcn add` invocations land in Phase 1. The only third-party visual asset that enters the repo is the placeholder app icon SVG (authored in-repo, not pulled from a registry).

If a future phase pulls a third-party design asset (e.g. a vector icon set for Phase 6's Codespaces picker), that phase's UI-SPEC must run the registry vetting gate — Phase 1 does not pre-approve anything for downstream phases.

---

## Pre-populated Sources

| Source | Decisions Used | Examples |
|--------|----------------|----------|
| `01-CONTEXT.md` | 8 | D-10 (tick format), D-12 (overlay element), D-13 (build SHA source), D-14 (window size + title), D-15 (menu bar structure), D-16 (icon mechanism), D-25 (DMG tooling), D-26 (xattr in three places) |
| `STACK.md` | 3 | `objc2-app-kit 0.6.4` (menu/window APIs), `winit 0.30.13` (event loop), system font defaults |
| `ARCHITECTURE.md` | 1 | `vector-app` crate owns the `NSWindow` and menu bar; `vector-ui` crate exists but is empty in Phase 1 |
| `PROJECT.md` / `CLAUDE.md` | 2 | Native macOS feel (no Electron), unsigned distribution requires `xattr` instruction surface |
| `REQUIREMENTS.md` | 6 | BUILD-01..05 (DMG / install / xattr surfaces), WIN-05 (threading smoke test is the visible signal) |
| Researcher discretion | 7 | Color palette (#1A1A1A / #2A2A2A / #7B61FF / #9A9A9A), version-overlay typography, icon concrete shape, DMG background layout, README install block format, menu item disabled/functional matrix, window-chrome property table |

---

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS
- [ ] Dimension 2 Visuals: PASS
- [ ] Dimension 3 Color: PASS
- [ ] Dimension 4 Typography: PASS
- [ ] Dimension 5 Spacing: PASS
- [ ] Dimension 6 Registry Safety: PASS

**Approval:** pending
