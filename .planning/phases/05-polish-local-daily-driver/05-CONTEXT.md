# Phase 5: Polish (Local Daily-Driver) - Context

**Gathered:** 2026-05-12
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 5 makes Vector the user's **daily-driver local terminal**. Scope is bounded by POLISH-01..08 + the deferred D-65 (`Cmd-N` new window) + the folded `code-quality-hardening` todo + the Phase-3 deferral D-53/D-54 (`Cmd-C` copy + selection-string extraction).

In-scope capabilities:
- TOML config + hot-reload + profile inheritance (POLISH-01)
- Custom fonts + opt-in ligatures + Nerd-Font glyphs (POLISH-02)
- Built-in light/dark themes + `.itermcolors` import (POLISH-03)
- OSC 7 (cwd), OSC 8 (hyperlinks), OSC 10/11/12 (color queries), OSC 133 (semantic prompts) (POLISH-04)
- OSC 52 clipboard, both raw + DCS-wrapped (POLISH-05)
- Scrollback regex search with UI (POLISH-06) — backed by `vector-term::search()` library API from D-39
- Profiles (POLISH-07) — `kind = { local, codespace, dev_tunnel }` schema lands; only `local` transport wired in Phase 5
- Secure Keyboard Entry toggle + basic IME preedit (POLISH-08)
- `Cmd-N` new window (deferred from D-65)
- Selection-string extraction + `Cmd-C` copy (deferred from D-53/D-54)
- Code-quality hardening: workspace lints, path-dep version arch-lint, `cargo deny` in pre-commit, `cargo-machete`

Explicit non-goals (re-checked Phase-5 risks):
- No Lua / no plugin system / no general extensible command palette (the Cmd-Shift-P profile picker is a *narrow* exception — see D-75)
- No full IME with candidate window (Pitfall 16 — strictly v2)
- No transport for `codespace` / `dev_tunnel` profile kinds (Phase 6 / Phase 7)
- No "new tab from picker" / multi-profile picker UI beyond the minimal Cmd-Shift-P
- No `OSC 9;4` progress, no kitty graphics, no sixel

</domain>

<decisions>
## Implementation Decisions

### Config + hot-reload model (POLISH-01)

- **D-68:** **Single `~/.config/vector/config.toml`; `[default]` + `[profile.<name>]` overlay inheritance.** `[profile.X]` overrides only the keys it specifies; nested tables do *not* deep-merge (a profile-level `[profile.work.font]` table replaces the whole `[default.font]` table). Matches ghostty's single-file approach. `deny_unknown_fields` per Pitfall 11. Schema validation via `serde` + a follow-up `Result<Config, ConfigError>` that surfaces the first error line + column.

- **D-69:** **Live-apply theme, keybinds, font-size, ligatures-toggle, tint, and per-profile params. Font-family change and GPU-shaped keys show a `restart required` toast. On parse error: keep last-good config in memory + emit a non-blocking toast with the first error.** Debounce FSEvents at 150 ms quiescent. The toast surface is reused for D-70's clipboard prompt UI.

### Clipboard + tmux passthrough (POLISH-05)

- **D-70:** **OSC 52 clipboard writes are prompt-once-per-origin then remembered per-profile.** First write from a session shows a non-modal toast: `Allow [profile-name : foreground-process] to write to your clipboard? [Allow once] [Always] [Block]`. The Always/Block choice is persisted into the active `[profile.X]` block as `clipboard_write = "allow" | "block"`. Reads via OSC 52 query are *always denied* in v1 (iTerm2 default — Pitfall security row CVE-class).

- **D-71:** **Accept both raw OSC 52 and DCS-wrapped `\eP\e]52;c;…\a\e\\` inbound. Vector never re-wraps outbound — that's tmux's job.** Payloads emitted by Vector chunk at 58 bytes to dodge the ~60-char tmux passthrough bug. README documents `set -g allow-passthrough on` and we ship a shell-integration script (Phase 6+) that sets it automatically inside a Codespace.

### Theme strategy + `.itermcolors` (POLISH-02 / POLISH-03)

- **D-72:** **Two bundled themes: Vector Light + Vector Dark. `[default].appearance` accepts `"system" | "light" | "dark"`, default `"system"` (follow macOS via `NSApplication.effectiveAppearance` + `NSApp` KVO).** No Solarized / Tomorrow / Gruvbox bundled — users bring those via `.itermcolors` (D-73).

- **D-73:** **`.itermcolors` import = drop file in `~/.config/vector/themes/`, reference by stem in config.** `theme = "Solarized-Dark"` resolves to `themes/Solarized-Dark.itermcolors`. The themes dir is watched; new files become available after save, no app restart. Importer is a plist parser mapping the iTerm key set (`Ansi 0..15 Color`, `Foreground Color`, `Background Color`, `Cursor Color`, `Selection Color`, `Bold Color`) into Vector's palette struct. Unknown keys are warned + ignored. No CLI/GUI import command in v1.

### Scrollback search UX (POLISH-06)

- **D-76:** **Inline bottom search bar, per-pane. `Cmd-F` opens, `Esc` closes + restores prior selection.** Bar renders inside the pane's viewport (under the grid, above the divider), height ~32 px. Layout: `[/{query}/▢] [aA] [↑] [↓] [{i}/{n}] [×]` — but per D-77 the case/regex toggles are *not* visible affordances; only the query field, position counter, prev/next arrows, and close button are rendered. Built on Phase-3's compositor: search bar is its own viewport rect with a tinted background; no new pipeline.

- **D-77:** **Smart-case + always-regex; Enter = next, Shift-Enter = prev.** Smart-case = case-insensitive when query is all-lowercase, case-sensitive when query has any uppercase (rg / vim convention). The query is *always* compiled as a regex via `vector-term::search()` D-39; non-regex chars work because they parse as literal patterns. All matches highlighted with a translucent yellow box per cell (theme-aware: yellow on dark, orange on light); active match boldened with a 1 px border. Up to 1000 matches cached; beyond that, show `1000+ matches` and step lazily.

### Profile model + switcher UX (POLISH-07)

- **D-74:** **`Profile { kind: Kind, name: String, … }` with `Kind = { Local, Codespace, DevTunnel }` enum and unlimited named profiles per kind.** TOML shape:
  ```toml
  [profile.work-cs]
  kind = "codespace"
  codespace_name = "octocat/hello-world-abc123"
  theme = "Solarized-Dark"
  tint = "#7a3aaf"
  env = { FOO = "bar" }
  startup_command = "tmux new -A -s vector"
  ```
  Phase 5 only wires `kind = "local"` end-to-end (`SpawnCommand` via `LocalDomain`). `codespace` / `dev_tunnel` profiles parse, persist, and appear in the switcher with a `"⚠ Phase 6+"` label; connecting them no-ops with a toast. The `Profile` struct is the long-term type — Phases 6/7 fill in the transport, never reshape the schema.

- **D-75:** **Title-bar tint stripe + minimal Cmd-Shift-P profile picker.**
  - **Tint:** the per-profile `tint = "#RRGGBB"` paints a 24–32 px stripe under the NSWindow title bar (above the tab bar). Reuses an existing pipeline (researcher's call: extend the per-cell tint uniform from Phase 3 or paint via `NSVisualEffectView` overlay). Default profile has no stripe.
  - **Switcher:** `Cmd-Shift-P` opens a *narrow* modal listing profile names with fuzzy match. Enter swaps the active pane's profile (re-spawns its `Domain`). No general action listing, no extensibility, no Lua surface — this is **not** a general command palette. The roadmap line "no command palette" (Phase 5 risks/notes) is honored in spirit: we are explicitly carving out profile-switching only.
  - Menu fallback: `Vector → Switch Profile →` submenu mirrors the picker for users who don't know the shortcut.

### OSC 7 / OSC 8 / OSC 133 visible behaviors (POLISH-04)

- **D-78:** **OSC 8 hyperlinks render plain by default; mouse hover shows a dotted underline + Cmd-cursor; `Cmd-click` opens via `NSWorkspace.openURL`.** Schemes allowlisted: `https`, `http`, `mailto`, `file://`. Everything else is logged at `info` and ignored (CVE-class per Pitfalls security row — malicious log content could shellward `gopher://` etc.). URLs longer than 4096 chars are truncated + warned. Hyperlink ID (`id=`) is tracked so multi-cell ranges underline together.

- **D-79:** **OSC 7 cwd is preferred over `proc_pidinfo` (D-63) for new-pane / new-tab cwd inheritance when present.** Each pane stores `cwd: Option<PathBuf>`; updated whenever the shell emits `OSC 7;file://host/path/\a`. New-pane spawn uses `pane.cwd.or_else(|| proc_pidinfo_fallback(pane.pid)).unwrap_or(home)`. The tab title (D-57) gains a cwd-stem suffix when OSC 7 is present: `zsh: vector` instead of just `zsh`. **OSC 133** prompt marks (`OSC 133;A/B/C/D`) are captured into a `Vec<PromptMark>` per pane (start, command, output, end with exit-code). UI for prompt-mark navigation (`Cmd-PageUp` jump-to-prev-prompt) is **stubbed but not wired** — the data captures now so Phase 6+ shell-integration scripts produce something useful.

- **Claude's Discretion:** **OSC 10/11/12** fg/bg/cursor color query responses are pure VT parser responses; vim/neovim need them to detect dark mode. Implementation is mechanical — respond with the current theme colors in xterm format.

### Secure Keyboard Entry + IME (POLISH-08)

- **D-80:** **Secure Keyboard Entry is a single global app-state toggle, exposed only via `Vector → Secure Keyboard Entry` menu item with a checkmark.** Persisted as `[default].secure_keyboard_entry = true | false`. Implementation calls `EnableSecureEventInput()` / `DisableSecureEventInput()` on the whole process — Apple's API is process-level, not per-window, so "per-window SKE" is illusory and we don't pretend. No keyboard shortcut (`Cmd-Shift-K` is too easy to fat-finger and would surprise users).

- **D-81:** **Basic IME = display marked text under the cursor only, no candidate window.** Implement `NSTextInputClient` `setMarkedText:selectedRange:replacementRange:` and `insertText:replacementRange:`. Preedit text renders inline at the active cell, underlined (using the existing cell pipeline's underline attribute). Commit on Enter, cancel on Escape. macOS dead-keys (`Option-e + e → é`) work out of the box. CJK users see the preedit but no candidate selector — they'll need to use the system input source's candidate window if it can position itself; we don't coordinate placement. **Full IME with candidate window is strictly v2 per Pitfall 16.**

### Cmd-N + code-quality hardening

- **D-82:** **`Cmd-N` spawns a fresh local profile in a new ungrouped `NSWindow`.** Always uses the `[default]` profile and `$HOME` cwd (no inheritance from focused window). Predictable: `Cmd-N` = "clean slate"; `Cmd-T` = "duplicate context as new tab"; the Cmd-Shift-P picker = "switch profile". Matches Apple Terminal.

- **D-83:** **Code-quality hardening — all four sub-items from the folded todo land in Phase 5:**
  1. **Workspace `[lints]` inheritance:** add a top-level `[workspace.lints]` block in `Cargo.toml` (`rust.unsafe_code = "forbid"` except an allowlist, `clippy.pedantic = "warn"`, `clippy.await_holding_lock = "deny"` per D-11); every crate's `Cargo.toml` adds `[lints] workspace = true`. Existing per-crate inline lint allowlist for `vector-app` (AppKit `unsafe_code`) is preserved.
  2. **Path-dep version arch-lint:** extend each `tests/no_tokio_main.rs` (or factor into one workspace-level integration test) to parse the crate's `Cargo.toml` via `toml` and assert every `dependencies.*` with `path = "..."` *also* has `version = "..."`. Failing message names the offending line. Closes the cargo-deny `bans FAILED` regression class from `d652c8b`.
  3. **`cargo deny check` in pre-commit:** add to `.pre-commit-config.yaml` (`cargo deny check bans licenses sources advisories`), `pass_filenames: false`. Stages: `pre-commit`. Catches the failure locally before push.
  4. **`cargo-machete` in CI:** runs on every PR, fails on unused workspace deps. Dev-time signal we're not dragging in libs we don't use.

### Selection-string extraction + Cmd-C (carries from D-53/D-54)

- **Claude's Discretion:** **`Cmd-C` copies the selection-range string to `NSPasteboard.general`.** Walk Phase-3's `SelectionRange` (D-54) over the grid, join cells with newline boundaries, strip trailing whitespace per line, drop the OSC-52 path entirely (clipboard goes through native pasteboard for `Cmd-C`, not through the wire). Handles wide chars + zero-width via `unicode-width`. Smart line endings: rectangular selections use `\n`; stream selections preserve grid newlines.

### Profile scope (binding to D-74)

- **Claude's Discretion:** **Profile is per-pane state.** Each pane owns a `Domain` (per D-38); `profile_name: String` lives on the pane. Switching profile via Cmd-Shift-P respawns the active pane's `Domain` with the new profile's `SpawnCommand`. New windows/tabs/panes inherit the spawning context's profile (a new tab in a Codespace window stays in that Codespace profile once Phase 6 transports it). `Cmd-N` is the explicit exception (always `[default]` profile per D-82).

### Claude's Discretion

The following are downstream-agent calls — researcher/planner pick the best approach without re-asking the user:

- **Keybind override TOML syntax** — propose `[[keybind]]` array of `{ key = "cmd-shift-r", action = "reload-config" }` entries with a sealed `Action` enum and conflict detection at config-load time.
- **Font fallback chain** — CoreText system default; emoji via Apple Color Emoji; CJK via system fonts. JetBrains Mono bundle (D-41) stays the default for `[font].family`.
- **`vector-config` / `vector-theme` / `vector-secrets` crate boundaries** — Phase 1 stubbed all three. `vector-config` owns the schema + loader + watcher; `vector-theme` owns the palette struct + `.itermcolors` parser + appearance follow logic; `vector-secrets` owns Keychain plumbing via `keyring 4.0` (initialized here for Phase 6's OAuth token caching — Phase 5 may not yet write anything to Keychain, just lock the API surface).
- **`notify` debounce strategy** — debounce at 150 ms quiescent on the config file and themes dir. Multi-write atomic-rename editors (vim, nvim) replace the inode; watcher must re-arm on parent dir.
- **Toast surface** — a thin top-of-window banner inside the active NSWindow that fades in/out at fixed durations (≤ 5s for informational, until-dismissed for clipboard prompts). Implementation reuses the Phase-3 compositor (drawn as a separate pass over the active pane) — no AppKit toast framework dependency.
- **Profile picker fuzzy match** — `fuzzy-matcher` crate (smith-waterman) over profile names. Up to 500 profiles considered (~impossible in practice).
- **OSC 8 hyperlink span detection during hover** — hit-test the mouse cell, look up the hyperlink ID from the cell's attribute set, find the contiguous run of cells sharing that ID, underline all of them.
- **Search highlight color choice** — yellow background on dark themes, orange on light, alpha ~0.4; final color planner's call. Reuse the per-cell tint uniform (no new pipeline).
- **OSC 133 mark struct** — `PromptMark { kind: A|B|C|D, row: usize, exit_code: Option<i32>, time: Instant }`. Bounded ring (most recent 1000 prompts per pane).
- **SKE menu item position** — under `Vector` menu, between `About Vector` and the separator above `Quit`.

### Folded Todos

- **`code-quality-hardening`** (`.planning/todos/pending/2026-05-11-code-quality-hardening-workspace-lints-arch-lint-upgrade-pre-commit-cargo-deny.md`) — folded into D-83. All four sub-items land in Phase 5; closes the cargo-deny `bans FAILED` regression class.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Roadmap + requirements
- `.planning/ROADMAP.md` §"Phase 5: Polish (Local Daily-Driver)" — goal, depends-on, requirements list, success criteria, stack additions, risks
- `.planning/REQUIREMENTS.md` POLISH-01 through POLISH-08 — checkbox requirements + traceability table

### Research / pitfalls (load-bearing)
- `.planning/research/PITFALLS.md` §Pitfall 8 — Tmux DCS passthrough, ~60-char truncation, `allow-passthrough on` (informs D-71)
- `.planning/research/PITFALLS.md` §Pitfall 11 — Configuration sprawl; "single TOML file, `deny_unknown_fields`, no DSL" (informs D-68)
- `.planning/research/PITFALLS.md` §Pitfall 16 — IME defer to v2 unconditionally (informs D-81)
- `.planning/research/PITFALLS.md` §Security (table row "Using user-content as escape sequences without sanitization") — OSC 8 https/mailto/file:// allowlist, OSC 52 opt-in (informs D-70, D-78)
- `.planning/research/FEATURES.md` lines 27–32 — OSC 7/8/10/11/12/133/52 capability matrix
- `.planning/research/FEATURES.md` line 66 — Hot-reload via `notify` crate, debounce, SIGHUP/USR2 (informs D-69)
- `.planning/research/FEATURES.md` line 68 — Command palette is a deliberate distinct concept; explicitly NOT in Phase 5 except the D-75 narrow profile-picker exception
- `.planning/research/FEATURES.md` line 80 — `.itermcolors` importer pattern (informs D-73)
- `.planning/research/FEATURES.md` line 83 — Ligatures opt-in via HarfBuzz shaping (informs D-42 / Phase 5 finish)

### Architecture decisions (prior phases — load-bearing)
- `.planning/phases/01-foundation-ci-dmg-pipeline/01-CONTEXT.md` D-06 (workspace lints), D-32 (tracing), D-33 (ADR practice) — Phase 1 baselines extended by D-83
- `.planning/phases/02-headless-terminal-core/02-CONTEXT.md` D-38 (`PtyTransport`/`Domain` final), D-39 (`vector-term::search()` API) — POLISH-06 wires UX onto this existing API
- `.planning/phases/03-gpu-renderer-first-paint/03-CONTEXT.md` D-41 (JetBrains Mono bundle), D-42 (ligatures opt-in), D-50 (CoreText grayscale AA), D-52 (xterm keymap), D-53 (Cmd-V bracketed paste; Cmd-C deferred to Phase 5), D-54 (selection rectangle; string extraction deferred to Phase 5)
- `.planning/phases/04-mux-tabs-splits/04-CONTEXT.md` D-57 (tab title = foreground process), D-63 (cwd via proc_pidinfo — D-79 swaps to OSC 7), D-65 (Cmd-N deferred to Phase 5 — closed by D-82), D-66 (active-pane border pipeline — reusable for toast / tint stripe)

### Project + crate context
- `.planning/PROJECT.md` §Out of Scope — confirms no Lua / no plugins / no general command palette (D-75 carves a narrow exception with explicit reasoning)
- `crates/vector-term/src/search.rs` (already exists, D-39) — `Term::search(&self, regex: &Regex) -> Vec<Match>` is the existing API POLISH-06 builds UX on top of
- `crates/vector-config/src/lib.rs`, `crates/vector-theme/src/lib.rs`, `crates/vector-secrets/src/lib.rs` — Phase 1 skeleton stubs; Phase 5 fills these in
- `crates/vector-input/src/paste.rs` (D-53) — bracketed-paste helper; selection-string extraction (Cmd-C) and OSC 52 wiring extend this module
- `docs/adr/0003-architecture-lint-mechanism.md` — current arch-lint mechanism; D-83 sub-item 2 extends it with path-dep version assertion

### Tooling
- `.planning/todos/pending/2026-05-11-code-quality-hardening-workspace-lints-arch-lint-upgrade-pre-commit-cargo-deny.md` — folded todo (full spec for D-83's four sub-items)

### External references (web)
- iTerm2 OSC 52 / `.itermcolors` plist format reference
- tmux `allow-passthrough` docs (https://tmuxai.dev/tmux-allow-passthrough/) — informs D-71
- tmux passthrough cut-off bug (https://github.com/tmux/tmux/issues/4377) — informs D-71's 58-byte chunking
- Apple Secure Keyboard Entry guide (https://support.apple.com/guide/terminal/use-secure-keyboard-entry-trml109/mac) — informs D-80
- Apple `NSTextInputClient` reference + `setMarkedText:selectedRange:replacementRange:` — informs D-81
- `notify` crate docs (FSEvents on macOS) — informs D-69
- `keyring 4.0` crate docs (macOS Keychain) — informs `vector-secrets` API surface (Phase 6 caller)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`vector-term::search()`** (`crates/vector-term/src/search.rs`) — D-39 library API already returns regex matches across scrollback. POLISH-06 wires UX onto this; no new core logic needed.
- **`vector-input::wrap_bracketed_paste`** (`crates/vector-input/src/paste.rs`) — D-53 module that already handles bracketed-paste wrapping. OSC 52 outbound and Cmd-C copy extend this file.
- **Phase 3 per-cell tint uniform** (D-66 was reused for active-pane border in Phase 4) — reusable for:
  - Search match highlights (D-77)
  - Title-bar tint stripe (D-75) — or alternative `NSVisualEffectView`
  - Toast banner backgrounds (D-69)
- **Per-pane `Term` + grid** (Phase 2 + 4) — selection-string extraction (Cmd-C) walks the existing grid.
- **`vector-config` / `vector-theme` / `vector-secrets` crate skeletons** (Phase 1 D-01) — already in workspace; Phase 5 fills them in.
- **AppKit menu bar** (D-15) — File / Edit / Vector menus already wired with stubbed items. POLISH-08 adds `Vector → Secure Keyboard Entry`; D-82 enables `File → New Window`; D-75 adds `Vector → Switch Profile →` submenu.
- **`tracing` infrastructure** (D-32) — used for OSC-8 disallowed-scheme logging (D-78), config invalid-line warnings (D-69), tmux DCS rewrap traces (D-71).

### Established Patterns
- **Threading rules** — D-09 dedicated I/O thread; D-11 `clippy::await_holding_lock = "deny"`. All `notify` watcher work happens on the I/O thread; reload events route to main via `EventLoopProxy::send_event` (existing `UserEvent` channel).
- **Per-crate arch-lint test** (D-08) — D-83 sub-item 2 extends this pattern.
- **`SpawnCommand` for PTY** (D-38 + Phase 4 spawn flow) — profile config translates into a `SpawnCommand` for `LocalDomain::spawn_local` (Phase 5 wires only kind = local).

### Integration Points
- **`AppWindow` (Phase 4 D-67)** — owns `active_pane_id` + `compositors: HashMap<PaneId, Compositor>`. Search bar + toast banner + tint stripe render as additional viewport passes during the existing `RedrawRequested` loop.
- **`Mux` singleton (D-67)** — pane state grows: `cwd: Option<PathBuf>` (D-79), `prompt_marks: VecDeque<PromptMark>` (D-79), `profile_name: String` (D-74), `clipboard_write_policy: ClipboardPolicy` (D-70 — denormalized from active profile for fast access in the OSC 52 hot path).
- **`vector-term` parser dispatch** — extend OSC handler match arms for 7 / 8 / 10 / 11 / 12 / 52 / 133.
- **`vector-input::keymap`** — new entries: `Cmd-F` (open search), `Esc` (close search when search-open), `Cmd-Shift-R` (reload config — explicit menu fallback to FSEvents), `Cmd-N` (new window), `Cmd-Shift-P` (profile picker), `Cmd-C` (copy selection).

</code_context>

<specifics>
## Specific Ideas

- **Smart-case search** (D-77) — explicit reference to vim/rg behavior; never case-sensitive when query is all lowercase.
- **Title-bar tint stripe + minimal Cmd-Shift-P picker** (D-75) — user wants visual profile identification AND a fast keyboard switcher, but explicitly NOT a general command palette. This narrow carve-out is the meaningful product decision of Phase 5.
- **Prompt-once-per-origin clipboard policy** (D-70) — modeled on the macOS location-permission UX, not iTerm2's silent-allow.
- **`Cmd-N` = clean slate; `Cmd-T` = duplicate context** (D-82 vs D-65 / D-63) — semantic split between the two new-context shortcuts is deliberate.
- **Pitfall 11 alignment** — single TOML file, `deny_unknown_fields`, no DSL. This was the user's stated direction at the project level (PROJECT.md "out of scope: Lua") and is reinforced by D-68.

</specifics>

<deferred>
## Deferred Ideas

- **General command palette** (FEATURES.md row 68) — explicit deferral; the D-75 minimal profile picker does NOT establish precedent for action-listing, plugin actions, or fuzzy-everywhere search.
- **OSC 133 prompt-mark navigation UI** (`Cmd-PageUp` jump-to-prev-prompt, visible gutter chevrons) — data is captured in Phase 5 (D-79) but UI ships in a later polish phase.
- **OSC 9;4 progress reporting** (taskbar / tab indicator) — not in POLISH-04; FEATURES.md row 31 marks "Optional".
- **Drag-and-drop `.itermcolors` import + CLI `vector theme import`** — D-73 picks the file-drop approach; the drag-onto-app and CLI surfaces are noted but deferred.
- **Cmd-1..9 jump-to-tab + Cmd-1..9 jump-to-profile** — both collide with each other and with D-62; out of v1 scope.
- **Sparkle auto-updater** — PROJECT.md "not for v1" (signed builds required); revisit when signing happens.
- **Full IME with candidate window + active-composition coordination** — strictly v2 per Pitfall 16; D-81 is the v1 floor.
- **Per-window Secure Keyboard Entry** — Apple's API is process-level; the per-window illusion is rejected (D-80).
- **OSC 52 read** (clipboard query from terminal) — denied in v1 (D-70); could be opt-in v2 with per-profile gate.
- **Backlog 999.1 AI autocomplete** — independent v2 ambition, not Phase 5.

### Reviewed Todos (not folded)

- None. The only matching todo (`code-quality-hardening`) was folded into D-83.

</deferred>

---

*Phase: 05-polish-local-daily-driver*
*Context gathered: 2026-05-12*
