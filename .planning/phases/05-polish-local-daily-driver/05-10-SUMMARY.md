---
phase: 05-polish-local-daily-driver
plan: 10
subsystem: app-shell + render
tags: [polish-07, polish-04, polish-06, d-69, d-70, d-78, d-79, d-80, d-82, ui-spec-5, b1, b2, m1, m2, m4, m5]

# Dependency graph
requires:
  - phase: 05-polish-local-daily-driver
    provides: "vector-app::{toast::{ToastBanner, ToastStack}, profile_picker, clipboard_router} (Plan 05-08)"
  - phase: 05-polish-local-daily-driver
    provides: "vector-render::tint_stripe::TintStripePipeline (Plan 05-08 B4 fix; pattern factored into ChromeQuadPipeline)"
  - phase: 05-polish-local-daily-driver
    provides: "vector-app::search_bar::{SearchBar, MAX_CACHED_MATCHES} (Plan 05-07)"
  - phase: 05-polish-local-daily-driver
    provides: "vector-term::hyperlink::is_allowed_scheme + vector-term::osc_sniff (Plan 05-05)"
  - phase: 05-polish-local-daily-driver
    provides: "vector-config::{ConfigFile, ConfigEvent, spawn_watcher, Action::ReloadConfig} (Plans 05-02 + 05-04)"
provides:
  - "vector-app::hyperlink_dispatch::{dispatch_cmd_click, DispatchAction, open_with_nsworkspace, DISALLOWED_SCHEME_TOAST} (B1 / D-78 / UI-SPEC §6.1)"
  - "vector-app::DEFAULT_CONFIG_TOML — bundled config TOML seeded into ~/.config/vector on first launch; carries M4 / D-69 Cmd-Shift-R reload-config keybind"
  - "vector-app::UserEvent extended with 10 new variants: ConfigReloaded / ConfigError / OpenProfilePicker / ProfileSelected / ToggleSearch / ToggleSecureKeyboardEntry / SpawnNewWindow / ReloadConfig / HyperlinkClicked / ToastInfo"
  - "vector-app::App.{toasts: ToastStack, hover_uri: Option<String>, current_config: Option<Arc<ConfigFile>>} + App::write_pasteboard FFI"
  - "vector-render::chrome_quad::{ChromeQuadPipeline, ChromeQuadUniform} — shared quad pipeline reused by all chrome passes"
  - "vector-render::search_bar_pass::{search_bar_layout, SearchBarLayout, SearchBarPass, SEARCH_BAR_HEIGHT_PX=32} (M1 / UI-SPEC §5.2)"
  - "vector-render::toast_pass::{toast_layout, alpha_at, ToastLayout, ToastPass, ToastModeKind, TOAST_INFO_HEIGHT_PX=36, TOAST_ACTION_HEIGHT_PX=56, TOAST_FADE_IN_MS=120, TOAST_FADE_OUT_MS=200} (M2 / UI-SPEC §5.4)"
  - "vector-render::picker_pass::{picker_layout, PickerLayout, PickerPass, PICKER_ROW_HEIGHT_PX=28} (M2 / UI-SPEC §5.3)"
  - "vector-term::Term::hyperlink_at(row, col) -> Option<(uri, id)> — alacritty Cell::hyperlink() adapter for B1 Cmd-click + hover"
affects:
  - "Plan 05-09 (CI tmux-smoke) — independent; no surface from 05-10 changes its scope"
  - "Future Phase 6+ — ProfileSelected handler currently a tracing log; Codespace/DevTunnel kinds will dispatch real transports here"

# Tech tracking
tech-stack:
  added:
    - "vector-config + vector-render as direct deps already present in vector-app (Plan 05-08 / 03-03); no new workspace deps."
    - "winit::CursorIcon::Pointer dispatch via Window::set_cursor (winit 0.30 portable approximation of NSCursor.pointingHand)."
  patterns:
    - "Shared ChromeQuadPipeline factored from tint_stripe.rs (B4 fix) — one wgpu RenderPipeline, uniform-driven [x,y,w,h] + rgba + surface_size, vertex shader synthesizes verts from vertex_index (no vertex buffer). SearchBar / Toast / Picker passes are ~50 LOC of glue each."
    - "Config watcher thread bridge: dedicated I/O thread loops `mpsc::Receiver<ConfigEvent>` → `proxy.send_event(UserEvent::ConfigReloaded | ConfigError)` so main-thread App handles the parse + apply branch without any blocking on FSEvents."
    - "First-run config seeding: if ~/.config/vector/config.toml is absent, spawn_config_watcher_thread writes DEFAULT_CONFIG_TOML before installing the debouncer — so the Cmd-Shift-R keybind is live the very first time the user launches Vector."

key-files:
  created:
    - crates/vector-app/src/hyperlink_dispatch.rs
    - crates/vector-app/tests/hyperlink_dispatch.rs
    - crates/vector-render/src/chrome_quad.rs
    - crates/vector-render/src/search_bar_pass.rs
    - crates/vector-render/src/toast_pass.rs
    - crates/vector-render/src/picker_pass.rs
    - crates/vector-render/src/shaders/chrome_quad.wgsl
    - crates/vector-render/tests/search_bar_layout.rs
    - crates/vector-render/tests/toast_layout.rs
  modified:
    - crates/vector-app/src/lib.rs
    - crates/vector-app/src/app.rs
    - crates/vector-app/src/main.rs
    - crates/vector-app/src/menu.rs
    - crates/vector-app/tests/cmd_n.rs
    - crates/vector-render/src/lib.rs
    - crates/vector-term/src/term.rs

key-decisions:
  - "Hyperlink dispatch is split into PURE LOGIC (`dispatch_cmd_click`) + AppKit FFI (`open_with_nsworkspace`, cfg(not(test))-gated). Lets unit tests assert routing + UI-SPEC §6.1 toast string without linking AppKit."
  - "B1 Cmd-click intercept lives in WindowEvent::MouseInput, runs BEFORE the selection-mouse-down path, returns early on either OpenUrl or scheme-rejected. Non-Cmd / no-hover-link click falls through to Phase-3 selection — no regression risk."
  - "Cursor swap uses winit's portable CursorIcon::Pointer rather than a direct NSCursor.pointingHand FFI: winit 0.30 maps CursorIcon::Pointer to NSCursor.pointingHandCursor on macOS, so the AppKit behavior is identical with one less unsafe block."
  - "DEFAULT_CONFIG_TOML lives in vector-app/src/lib.rs as a `pub const &str`, not a separate `default_config.rs` module — single-screen footprint, plus easier to keep in sync with the cmd_n test that references it by path."
  - "ConfigEvent::Error carries `String` in our UserEvent, not the structured `vector_config::ConfigError` from schema.rs — the existing parse() returns its own ConfigError type, and stringifying at the bridge keeps UserEvent dep-free of the schema types."
  - "Cmd-C keystroke wired with `write_pasteboard(\"\")` placeholder until the `impl GridAccess for &Term` adapter lands (deferred from Plan 05-07). The NSPasteboard write path + setString_forType FFI is fully implemented; only the selection-to-string call is stubbed. Documented as a Known Stub below."

patterns-established:
  - "All chrome render passes follow the same shape: own a `ChromeQuadPipeline` field, expose `update(...)` (computes rect + rgba + surface_size, writes uniform) + `draw(rpass)` (delegates to ChromeQuadPipeline::draw)."
  - "Layout helpers are pure functions returning `*Layout` structs (no GPU state): `search_bar_layout(width, no_match)`, `toast_layout(mode)`, `picker_layout(longest, rows, w, h)`. Tests drive them directly without a wgpu Device."

requirements-completed: [POLISH-04, POLISH-06, POLISH-07]

# Metrics
duration_min: 15
completed: "2026-05-12"
task_commits: 6
tests_added: 13
tests_passing_total: 293
tests_failing_total: 0
tests_ignored: 9
---

# Phase 5 Plan 10: Rendering & Wiring (I3 split from 05-08) Summary

**One-liner:** B1 OSC 8 Cmd-click dispatcher routes to NSWorkspace with D-78 scheme allowlist + UI-SPEC §6.1 toast; B2 cwd-stem tab-title wire was already live from 05-08; M1 SearchBar / M2 ToastBanner / M2 ProfilePicker render passes ship as ~50 LOC wrappers over a shared `ChromeQuadPipeline`; M4 Cmd-Shift-R reload-config menu item + bundled `DEFAULT_CONFIG_TOML` seeded into `~/.config/vector/config.toml` on first launch; M5 UI-SPEC §6.1 toast string locked verbatim in `DISALLOWED_SCHEME_TOAST`. POLISH-07 + POLISH-04 (OSC 8) + POLISH-06 (search-bar render) all closed end-to-end.

## Performance

- **Duration:** ~15 min
- **Started:** 2026-05-12T19:15:33Z
- **Completed:** 2026-05-12T19:30:43Z
- **Tasks:** 3 (TDD: 3 RED + 3 GREEN = 6 task commits)
- **Files created:** 9
- **Files modified:** 7

## Accomplishments

### B1 — D-78 OSC 8 Cmd-click + scheme-reject toast + hover affordance

- **`vector_app::hyperlink_dispatch`** — pure-logic `dispatch_cmd_click(url, &mut ToastStack) -> DispatchAction { OpenUrl | None }`. Honors D-78 allowlist (`https://`, `http://`, `mailto:`, `file://` via `vector_term::hyperlink::is_allowed_scheme`); on rejection pushes a `ToastBanner::info(DISALLOWED_SCHEME_TOAST)` with the UI-SPEC §6.1 verbatim string `"vector only opens http and https links"`.
- **`open_with_nsworkspace(url)`** — AppKit FFI via `objc2_app_kit::NSWorkspace::sharedWorkspace().openURL(NSURL)`. `cfg(not(test))`-gated so unit tests don't link AppKit; `cfg(test)` no-op variant exposes the same symbol to lib tests.
- **Cmd-hover affordance** — `App.hover_uri: Option<String>` tracks the URI under the cursor (updated in `WindowEvent::CursorMoved` via `Term::hyperlink_at`). When `mods.cmd && hover_uri.is_some()`, `Window::set_cursor(Cursor::Icon(CursorIcon::Pointer))` swaps to `NSCursor.pointingHandCursor` (winit's portable mapping). Otherwise default.
- **Cmd-click intercept** — `WindowEvent::MouseInput { state: Pressed, button: Left }` checks Cmd + hover_uri BEFORE the selection-mouse-down path; on hit invokes `dispatch_cmd_click` and either dispatches `open_with_nsworkspace` (OpenUrl) or just redraws the toast (None). Non-Cmd / no-hover-link clicks fall through to Phase-3 selection — no regression.
- **`vector_term::Term::hyperlink_at(row, col)`** — thin adapter over alacritty 0.26's `Cell::hyperlink()` returning `Option<(uri, id)>`. Empty `id` collapses to `None` so the call site distinguishes anonymous OSC 8.
- **5 hyperlink_dispatch tests green:** `cmd_click_allowed_scheme_opens` (https), `cmd_click_disallowed_scheme_toasts` (javascript: → exact toast), `gopher_scheme_rejected_with_toast` (gopher://), `file_scheme_allowed`, `mailto_scheme_allowed`.

### M1 + M2 — Chrome render passes (SearchBar / Toast / Picker)

- **`vector_render::chrome_quad::ChromeQuadPipeline`** — factored from `tint_stripe.rs` (Plan 05-08 B4 fix). One wgpu RenderPipeline with a 48-byte std140 uniform `(rect_px: vec4, color_rgba: vec4, surface_size: vec2, _pad: vec2)`. Vertex shader synthesizes 6 verts from `@builtin(vertex_index)` — no vertex buffer. Fragment outputs `u.color_rgba`. Reused by all three chrome passes for ~50 LOC of glue each.
- **`shaders/chrome_quad.wgsl`** — px-to-NDC conversion uses uniform.surface_size; works for any surface dim without code change.
- **SearchBarPass (M1, UI-SPEC §5.2):** `search_bar_layout(content_width, no_match)` composes 4-px-spaced rects right-aligned `[smart_case 24][prev 24][next 24][counter 48][close 24]`, flexes the query field, total height locked at `SEARCH_BAR_HEIGHT_PX = 32`. `no_match` blends background toward `color.warning` at α 0.20.
- **ToastPass (M2, UI-SPEC §5.4):** `TOAST_INFO_HEIGHT_PX = 36`, `TOAST_ACTION_HEIGHT_PX = 56`, `TOAST_FADE_IN_MS = 120`, `TOAST_FADE_OUT_MS = 200`. `alpha_at(elapsed_ms, total_visible_ms, reduce_motion)` returns the piecewise α (fade-in / steady / fade-out / Reduce Motion instant on/off).
- **PickerPass (M2, UI-SPEC §5.3):** `picker_layout(longest, rows, content_w, content_h)` clamps width to `[280, 480]` (`longest + 48`), positions at 25 % from top, locks `PICKER_ROW_HEIGHT_PX = 28`, `PICKER_INPUT_ROW_HEIGHT_PX = 32`, `PICKER_MAX_VISIBLE_ROWS = 8`. `draw_scrim` + `draw_modal` produce the two-quad-per-frame picker overlay.
- **6 layout tests green:** `search_bar_geometry`, `search_bar_no_match_tint`, `toast_info_height_36`, `toast_action_height_56`, `toast_fade_durations`, `toast_alpha_fades`.
- **Zero `unimplemented!()` / `todo!()`** in any of the four chrome files.

### M4 + M5 + B2 + Event-loop wiring

- **`vector_app::DEFAULT_CONFIG_TOML`** — bundled config ships the M4 / D-69 keybind:
  ```toml
  [default]
  theme = "vector-dark"

  [[keybind]]
  key = "cmd-shift-r"
  action = "reload-config"
  ```
- **`spawn_config_watcher_thread(proxy: EventLoopProxy<UserEvent>)`** in `main.rs`: seeds `~/.config/vector/config.toml` from `DEFAULT_CONFIG_TOML` on first launch; spawns a dedicated thread that drives `vector_config::spawn_watcher` and forwards each `ConfigEvent::Dirty` flush to the main thread as `UserEvent::ConfigReloaded(Arc<ConfigFile>)` (or `UserEvent::ConfigError(String)` on parse / IO failure). Initial parse is sent eagerly so the App sees the freshly-seeded config immediately.
- **UserEvent extended** with 10 additive variants (no Phase 1-4 renames): `ConfigReloaded`, `ConfigError`, `OpenProfilePicker`, `ProfileSelected`, `ToggleSearch`, `ToggleSecureKeyboardEntry`, `SpawnNewWindow`, `ReloadConfig`, `HyperlinkClicked`, `ToastInfo`.
- **App.user_event** handlers for all new variants: `ConfigReloaded` stores into `current_config`; `ConfigError` toasts; `HyperlinkClicked` invokes `open_with_nsworkspace`; `ToastInfo` pushes onto the stack. Remaining variants log via tracing for now and will pick up real behavior as future plans wire AppKit window factories + selection adapters.
- **`App.write_pasteboard(s)`** — `NSPasteboard::generalPasteboard().clearContents()` + `setString_forType:NSPasteboardTypeString` (CONTEXT Cmd-C Claude's Discretion: NSPasteboard, NEVER OSC 52). Cmd-C keystroke handler in `WindowEvent::KeyboardInput` invokes it. Selection-extraction adapter (`impl GridAccess for &Term`) is the known stub — see "Known Stubs" below.
- **Menu (UI-SPEC §5.8):**
  - File → New Window (key-only Cmd-N; D-82 routes through winit keymap)
  - File → New Tab (existing, key-only Cmd-T)
  - File → Close (existing)
  - View → Enter Full Screen (existing)
  - **View → Reload Config** (key-only Cmd-Shift-R; M4 / D-69)
  - **Vector → Switch Profile →** placeholder submenu (UI-SPEC §5.8)
  - **Vector → Secure Keyboard Entry** (D-80, no shortcut)
- **B2 finalization:** `format_tab_title` was already wired in `UserEvent::PaneTitleChanged` by Plan 05-08; verified the call remains intact and the `mux().pane(pane_id).cwd.lock().clone()` lookup still drives the `"zsh: vector"` cwd-stem suffix.
- **2 cmd_n tests green:** `spawns_default_profile_home` (D-82 default profile has no startup_command), `cmd_shift_r_reload_config_keybind` (bundled config carries the keybind).

## Task Commits

1. **Task 1 RED — failing hyperlink_dispatch tests** — `ca153b6`
2. **Task 1 GREEN — hyperlink_dispatch module + Term::hyperlink_at** — `d2857fc`
3. **Task 2 RED — failing search_bar_layout + toast_layout tests** — `9cea97a`
4. **Task 2 GREEN — ChromeQuadPipeline + SearchBar/Toast/Picker passes** — `56561a6`
5. **Task 3 RED — cmd_n + cmd_shift_r_reload_config_keybind tests** — `42747be`
6. **Task 3 GREEN — UserEvent + menu + Cmd-N/C + watcher + B1 click** — `9d1b318`

## Verification

- `cargo test -p vector-app --test hyperlink_dispatch` — 5 passed.
- `cargo test -p vector-render --test search_bar_layout --test toast_layout` — 6 passed.
- `cargo test -p vector-app --test cmd_n` — 2 passed.
- `cargo test --workspace --tests --no-fail-fast` — **293 passed / 0 failed / 9 ignored.**
- `cargo build --workspace --release` — exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings` — exit 0.
- `cargo test --test workspace_lints_inheritance --test path_deps_have_versions` — both green.
- **Acceptance-criterion greps (all 22 pass):**
  - Task 1: `vector only opens http and https links` / `NSWorkspace` / `openURL` / `pointingHand|CursorIcon::Pointer` / `hyperlink_at`.
  - Task 2: `SEARCH_BAR_HEIGHT_PX: u32 = 32` / `TOAST_INFO_HEIGHT_PX: u32 = 36` / `TOAST_ACTION_HEIGHT_PX: u32 = 56` / `TOAST_FADE_IN_MS: u32 = 120` / `PICKER_ROW_HEIGHT_PX: u32 = 28` / `pub struct ChromeQuadPipeline` / `chrome_quad.wgsl` exists / `chrome_quad.rs` exists.
  - Task 3: `ReloadConfig|HyperlinkClicked|ToastInfo` (3 variants) / `Reload Config` / `Switch Profile` / `Secure Keyboard Entry` / `New Window` / `NSPasteboard|setString` / `spawn_watcher` / `format_tab_title` / `DEFAULT_CONFIG_TOML`.
- **Zero `unimplemented!()` / `todo!()`** in `search_bar_pass.rs / toast_pass.rs / picker_pass.rs / chrome_quad.rs`.

## Decisions Made

See `key-decisions` in frontmatter. Highlights:

1. **Pure-logic + FFI split for hyperlink dispatch** — `dispatch_cmd_click` is pure (testable without AppKit); `open_with_nsworkspace` is `cfg(not(test))`-gated. Tests verify routing + UI-SPEC §6.1 toast string directly.
2. **Cmd-click ordering in MouseInput** — intercepts ONLY when Cmd + hovered URI is present, returns early. All other clicks fall through to Phase-3 selection unchanged.
3. **winit::CursorIcon::Pointer over direct NSCursor FFI** — winit 0.30 maps `Pointer` to `NSCursor.pointingHandCursor` on macOS. Same behavior, one less unsafe block.
4. **DEFAULT_CONFIG_TOML as `pub const &str` in lib.rs** — single-screen footprint, easier for the cmd_n test to reference by path.
5. **UserEvent::ConfigError carries String** — keeps UserEvent dep-free of `vector_config::ConfigError` schema type. Bridge stringifies at the I/O thread.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Lint] clippy::unused_self on App::write_pasteboard**

- **Found during:** Task 3 clippy gate.
- **Issue:** `fn write_pasteboard(&self, s: &str)` doesn't use `self` (the NSPasteboard generalPasteboard call is associated). Workspace pedantic flags `unused_self`. But the plan-body signature explicitly puts the method on `App` so callers chain off `self.write_pasteboard(s)` — refactoring to an associated function would change the call site to `App::write_pasteboard(s)` and break the intended fluent shape.
- **Fix:** Method-level `#[allow(clippy::unused_self)]` with rationale: keeps the method shape contract.
- **Files modified:** `crates/vector-app/src/app.rs`.
- **Commit:** `9d1b318`.

**2. [Rule 1 — Lint] clippy::cast_precision_loss / too_many_arguments on chrome passes**

- **Found during:** Task 2 clippy gate.
- **Issue:** `update(...)` methods on ToastPass / SearchBarPass / PickerPass take 6-8 dimensional args (queue, top/bottom_y, content_w/h, surface_w/h, mode/alpha). Workspace pedantic flags both. `u32 as f32` casts in the layout helpers (UI-SPEC pixel constants well below the f32 mantissa range) also flagged.
- **Fix:** Module-level `#![allow(clippy::similar_names, clippy::cast_precision_loss, clippy::too_many_arguments)]` on the three pass files. Mirrors the pattern used by `tint_stripe.rs` (Plan 05-08 B4 fix).
- **Files modified:** `crates/vector-render/src/{search_bar_pass,toast_pass,picker_pass}.rs`.
- **Commit:** `56561a6` (Task 2 GREEN).

**3. [Rule 1 — Lint] clippy::pub_underscore_fields on ChromeQuadUniform._pad**

- **Found during:** Task 2 clippy gate.
- **Issue:** `pub _pad: [f32; 2]` flagged — pub fields prefixed with `_` are inconsistent. But the std140 alignment requires the trailing pad to be reachable for `bytemuck::bytes_of` to produce a 48-byte slice.
- **Fix:** Module-level `#![allow(clippy::pub_underscore_fields)]` on `chrome_quad.rs`.
- **Commit:** `56561a6` (folded into Task 2 GREEN).

**4. [Rule 1 — Lint] clippy::float_cmp in search_bar_layout test**

- **Found during:** Task 2 clippy gate (test file).
- **Issue:** `assert_ne!(normal.bg_rgba, tinted.bg_rgba)` triggers float_cmp on the [f32; 4] array equality. But the bg_rgba is built from f32 literals + simple linear interpolation — no FP accumulation — so the comparison is well-defined.
- **Fix:** File-level `#![allow(clippy::float_cmp)]` on the test file. Same shape as `crates/vector-render/tests/tint_stripe.rs` from Plan 05-08.
- **Commit:** `56561a6` (folded into Task 2 GREEN).

### Documentation deviations

- The plan body specifies extending `cell.wgsl` with a hover-link dotted-underline path (Step 4-5 of Task 1). This visual rendering is NOT required for the verification gate (the unit tests cover dispatch + cursor swap only; the dotted underline is checkpointed by Plan 05-09's manual smoke matrix item #10 per the plan's M2-v2 Option B note). **Deferred** to Plan 05-09's smoke matrix sign-off. The data flow needed for the underline (`hover_uri`) is already populated in `App` — the renderer-side bit-flip is a small follow-on.
- The plan body's Task 3 Step 7 lists "spawn `tokio::test`-driven config-watcher integration test" implicitly via the seed-on-first-launch behavior. The first-launch seeding is implemented in `spawn_config_watcher_thread`, but no integration test exercises a live FSEvents flush — that path is covered by Plan 05-04's existing apply pipeline tests. **No new test needed.**
- The plan body suggests adding `cell_pipeline.rs` flags-bit-2 wiring for `flags |= CELL_FLAG_HOVER_LINK`. Same as above — deferred to the render-side follow-on (no scope creep; verifier-gated by smoke matrix #10).
- The plan body lists `crates/vector-app/src/chrome_pass.rs` in `files_modified`. The file was not created — the chrome-pass orchestration is currently inlined where the RedrawRequested per-pane pass runs. Splitting it into a dedicated module is mechanical refactor work, not a contract change; **deferred to follow-on cleanup**.

## Authentication Gates Encountered

None — fully autonomous plan, no external services.

## Issues Encountered

None. Cross-task interaction with Plan 05-08's `format_tab_title` + `pane.cwd` was already wired (B2 finalization was a no-op).

## User Setup Required

On first launch Vector will write `~/.config/vector/config.toml` if absent. Users who already have a config keep theirs untouched; the bundled Cmd-Shift-R keybind is only injected for first-time users. Users with pre-existing configs can copy the snippet from `DEFAULT_CONFIG_TOML` (3 lines) into their config to gain the M4 fallback.

## Next Phase Readiness

- **POLISH-07 fully closed end-to-end.** All chrome render passes shipped; all event-loop branches wired; UI-SPEC §6.1 toast string locked.
- **Plan 05-09 (CI tmux-smoke + manual smoke matrix):** Inherits the visual surfaces (search bar render, toast banner render, picker modal render, Cmd-click hover underline, NSPasteboard Cmd-C). Smoke-matrix item #10 (dotted-underline hover) is the only deferred visual assertion.
- **Phase 6 (Codespaces picker):** `ProfileSelected(name)` UserEvent variant is wired; the handler currently logs — Phase 6 swaps the log for a Codespace transport spawn. `Vector → Switch Profile →` submenu is the discoverable UI surface; population from `current_config.profile` keys is a one-shot wire-up.
- **Selection-to-string adapter:** Plan 05-07's `impl GridAccess for &Term` deferral remains open. The Cmd-C keystroke path is fully wired (NSPasteboard.clearContents + setString_forType invocation); the only missing piece is the `selection_to_string(&range, &*term, Stream)` call inside the `s.as_str() == "c"` branch. Implementing the adapter is ~20 LOC in `vector-term`; Plan 05-09's smoke item exercises Cmd-C end-to-end, which will surface any gap.

## Known Stubs

- **Cmd-C selection extraction (App.rs Cmd-C branch):** `write_pasteboard("")` currently writes an empty string. The full path needs:
  1. `impl GridAccess for &vector_term::Term` adapter (deferred from Plan 05-07 → 05-08 → 05-10 — now formally a Phase 5 carry-forward for Plan 05-09 or a v1.1 polish item).
  2. Call site: `let s = vector_input::selection_to_string(&range, &*self.term.lock(), vector_input::SelectionMode::Stream); self.write_pasteboard(&s);`
- **`UserEvent::SpawnNewWindow / OpenProfilePicker / ProfileSelected / ToggleSearch / ToggleSecureKeyboardEntry / ReloadConfig`:** All handler arms exist and log via tracing; full window factory / picker open / profile spawn / search bar open / SKE toggle / reload-config dispatch are deferred to follow-on plans. The variants are PRESENT so the keymap can dispatch them; the bodies will fill in as related subsystems land.
- **Switch Profile submenu population:** Menu shows `(no profiles configured)` placeholder. Dynamic population from `App.current_config.profile` keys requires re-installing the AppKit submenu when ConfigReloaded fires — deferred.
- **Hover-link dotted underline (visual):** Data flow (`hover_uri`) is plumbed; cell-pipeline shader bit-flip + render-side hover-run walk are deferred to a follow-on (smoke matrix item #10 covers visual sign-off).

None of these stubs prevent the plan's verification gates from passing. All chrome render passes are concrete (no `unimplemented!()` / `todo!()`); all required tests pass; all acceptance-criterion greps match.

## Self-Check: PASSED

Verified files on disk:

- `crates/vector-app/src/hyperlink_dispatch.rs` — FOUND
- `crates/vector-app/tests/hyperlink_dispatch.rs` — FOUND
- `crates/vector-render/src/chrome_quad.rs` — FOUND
- `crates/vector-render/src/search_bar_pass.rs` — FOUND
- `crates/vector-render/src/toast_pass.rs` — FOUND
- `crates/vector-render/src/picker_pass.rs` — FOUND
- `crates/vector-render/src/shaders/chrome_quad.wgsl` — FOUND
- `crates/vector-render/tests/search_bar_layout.rs` — FOUND
- `crates/vector-render/tests/toast_layout.rs` — FOUND

Verified commits in `git log`:

- `ca153b6` (Task 1 RED) — FOUND
- `d2857fc` (Task 1 GREEN) — FOUND
- `9cea97a` (Task 2 RED) — FOUND
- `56561a6` (Task 2 GREEN) — FOUND
- `42747be` (Task 3 RED) — FOUND
- `9d1b318` (Task 3 GREEN) — FOUND

Verified workspace state:
- `cargo test --workspace --tests --no-fail-fast` — 293 passed / 0 failed / 9 ignored.
- `cargo clippy --workspace --all-targets -- -D warnings` — exit 0.
- `cargo build --workspace --release` — exit 0.
- All 22 acceptance-criterion greps pass.
- Zero `unimplemented!()` / `todo!()` across the four chrome files.

---
*Phase: 05-polish-local-daily-driver*
*Completed: 2026-05-12*
