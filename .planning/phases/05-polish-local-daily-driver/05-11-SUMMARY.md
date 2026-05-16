---
phase: 05-polish-local-daily-driver
plan: 11
subsystem: ui
tags: [appkit, ns-pasteboard, ns-menu, objc2, once-lock, selection, gap-closure]

# Dependency graph
requires:
  - phase: 05-polish-local-daily-driver
    provides: vector_input::selection_to_string + GridAccess trait (Plan 05-06), vector-app NSPasteboard FFI + Switch Profile NSMenu install (Plan 05-10), POLISH-07 config watcher pumping UserEvent::ConfigReloaded (Plan 05-04/05-08)
provides:
  - "Cmd-C writes the real, wide-char-aware selection string to NSPasteboard"
  - "Switch Profile submenu rebuilds atomically on every UserEvent::ConfigReloaded"
  - "TermGridAccess newtype in vector-app — GridAccess impl avoiding crate cycle (B1)"
  - "OnceLock<MainThreadOnly<Retained<NSMenu>>> direct submenu reference (MEDIUM-4)"
affects: [05-14 (App-shortcut dispatch), 05-16 (final smoke + acceptance)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "MainThreadOnly<T> Sync-asserting newtype for AppKit handles in statics (alternative to dispatch2::MainThreadBound)"
    - "Newtype-in-consumer pattern to break trait crate cycles (TermGridAccess in vector-app, not vector-term)"

key-files:
  created:
    - crates/vector-app/src/term_grid_access.rs
    - crates/vector-app/tests/cmd_c_selection.rs
    - crates/vector-app/tests/switch_profile_menu.rs
  modified:
    - crates/vector-term/src/term.rs
    - crates/vector-app/src/lib.rs
    - crates/vector-app/src/app.rs
    - crates/vector-app/src/menu.rs

key-decisions:
  - "GridAccess impl lives in vector-app (not vector-term) to keep vector-term independent of vector-input — avoids cycle"
  - "MainThreadOnly<T> wrapper to satisfy OnceLock<...>: Sync without depending on dispatch2::MainThreadBound"
  - "Add EncodedKey::App(_) no-op match arm so the build stays green while Plan 05-14 lands the App-shortcut dispatch"

patterns-established:
  - "Trait adapters for foreign types should land in the consuming crate as newtypes to keep the producing crate free of dependencies"
  - "Module-level OnceLock + main-thread newtype lets AppKit Retained<...> handles park in statics without a menu-tree walk on rebuild"

requirements-completed: [POLISH-06, POLISH-07]

# Metrics
duration: ~20min
completed: 2026-05-13
---

# Phase 5 Plan 11: Cmd-C selection extraction + dynamic Switch Profile submenu Summary

**Cmd-C writes the live, wide-char-aware selection string to NSPasteboard via a TermGridAccess newtype, and the Switch Profile submenu rebuilds dynamically on every ConfigReloaded through a direct OnceLock NSMenu reference (no mainMenu walk).**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-05-13T04:08Z
- **Completed:** 2026-05-13T04:27Z
- **Tasks:** 2
- **Files modified:** 4 (1 in vector-term, 3 in vector-app)
- **Files created:** 3 (term_grid_access.rs, cmd_c_selection.rs, switch_profile_menu.rs)

## Accomplishments

- **POLISH-06 (gap #5 closed):** Cmd-C now copies the real selection. `Term::cell_at` + `Term::grid_cols` expose char + `WIDE_CHAR_SPACER` flag; `TermGridAccess` (newtype in vector-app) implements `vector_input::GridAccess`; `selection_to_string` is called in Stream mode and the result is written to `NSPasteboard`. The previous `write_pasteboard("")` placeholder is gone. Three integration tests cover basic word, trailing-whitespace strip, and Pitfall 8 wide-char spacer skip.
- **POLISH-07 (gap #6 closed):** `Switch Profile` submenu rebuilds dynamically. `submenu_rows_for(cfg)` produces alphabetical `(label, enabled)` rows (Local enabled; Codespace/DevTunnel suffixed `(phase 6+)` and disabled per UI-SPEC §6.4). `rebuild_switch_profile_submenu(mtm, cfg)` drains and repopulates the live NSMenu via the `SWITCH_PROFILE_SUBMENU` OnceLock — no `NSApplication.mainMenu` walk. The `UserEvent::ConfigReloaded` arm invokes the rebuild on the main thread.
- **MEDIUM-4 invariant satisfied:** `SWITCH_PROFILE_SUBMENU: OnceLock<MainThreadOnly<Retained<NSMenu>>>` is set exactly once inside `add_switch_profile_submenu` at install time and read directly by the rebuild path. The fragile `mainMenu().itemAtIndex(0)` + title-string walk is gone.
- **B1 invariant satisfied:** No crate dependency cycle. `TermGridAccess` is defined in `vector-app` (which already depends on both `vector-term` and `vector-input`), so `vector-term` never imports `vector-input`.

## Task Commits

1. **Task 1: TermGridAccess wrapper + Cmd-C real selection extraction** — `fdba618` (feat)
2. **Task 2 (RED): failing tests for submenu_rows_for** — `fd48787` (test)
3. **Task 2 (GREEN): submenu_rows_for + rebuild_switch_profile_submenu via OnceLock** — `113916a` (feat)

## Files Created/Modified

- `crates/vector-term/src/term.rs` — added `Term::cell_at(row, col) -> Option<(char, bool)>` and `Term::grid_cols() -> usize`.
- `crates/vector-app/src/term_grid_access.rs` (new) — `pub struct TermGridAccess<'a>(pub &'a Term)` + `impl GridAccess`.
- `crates/vector-app/src/lib.rs` — exposes `pub mod term_grid_access;`.
- `crates/vector-app/src/app.rs` — Cmd-C arm uses `selection_to_string` over `TermGridAccess`; `UserEvent::ConfigReloaded` calls `menu::rebuild_switch_profile_submenu`; new `Some(EncodedKey::App(_)) => {}` placeholder arm (Plan 05-13 introduced the variant, Plan 05-14 will wire dispatch).
- `crates/vector-app/src/menu.rs` — added `submenu_rows_for`, `rebuild_switch_profile_submenu`, `SWITCH_PROFILE_SUBMENU` static, `MainThreadOnly<T>` Sync wrapper, set-on-install in `add_switch_profile_submenu`; removed the `(no profiles configured)` static placeholder row.
- `crates/vector-app/tests/cmd_c_selection.rs` (new) — three integration tests for the Stream-mode `selection_to_string` path against a real `Term`.
- `crates/vector-app/tests/switch_profile_menu.rs` (new) — three tests for `submenu_rows_for` covering three-profile sort, empty config, and `kind`-less (defaults to enabled).

## Decisions Made

- **`MainThreadOnly<T>` newtype instead of `dispatch2::MainThreadBound`.** The plan literally specified `OnceLock<Retained<NSMenu>>`, but `Retained<NSMenu>` is `!Sync` (AppKit objects are not thread-safe). Two options: (a) pull in `dispatch2` for `MainThreadBound`; (b) define a one-screen newtype with `unsafe impl Sync` justified by the `MainThreadMarker` gate at every callsite. Chose (b) — smaller dependency surface, identical safety story, and the static lives in just one module.
- **`EncodedKey::App(_)` placeholder arm.** Parallel Plan 05-13 added the variant ahead of Plan 05-14's dispatch wiring. Without a match arm, this plan would not compile. Added a `Some(EncodedKey::App(_)) => {}` no-op with a comment pointing at Plan 05-14; this is a Rule-3 (blocking) deviation, not scope creep.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Add `Some(EncodedKey::App(_)) => {}` match arm in `app.rs`**
- **Found during:** Task 1 (first build after wiring Cmd-C)
- **Issue:** Parallel Plan 05-13 committed `EncodedKey::App(AppShortcut)` to `vector-input/keymap.rs` before this plan ran, but app.rs's `match encode_key(...)` is non-exhaustive. Compilation failed with E0004.
- **Fix:** Added `Some(EncodedKey::App(_)) => {}` — no-op until Plan 05-14 wires the App-shortcut dispatch. The arm has an inline comment pointing at Plan 05-14.
- **Files modified:** `crates/vector-app/src/app.rs`
- **Verification:** `cargo build -p vector-app` succeeds; `cargo test -p vector-app` passes.
- **Committed in:** `fdba618` (Task 1 commit)

**2. [Rule 1 - Bug] `MainThreadOnly<T>` wrapper required for `OnceLock<Retained<NSMenu>>: Sync`**
- **Found during:** Task 2 GREEN step
- **Issue:** Plan signature specified `static SWITCH_PROFILE_SUBMENU: OnceLock<Retained<NSMenu>>`, but `Retained<NSMenu>` is `!Sync` (AppKit objects are not thread-safe). `static` items require `Sync`, so compilation failed.
- **Fix:** Defined `struct MainThreadOnly<T>(T)` with `unsafe impl Sync for MainThreadOnly<T>` justified by the `MainThreadMarker` gate at every accessor. Storage now `OnceLock<MainThreadOnly<Retained<NSMenu>>>`. All MEDIUM-4 acceptance greps (`static SWITCH_PROFILE_SUBMENU: OnceLock`, `SWITCH_PROFILE_SUBMENU.set(`, `SWITCH_PROFILE_SUBMENU.get()`, no menu walk) still pass.
- **Files modified:** `crates/vector-app/src/menu.rs`
- **Verification:** Test passes; build clean; the must_haves invariant ("submenu's NSMenu is stored at install time in a module-level OnceLock") still holds — the inner `Retained<NSMenu>` is the genuine submenu handle.
- **Committed in:** `113916a` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 Rule 3 blocking, 1 Rule 1 type-system bug)
**Impact on plan:** Both auto-fixes were strictly necessary to compile. No scope creep. The MEDIUM-4 invariant is preserved; the static-Sync requirement was a planning oversight resolved with a one-screen wrapper.

## Issues Encountered

- Parallel Plan 05-13 landed the `EncodedKey::App` variant mid-execution. Handled as a Rule-3 blocking deviation with a no-op match arm; Plan 05-14 will replace it with real dispatch.

## Self-Check

- `crates/vector-app/src/term_grid_access.rs`: **FOUND**
- `crates/vector-app/tests/cmd_c_selection.rs`: **FOUND** (3/3 passing)
- `crates/vector-app/tests/switch_profile_menu.rs`: **FOUND** (3/3 passing)
- Commit `fdba618`: **FOUND** in `git log`
- Commit `fd48787`: **FOUND** in `git log`
- Commit `113916a`: **FOUND** in `git log`
- Acceptance greps: ALL PASS
  - `impl.*GridAccess.*for.*TermGridAccess` — match in `term_grid_access.rs`
  - `selection_to_string` — match in `app.rs`
  - `pub mod term_grid_access` — match in `lib.rs`
  - `write_pasteboard\(""\)` — zero matches
  - `pub fn submenu_rows_for` — match in `menu.rs`
  - `pub unsafe fn rebuild_switch_profile_submenu` — match in `menu.rs`
  - `static SWITCH_PROFILE_SUBMENU: OnceLock` — match in `menu.rs`
  - `SWITCH_PROFILE_SUBMENU.set(` — match in `add_switch_profile_submenu`
  - `SWITCH_PROFILE_SUBMENU.get()` — match in `rebuild_switch_profile_submenu`
  - `mainMenu\(\).*itemAtIndex\(0\)` — zero matches (no walk)
  - `rebuild_switch_profile_submenu` — match in `app.rs` (ConfigReloaded arm)
  - `"\(no profiles configured\)"` — zero matches
- Workspace release build: **CLEAN**

## Self-Check: PASSED

## Manual Reproduce

- **Cmd-C:** launch Vector → type `hello world` → mouse-select `hello` → Cmd-C → paste into another app → expect exactly `hello` (no trailing blanks, no NUL bytes).
- **Switch Profile submenu:** write `~/.config/vector/config.toml` with three profiles (one each of `kind = "local"`, `"codespace"`, `"dev-tunnel"`) → save while Vector is running → open Vector menu → Switch Profile → expect exactly three rows in alphabetical order; Codespace and DevTunnel rows show `(phase 6+)` suffix and are disabled; Local row is enabled.

## Next Phase Readiness

- Two gap-closure items shipped; no blockers for the remaining Phase 5 polish plans.
- Plan 05-14 should remove the placeholder `Some(EncodedKey::App(_)) => {}` arm and route to the App-shortcut handlers (D-69 / D-75 / D-76 / D-82).
- Plan 05-16 final smoke matrix should include the two manual reproduce steps above.

---
*Phase: 05-polish-local-daily-driver*
*Completed: 2026-05-13*
