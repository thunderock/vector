---
phase: 05-polish-local-daily-driver
verified: 2026-05-14T08:00:00Z
status: passed
score: 8/8 must-haves verified
re_verification:
  previous_status: gaps_found
  previous_score: 8/8 logic-level; 5 wiring gaps
  gaps_closed:
    - "Chrome render surfaces (tint stripe, search bar, toast, profile picker) appear on screen — ChromePipelines struct wired into AppWindow.chrome_pipelines; render_window invokes all four passes per UI-SPEC §11 order"
    - "Cmd-N / Cmd-F / Cmd-Shift-P / Cmd-Shift-R keystrokes fire their UserEvents — EncodedKey::App variant + match_app_shortcut added to keymap; handle_app_shortcut dispatches to real state mutations"
    - "NSTextInputClient selectors implemented (D-81 five-selector minimum) — declare_class! VectorInputView subclass ships in ime.rs appkit_impl; App.ime field + WindowEvent::Ime handler wired"
    - "Cmd-C copies selected text to NSPasteboard — TermGridAccess adapter + selection_to_string call replacing empty-string stub"
    - "Switch Profile submenu populates from active profiles — rebuild_switch_profile_submenu via OnceLock + submenu_rows_for called on ConfigReloaded"
    - "ClipboardRouter wired end-to-end — clipboard_tx plumbed through Mux::with_channels; UserEvent::ClipboardStore dispatched to App.clipboard_router.handle"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Phase 5 manual smoke matrix (10 items) — re-verified on 2026-05-13 for IME gap (smoke #3)"
    expected: "All 10 items PASS; user signed off on 2026-05-12 (initial matrix) and 2026-05-13 (IME closure smoke #3)"
    why_human: "Visual chrome surfaces (tint stripe, search bar overlay, toast banner, profile picker modal), IME preedit underline, OSC 8 hover dotted-underline, and Cmd-N native window spawn cannot be programmatically verified in headless test environment."
    approval: "User-approved 2026-05-12 (initial 10/10) and 2026-05-13 (IME closure; per 05-15-SUMMARY.md 'Status: PASS — user approved 2026-05-13')"
---

# Phase 5: Polish (Local Daily-Driver) Verification Report

**Phase Goal:** Polish the local terminal experience to daily-driver quality — config hot-reload, theme engine, search bar, profile picker, OSC 52 clipboard, IME, Secure Keyboard Entry, hyperlinks, OSC 7 cwd tracking, and Cmd-N window spawning all wire up end-to-end from a cold `cargo run`.
**Verified:** 2026-05-14T08:00:00Z
**Status:** passed
**Re-verification:** Yes — after gap closure (plans 05-11 through 05-16)

## Goal Achievement

### Observable Truths

| #   | Truth                                                                                                  | Status     | Evidence                                                                                                                                                                                                                      |
| --- | ------------------------------------------------------------------------------------------------------ | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Editing `~/.config/vector/config.toml` hot-reloads theme/font/keybinds without restart                | ✓ VERIFIED | `vector_config::spawn_watcher` (notify-debouncer-full, 150ms) + `try_load_or_keep` + `diff_config`; `main.rs` spawns watcher; `UserEvent::ConfigReloaded` handler updates `current_config`, rebuilds Switch Profile submenu  |
| 2   | Ligatures, Nerd Font glyphs, `.itermcolors` import all render with user-supplied font                  | ✓ VERIFIED | `vector_fonts::FontStack` + `set_ligatures`; `vector_theme::parse_itermcolors`; all theme/font tests pass; human smoke #1 + #2 approved 2026-05-12                                                                            |
| 3   | `printf '\e]52;c;...\a'` puts text in macOS clipboard; tmux DCS-wrapped round-trips                   | ✓ VERIFIED | `ClipboardRouter` on `App`; `clipboard_tx` plumbed Mux → PtyActor → `UserEvent::ClipboardStore` → `ClipboardRouter::handle` → `write_pasteboard`; `osc52::*` tests pass                                                     |
| 4   | Scrollback regex search highlights with next/prev; OSC 7/8/10/11/12/133 observable                    | ✓ VERIFIED | `SearchBar` + `MatchCache` on App; `SearchBarPass` drawn by chrome pass when `search_bar.open`; `EncodedKey::App(AppShortcut::ToggleSearch)` routes Cmd-F; OSC sniffers green                                                 |
| 5   | Profiles, SKE toggle, Cmd-N window spawn, and IME preedit display all function                         | ✓ VERIFIED | `ProfilePicker`/`PickerPass` wired; `SecureInputGuard` toggle live; `WinitWindowFactory::create_ungrouped` called from `handle_app_shortcut(SpawnNewWindow)`; `VectorInputView` declare_class! + `WindowEvent::Ime` dispatch  |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact                                               | Expected                                               | Status        | Details                                                      |
| ------------------------------------------------------ | ------------------------------------------------------ | ------------- | ------------------------------------------------------------ |
| `crates/vector-app/src/chrome.rs`                      | ChromePipelines struct holding four wgpu passes        | ✓ VERIFIED    | New (Plan 05-16); instantiated at AppWindow creation         |
| `crates/vector-app/src/app.rs` (AppWindow field)       | `chrome_pipelines: Option<ChromePipelines>`            | ✓ VERIFIED    | Line 59; parallel to `render_host` for disjoint borrows      |
| `crates/vector-app/src/app.rs` (render_window)         | Chrome pass invoked after pane compositor loop         | ✓ VERIFIED    | Lines 705-771; all four draw calls conditional on state      |
| `crates/vector-input/src/keymap.rs`                    | `match_app_shortcut` + `EncodedKey::App` variant       | ✓ VERIFIED    | All four shortcuts: Cmd-N/F/Shift-P/Shift-R recognized       |
| `crates/vector-app/src/app.rs` (handle_app_shortcut)   | Real state mutations for all four shortcuts            | ✓ VERIFIED    | `do_toggle_search`, `do_open_profile_picker`, `create_ungrouped`, `do_reload_config` |
| `crates/vector-app/src/app.rs` (App fields)            | `search_bar: SearchBar`, `profile_picker: ProfilePicker` | ✓ VERIFIED  | Lines 116-119; defaults closed; populated on ConfigReloaded  |
| `crates/vector-app/src/ime.rs` (appkit_impl)           | `define_class!` VectorInputView with 5 selectors       | ✓ VERIFIED    | `insertText:`, `setMarkedText:`, `unmarkText`, `markedRange`, `selectedRange`, `hasMarkedText` |
| `crates/vector-app/src/app.rs` (WindowEvent::Ime)      | `Ime::Preedit` → `ime.set_preedit`, `Ime::Commit` → `ime.commit` | ✓ VERIFIED | Lines 1460-1482; Pitfall-9 safe (preedit never written to PTY) |
| `crates/vector-app/src/term_grid_access.rs`            | `TermGridAccess` implementing `GridAccess` for `Term`  | ✓ VERIFIED    | New (Plan 05-11); avoids vector-input→vector-mux→vector-term cycle |
| `crates/vector-app/src/app.rs` (Cmd-C)                 | `selection_to_string` call replacing empty-string stub | ✓ VERIFIED    | Lines 1290-1300; passes `TermGridAccess` + `SelectionMode::Stream` |
| `crates/vector-app/src/menu.rs`                        | `rebuild_switch_profile_submenu` via `OnceLock`        | ✓ VERIFIED    | Lines 316-342; drain-and-repopulate on ConfigReloaded        |
| `crates/vector-mux/src/mux.rs`                         | `clipboard_tx: Option<mpsc::Sender<ClipboardEvent>>`  | ✓ VERIFIED    | Lines 37-72; `with_channels` plumbs tx to ForwardingListener |
| `crates/vector-app/src/clipboard_router.rs`            | `ClipboardRouter` on App; `UserEvent::ClipboardStore` arm | ✓ VERIFIED | Lines 1241-1257 in app.rs; router policy applied             |

Previously ORPHANED artifacts now VERIFIED:

| Artifact                                              | Previous Status | Current Status | Change                                     |
| ----------------------------------------------------- | --------------- | -------------- | ------------------------------------------ |
| `crates/vector-render/src/tint_stripe.rs`             | ORPHANED        | ✓ VERIFIED     | Instantiated via ChromePipelines; draw called per frame |
| `crates/vector-render/src/search_bar_pass.rs`         | ORPHANED        | ✓ VERIFIED     | Draw called when `search_bar.open`         |
| `crates/vector-render/src/toast_pass.rs`              | ORPHANED        | ✓ VERIFIED     | Draw called when `toasts.current()` is Some |
| `crates/vector-render/src/picker_pass.rs`             | ORPHANED        | ✓ VERIFIED     | Draw called when `profile_picker.open`     |
| `crates/vector-app/src/clipboard_router.rs`           | ORPHANED        | ✓ VERIFIED     | App.clipboard_router field; ClipboardStore UserEvent |
| `crates/vector-app/src/search_bar.rs`                 | ORPHANED        | ✓ VERIFIED     | App.search_bar field; opened by ToggleSearch shortcut |
| `crates/vector-app/src/profile_picker.rs`             | ORPHANED        | ✓ VERIFIED     | App.profile_picker field; opened by OpenProfilePicker |
| `crates/vector-app/src/ime.rs`                        | STUB            | ✓ VERIFIED     | `declare_class!` ships; `App.ime` wired to WindowEvent::Ime |

### Key Link Verification

| From                                          | To                                        | Via                                             | Status     |
| --------------------------------------------- | ----------------------------------------- | ----------------------------------------------- | ---------- |
| Keyboard Cmd-N/F/Shift-P/Shift-R              | `handle_app_shortcut`                     | `match_app_shortcut` → `EncodedKey::App`        | ✓ WIRED    |
| `UserEvent::SpawnNewWindow`                   | `WinitWindowFactory::create_ungrouped`    | `handle_app_shortcut(SpawnNewWindow)`            | ✓ WIRED    |
| `UserEvent::ToggleSearch`                     | `App.search_bar.open_with` / `close`      | `handle_app_shortcut(ToggleSearch)`              | ✓ WIRED    |
| `UserEvent::OpenProfilePicker`                | `App.profile_picker.open()`               | `handle_app_shortcut(OpenProfilePicker)`         | ✓ WIRED    |
| `UserEvent::ReloadConfig`                     | `do_reload_config` + submenu rebuild      | `handle_app_shortcut(ReloadConfig)`              | ✓ WIRED    |
| `TintStripePipeline::draw`                    | wgpu RenderPass                           | `chrome.tint.draw(&mut rpass)` in render_window | ✓ WIRED    |
| `SearchBarPass::draw`                         | wgpu RenderPass                           | conditional on `search_bar.open` + `active_pane_rect` | ✓ WIRED |
| `ToastPass::draw`                             | wgpu RenderPass                           | conditional on `toasts.current()` Some           | ✓ WIRED    |
| `PickerPass::draw_scrim + draw_modal`         | wgpu RenderPass                           | conditional on `profile_picker.open`             | ✓ WIRED    |
| `declare_class! VectorInputView` (AppKit)     | `ImeState::{set_preedit, commit, clear}`  | `Mutex<ImeState>` ivars; selector implementations | ✓ WIRED  |
| `WindowEvent::Ime(Preedit/Commit)`            | `App.ime.set_preedit / commit / clear`    | match arm at line 1460                           | ✓ WIRED    |
| `Cmd-C (mods.cmd + "c")`                      | `selection_to_string` + `write_pasteboard` | `TermGridAccess` adapter + `SelectionMode::Stream` | ✓ WIRED |
| `UserEvent::ConfigReloaded`                   | `rebuild_switch_profile_submenu`          | OnceLock + `submenu_rows_for(cfg)`               | ✓ WIRED    |
| `ForwardingListener::clipboard_tx`            | `UserEvent::ClipboardStore`               | Mux::with_channels → PtyActor drain task → EventLoopProxy | ✓ WIRED |
| `vector-config::watcher::spawn_watcher`       | `App::user_event(ConfigReloaded)`         | mpsc + EventLoopProxy                            | ✓ WIRED    |
| `vector-term::Term::feed`                     | `osc_sniff::OscSniff`                     | parallel-parser dispatch                         | ✓ WIRED    |
| `WindowEvent::MouseInput (Cmd+click)`         | `hyperlink_dispatch::dispatch_cmd_click`  | open_with_nsworkspace / toast on reject          | ✓ WIRED    |
| `App.ske_guard.toggle()`                      | Carbon EnableSecureEventInput             | UserEvent::ToggleSecureKeyboardEntry              | ✓ WIRED    |

### Data-Flow Trace (Level 4)

| Artifact                        | Data Variable           | Source                                     | Produces Real Data | Status    |
| ------------------------------- | ----------------------- | ------------------------------------------ | ------------------ | --------- |
| `App.current_config`            | Arc<ConfigFile>         | watcher thread (FSEvents → parse)          | Yes                | ✓ FLOWING |
| `pane.cwd` (Mux::Pane)          | Mutex<Option<PathBuf>>  | OSC 7 sniffer → cwd_ring().back()          | Yes                | ✓ FLOWING |
| `App.toasts`                    | ToastStack              | hyperlink_dispatch, ConfigError, clipboard | Yes                | ✓ FLOWING |
| `App.search_bar`                | SearchBar               | Cmd-F → `do_toggle_search`                 | Yes                | ✓ FLOWING |
| `App.profile_picker`            | ProfilePicker           | Cmd-Shift-P → `do_open_profile_picker`     | Yes                | ✓ FLOWING |
| `App.clipboard_router`          | ClipboardRouter         | OSC 52 → clipboard_tx → ClipboardStore     | Yes                | ✓ FLOWING |
| `App.ime`                       | ImeState                | WindowEvent::Ime dispatch                  | Yes                | ✓ FLOWING |
| `App.ske_guard`                 | SecureInputGuard        | UserEvent::ToggleSecureKeyboardEntry        | Yes                | ✓ FLOWING |
| `App.write_pasteboard` (Cmd-C)  | selection text          | `selection_to_string` via TermGridAccess   | Yes                | ✓ FLOWING |
| Chrome pipelines (4)            | wgpu Buffer uniforms    | per-frame state snapshots in render_window | Yes                | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior                                              | Command                                                                  | Result                    | Status |
| ----------------------------------------------------- | ------------------------------------------------------------------------ | ------------------------- | ------ |
| Workspace compiles + all tests pass                   | `cargo test --workspace --tests --no-fail-fast`                          | 332 passed; 0 failed; 4 ignored | ✓ PASS |
| OSC 52 raw + DCS-wrapped + read-denied tests pass     | (included in workspace run)                                              | passing                   | ✓ PASS |
| OSC 7 + 133 sniffer tests pass                        | (included in workspace run)                                              | passing                   | ✓ PASS |
| iTerm2 importer + builtins + appearance tests pass    | (included in workspace run)                                              | passing                   | ✓ PASS |
| Config watcher 150ms debounce + last-good             | (included in workspace run)                                              | passing                   | ✓ PASS |
| SKE Carbon FFI mock counter                           | (included in workspace run)                                              | passing                   | ✓ PASS |
| ImeState set_preedit / commit / clear                 | (included in workspace run)                                              | passing                   | ✓ PASS |
| Chrome quad / search-bar / toast / picker geometry    | (included in workspace run)                                              | passing                   | ✓ PASS |
| `EncodedKey::App` dispatch for all 4 shortcuts        | (included in workspace run — plan 05-13/05-14 tests)                     | passing                   | ✓ PASS |
| `handle_app_shortcut` state mutations                 | (included in workspace run — plan 05-14 tests)                           | passing                   | ✓ PASS |
| Switch Profile rebuild `submenu_rows_for`             | (included in workspace run — plan 05-11 tests)                           | passing                   | ✓ PASS |
| Cmd-C `selection_to_string` via TermGridAccess        | (included in workspace run — plan 05-11 tests)                           | passing                   | ✓ PASS |
| ClipboardRouter policy dispatch tests                 | (included in workspace run — plan 05-12 tests)                           | passing                   | ✓ PASS |
| NSTextInputClient declare_class! regression tests     | (included in workspace run — plan 05-15 tests; macOS cfg-gated)          | passing (cfg-gated pass)  | ✓ PASS |
| Chrome render pass per-frame wiring order (W6)        | (included in workspace run — plan 05-16 tests; 7/7 PASS)                 | passing                   | ✓ PASS |
| Live render of chrome surfaces                        | n/a — headless; covered by human smoke matrix                            | user-approved 2026-05-12  | ? HUMAN|
| Hiragana IME preedit underline                        | n/a — requires macOS AppKit runtime                                      | user-approved 2026-05-13  | ? HUMAN|
| tmux 3.4+ real DCS round-trip                         | `cargo test -p vector-term --test osc52_tmux -- --ignored`               | `#[ignore]` (CI smoke job) | ? SKIP |

### Requirements Coverage

| Requirement | Source Plan(s)           | Description                                                                                           | Status         | Evidence                                                                                                           |
| ----------- | ------------------------ | ----------------------------------------------------------------------------------------------------- | -------------- | ------------------------------------------------------------------------------------------------------------------ |
| POLISH-01   | 05-02, 05-04             | TOML config hot-reload via notify; profile inheritance without scripting                              | ✓ SATISFIED    | `vector_config::{schema,loader,watcher,apply}`; `main.rs` spawns watcher; flat-overlay tested                     |
| POLISH-02   | 05-04, 05-07             | Bring-your-own-font; opt-in ligatures; Nerd Font glyphs render                                        | ✓ SATISFIED    | `vector_fonts::FontStack` ligature toggle + CoreText; smoke #2 approved                                           |
| POLISH-03   | 05-03                    | Built-in light + dark themes + `.itermcolors` importer                                                | ✓ SATISFIED    | `vector_theme::{builtins,itermcolors,appearance}`; UI-SPEC §9.1 chrome tokens                                     |
| POLISH-04   | 05-05, 05-10             | OSC 7 + OSC 8 + OSC 10/11/12 + OSC 133 implemented                                                   | ✓ SATISFIED    | `osc_sniff.rs`, `hyperlink.rs`, `listener.rs`; OSC 8 Cmd-click + hover wired live                               |
| POLISH-05   | 05-06                    | OSC 52 clipboard copy works in raw and DCS-wrapped forms                                              | ✓ SATISFIED    | `clipboard_tx` end-to-end; `ClipboardRouter` on App; `osc52::*` tests pass; `write_pasteboard` called            |
| POLISH-06   | 05-07, 05-10, 05-11      | Scrollback regex search with match highlighting and next/prev navigation                              | ✓ SATISFIED    | `SearchBar` on App; `SearchBarPass` drawn in chrome pass; Cmd-F in keymap; smoke #6 approved                     |
| POLISH-07   | 05-02, 05-08, 05-10, 05-11 | Profiles with per-profile env/theme/tint/startup command; profile picker                            | ✓ SATISFIED    | `ProfilePicker`/`PickerPass` wired; `TintStripePipeline` drawn; Switch Profile rebuilt on ConfigReloaded; smoke #7|
| POLISH-08   | 05-09, 05-15             | Secure Keyboard Entry toggle + basic IME composition via NSTextInputClient                            | ✓ SATISFIED    | `SecureInputGuard` RAII live; `VectorInputView` declare_class! ships; `App.ime` wired; smoke #3 #4 approved      |

**Orphaned requirements:** None — every POLISH-0[1-8] ID is claimed by at least one PLAN frontmatter `requirements` field and all are SATISFIED.

### Anti-Patterns Found

| File                                              | Line   | Pattern                                                                       | Severity | Impact                                                                                     |
| ------------------------------------------------- | ------ | ----------------------------------------------------------------------------- | -------- | ------------------------------------------------------------------------------------------ |
| `crates/vector-app/src/app.rs`                    | 91     | `TODO(phase-5): allocate a fresh Mux Window per NSWindow`                    | ℹ Info   | Mux WindowId sharing — noted but non-blocking for v1; Cmd-N spawns window, new Mux window allocation is a follow-up |
| `crates/vector-app/src/app.rs`                    | 326    | `does not yet exist; 05-15 only handles resumed() + handle_new_tab()`        | ℹ Info   | Comment documenting known ordering constraint; no user-visible stub behavior               |
| `crates/vector-app/src/menu.rs`                   | 79     | `// placeholder so the menu-bar surface is discoverable`                     | ℹ Info   | Legit comment about macOS menu bar structural requirement; not a code stub                 |

No WARNING or BLOCKER anti-patterns. The three Info items are documentation comments about known deferred scope, not stubs blocking observable behavior.

### Human Verification Required

The Phase 5 manual smoke matrix is the canonical phase-gate, **approved by the user at two checkpoints:**

1. **2026-05-12** — Initial 10/10 PASS (per 05-09-SUMMARY.md)
2. **2026-05-13** — IME gap closure re-smoke, items #3 + #4 (per 05-15-SUMMARY.md "Status: PASS — user approved 2026-05-13")

Items needing human confirmation (all previously approved; recording for audit trail):

1. **Config hot-reload (smoke #1)** — Edit `~/.config/vector/config.toml`, save, expect live apply + toast. User-approved 2026-05-12.
2. **Theme import (smoke #2)** — Drop `.itermcolors` into `~/.config/vector/themes/`. User-approved 2026-05-12.
3. **Hiragana preedit (smoke #3)** — Japanese input, `aiueo` → `あいうえお` preedit → Enter commits. User-approved 2026-05-13 (after declare_class! closure).
4. **SKE toggle (smoke #4)** — Vector → Secure Keyboard Entry menu toggle. User-approved 2026-05-13.
5. **Cmd-N window (smoke #5)** — Fresh `[default]` window at `$HOME`. User-approved 2026-05-12.
6. **Cmd-F search bar (smoke #6)** — 32 px bar, smart-case, next/prev, 1000+ counter, Esc restores. User-approved 2026-05-12.
7. **Cmd-Shift-P profile picker (smoke #7)** — Fuzzy modal, `Phase 6+` label on remote profiles. User-approved 2026-05-12.
8. **OSC 7 cwd-aware (smoke #8)** — Cmd-T inherits cwd; tab title shows `zsh: vector`. User-approved 2026-05-12.
9. **Cmd-Shift-R reload (smoke #9)** — View → Reload Config fallback. User-approved 2026-05-12.
10. **OSC 8 hover + click (smoke #10)** — Cmd-hover pointer cursor + dotted underline; Cmd-click opens browser; disallowed scheme toast. User-approved 2026-05-12.

### Gaps Summary

All five wiring gaps from the initial verification (2026-05-12) have been closed by plans 05-11 through 05-16:

1. **Gap #1 — Chrome render surfaces ORPHANED** → closed by Plan 05-16: `ChromePipelines` struct in `chrome.rs` owned by `AppWindow`; per-frame chrome pass draws all four surfaces in UI-SPEC §11 order.
2. **Gap #2 — Cmd-N/F/Shift-P/Shift-R keystrokes not routed** → closed by Plan 05-13/05-14: `match_app_shortcut` added to `keymap.rs`; `EncodedKey::App` dispatched to `handle_app_shortcut` with real state mutations.
3. **Gap #3 — NSTextInputClient declare_class! deferred** → closed by Plan 05-15: `VectorInputView` subclass with 5 selectors ships in `ime.rs appkit_impl`; `App.ime` field + `WindowEvent::Ime` handler wired; `set_ime_allowed(true)` called on new windows.
4. **Gap #4 — Cmd-C writes empty string** → closed by Plan 05-11: `TermGridAccess` implements `GridAccess`; `selection_to_string` called with range and grid adapter; non-empty selection written to `NSPasteboard`.
5. **Gap #5 — Switch Profile submenu static placeholder** → closed by Plan 05-11: `SWITCH_PROFILE_SUBMENU` OnceLock captures the NSMenu at install time; `rebuild_switch_profile_submenu` drains and repopulates on `UserEvent::ConfigReloaded`.
6. **Gap #6 (implicit) — ClipboardRouter not wired to ForwardingListener** → closed by Plan 05-12: `clipboard_tx` plumbed through `Mux::with_channels` to `Term`'s `ForwardingListener`; drain task emits `UserEvent::ClipboardStore`; App arm routes through `ClipboardRouter::handle`.

**All POLISH-0[1-8] requirements SATISFIED. 332 tests pass. Human smoke matrix approved at two checkpoints. Phase goal achieved.**

---

_Verified: 2026-05-14T08:00:00Z_
_Verifier: Claude (gsd-verifier; sonnet-4-6)_
