---
phase: 05-polish-local-daily-driver
plan: 14
subsystem: ui
tags: [rust, winit, objc2-app-kit, appkit, search-bar, profile-picker, shortcut-dispatch]

requires:
  - phase: 05-13
    provides: AppShortcut enum + EncodedKey::App variant in vector-input
  - phase: 05-11
    provides: submenu_rows_for + rebuild_switch_profile_submenu + SearchBar + ProfilePicker structs
  - phase: 05-15
    provides: App.ime field + set_ime_allowed for bootstrap + handle_new_tab sites

provides:
  - App.search_bar: SearchBar field (gap-closure for gap #3)
  - App.profile_picker: ProfilePicker field (gap-closure for gap #3)
  - WinitWindowFactory::create_ungrouped via NSWindowTabbingModeDisallowed (MEDIUM-1)
  - App::handle_app_shortcut — real state mutations for all 4 chrome shortcuts
  - EncodedKey::App dispatch arm in window_event KeyboardInput
  - do_toggle_search / do_open_profile_picker / do_reload_config private helpers
  - set_ime_allowed(true) on SpawnNewWindow site (MEDIUM-3)
  - LOW-3 idempotency test: switch_profile_menu_idempotent.rs

affects:
  - phase-05-16: Plan 05-16's SearchBarPass + PickerPass render passes need app.search_bar.open + app.profile_picker.open to be true to render

tech-stack:
  added: []
  patterns:
    - handle_app_shortcut delegates to do_* private helpers — both keyboard and UserEvent paths share one implementation
    - WinitWindowFactory::create_ungrouped uses NSWindowTabbingMode::Disallowed (objc2-app-kit) to prevent AppKit tab grouping — no uuid dep, no counter
    - do_reload_config reads disk via vector_config::parse and rebuilds submenu idempotently (LOW-3 proven by test)

key-files:
  created:
    - crates/vector-app/tests/app_default_fields.rs
    - crates/vector-app/tests/handler_state_mutation.rs
    - crates/vector-app/tests/spawn_new_window.rs
    - crates/vector-app/tests/switch_profile_menu_idempotent.rs
  modified:
    - crates/vector-app/src/app.rs
    - crates/vector-app/src/mux_commands.rs

key-decisions:
  - "MEDIUM-1 locked: create_ungrouped uses setTabbingMode:NSWindowTabbingModeDisallowed via objc2-app-kit NSWindowTabbingMode::Disallowed — no uuid dep, no AtomicUsize counter"
  - "MEDIUM-3 ownership: SpawnNewWindow set_ime_allowed(true) site is owned by Plan 05-14 (NOT 05-15); both keyboard and UserEvent::SpawnNewWindow paths call handle_app_shortcut which calls set_ime_allowed"
  - "LOW-3 idempotency: submenu_rows_for proven referentially transparent by switch_profile_menu_idempotent.rs; FSEvents + Cmd-Shift-R concurrent invocations are safe"
  - "v1 simplification: Cmd-N spawns an ungrouped NSWindow but does NOT auto-spawn a PTY; user presses Cmd-T to get a shell (deferred B3)"
  - "W8 deviation from D-76 (additive): ToggleSearch arm closes on second Cmd-F press in addition to Esc; Esc still closes as D-76 requires"

patterns-established:
  - "handle_app_shortcut pattern: pub(crate) method delegates to do_* helpers; both EncodedKey::App keyboard and UserEvent::* menu paths share one implementation body"

requirements-completed: [POLISH-01, POLISH-06, POLISH-07]

duration: 17min
completed: 2026-05-14
---

# Phase 05 Plan 14: Handler bodies + App fields for chrome shortcuts

**Real state mutations wired for all four App chrome shortcuts: SearchBar toggle, ProfilePicker open with entries, ungrouped NSWindow via NSWindowTabbingModeDisallowed, config disk-reload with submenu rebuild**

## Performance

- **Duration:** 17 min
- **Started:** 2026-05-13T21:24:52Z
- **Completed:** 2026-05-14T14:28:48Z
- **Tasks:** 2 (both TDD with RED + GREEN commits)
- **Files modified:** 6 (2 src + 4 test files)

## Accomplishments

- Added `search_bar: SearchBar` and `profile_picker: ProfilePicker` fields to `App` struct, closing gap #3 from 05-VERIFICATION.md
- Added `WinitWindowFactory::create_ungrouped` using `NSWindowTabbingMode::Disallowed` (objc2-app-kit typed binding — MEDIUM-1 fix, no uuid dep, no AtomicUsize counter)
- Added `App::handle_app_shortcut` with real state mutations for all four shortcuts, replacing the four `tracing::info!` placeholder arms
- Wired `EncodedKey::App(shortcut)` dispatch arm in `window_event` KeyboardInput match
- Added `set_ime_allowed(true)` on the SpawnNewWindow site (MEDIUM-3 — this plan owns it; Plan 05-15 cannot reach a branch that doesn't exist when 05-15 runs)
- Proved `submenu_rows_for` is referentially transparent via `switch_profile_menu_idempotent.rs` (LOW-3)

## Task Commits

1. **Task 1 RED: App fields test** - `2fd231f` (test)
2. **Task 1 GREEN: App fields + create_ungrouped** - `ab49352` (feat)
3. **Task 2 RED: handler tests** - `246271e` (test)
4. **Task 2 GREEN: handle_app_shortcut + dispatch** - `6672a6b` (feat)

## Files Created/Modified

- `crates/vector-app/src/app.rs` - Added search_bar/profile_picker fields, handle_app_shortcut, do_toggle_search, do_open_profile_picker, do_reload_config, test accessors, EncodedKey::App dispatch arm, replaced UserEvent stub arms
- `crates/vector-app/src/mux_commands.rs` - Added apply_tabbing_mode_disallowed helper + WinitWindowFactory::create_ungrouped
- `crates/vector-app/tests/app_default_fields.rs` - App field default-value tests (Task 1 RED)
- `crates/vector-app/tests/handler_state_mutation.rs` - 3 tests for ToggleSearch/OpenProfilePicker mutations (Task 2 RED)
- `crates/vector-app/tests/spawn_new_window.rs` - Cmd-N EncodedKey regression test (Task 2)
- `crates/vector-app/tests/switch_profile_menu_idempotent.rs` - LOW-3 idempotency proof (Task 2)

## Decisions Made

- **MEDIUM-1 approach locked:** `NSWindowTabbingMode::Disallowed` via objc2-app-kit typed binding, mirroring `set_tabbing_mode_preferred` pattern. No uuid dep. No AtomicUsize counter. The reviewer's exact recommendation.
- **MEDIUM-3 ownership:** This plan owns `set_ime_allowed(true)` on the `SpawnNewWindow` site. Both keyboard (`handle_app_shortcut`) and `UserEvent::SpawnNewWindow` menu paths call `handle_app_shortcut`, so both get IME enabled on the new window.
- **handle_app_shortcut design:** `pub(crate)` method dispatches to `do_*` private helpers. Both the `EncodedKey::App` keyboard path and the `UserEvent::*` menu path share a single implementation — no code duplication, no roundtrip latency.
- **ToggleSearch toggle behavior (W8 additive deviation):** Second Cmd-F press closes the bar in addition to Esc. D-76 mandates "Esc closes" which is still satisfied. Toggle-on-second-press is additive convenience. Documented per plan's `<deferred>` W8 note.

## Deviations from Plan

None — plan executed exactly as written. The MEDIUM-1, MEDIUM-3, and LOW-3 reviewer fixes were incorporated as specified. The W8 toggle behavior was anticipated and documented in the plan's `<deferred>` block.

## Deferred Items (from plan)

**B3 — SpawnNewWindow PTY spawn deferred.** The `AppShortcut::SpawnNewWindow` handler creates a fresh ungrouped NSWindow via `create_ungrouped` but does NOT spawn a `[default]` profile PTY at `$HOME` inside it. The new window appears without a shell; the user presses Cmd-T to spawn one. Spawning a real `[default]` shell at `$HOME` inside the new window is tracked for a future plan (Phase 5 tail or Phase 6 alongside codespace transport — both need a "spawn shell in window X" abstraction).

**MEDIUM-3 IME count:** `grep -c "set_ime_allowed(true)" crates/vector-app/src/app.rs` returns 4 (not 3). The 4th is a comment reference. Actual call sites: bootstrap (`resumed`), `handle_new_tab`, and `handle_app_shortcut` SpawnNewWindow arm. MEDIUM-3 satisfied.

## Manual Verification Steps

1. `cargo run -p vector-app` — launch Vector
2. **Cmd-F** → `tracing` log shows `search_bar.open == true` (no visual yet — Plan 05-16)
3. **Cmd-F again** → `search_bar.open` flips to `false` (toggle behavior, W8)
4. **Cmd-Shift-P** → `tracing` log shows `profile_picker.open == true` with entries from config
5. **Cmd-N** → A SECOND ungrouped NSWindow appears (not a tab — NSWindowTabbingModeDisallowed)
6. **Cmd-Shift-R** → Toast "config reloaded" or "config error: ..." + Switch Profile submenu rebuilds

## Known Stubs

- `AppShortcut::SpawnNewWindow`: new window has no PTY until user presses Cmd-T (v1 simplification, B3 deferred above)
- `SearchBar` / `ProfilePicker` open state has no render pass yet — Plan 05-16 adds SearchBarPass / PickerPass overlays

## Self-Check: PASSED
