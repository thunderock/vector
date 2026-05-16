---
phase: 05-polish-local-daily-driver
plan: 13
subsystem: vector-input::keymap
tags: [chrome-shortcuts, keymap, encoded-key, gap-closure]
gap_closure: true
requirements: [POLISH-06, POLISH-07, POLISH-08]
dependency_graph:
  requires:
    - "vector-input::keymap (encode + match_mux_command from Plan 04-04)"
    - "vector-input::ModState (cmd/shift/alt/ctrl flags)"
  provides:
    - "EncodedKey::App(AppShortcut) variant"
    - "AppShortcut enum (SpawnNewWindow/ToggleSearch/OpenProfilePicker/ReloadConfig)"
    - "match_app_shortcut() — pure-data keymap entry called between Mux and PTY"
  affects:
    - "vector-app::encode_key match arm (placeholder absorbed by Plan 05-11 in parallel; real wiring lands in Plan 05-14)"
tech_stack:
  added: []
  patterns:
    - "Precedence chain in encode(): match_mux_command → match_app_shortcut → encode_pty"
    - "Pure-data keymap layer: NO App-side handlers wired in this plan (Plan 05-14 owns that)"
key_files:
  created:
    - crates/vector-input/tests/chrome_shortcuts.rs
  modified:
    - crates/vector-input/src/keymap.rs
    - crates/vector-input/src/lib.rs
decisions:
  - "Match precedence: chrome shortcuts (Cmd-N/F/Shift-P/Shift-R) recognized AFTER Mux (Cmd-T/D/W/Shift-D/Shift-]/Shift-[) but BEFORE encode_pty — prevents PTY byte leakage while preserving Phase-4 mux behavior."
  - "AppShortcut enum is Copy + PartialEq + Eq — matches MuxCommand for pattern-match ergonomics in vector-app."
  - "match_app_shortcut accepts both shifted and unshifted character forms (\"P\"|\"p\", \"R\"|\"r\") to handle macOS sending the shifted glyph when Shift is held — mirrors character_shortcut() pattern from Plan 04-04."
metrics:
  duration_min: 2.3
  tasks: 1
  commits: 2
  files_changed: 3
  completed_at: "2026-05-13T04:22:07Z"
---

# Phase 05 Plan 13: Chrome Shortcut Keymap Entries Summary

**One-liner:** Add `EncodedKey::App(AppShortcut)` variant + `match_app_shortcut` in vector-input::keymap so Cmd-N/F/Shift-P/Shift-R produce app-shortcut events with zero PTY byte leakage; downstream wiring deferred to Plan 05-14.

## What Shipped

### vector-input::keymap

- **`AppShortcut` enum** (Copy/PartialEq/Eq): `SpawnNewWindow` (Cmd-N → D-82), `ToggleSearch` (Cmd-F → D-76), `OpenProfilePicker` (Cmd-Shift-P → D-75), `ReloadConfig` (Cmd-Shift-R → D-69 menu fallback).
- **`EncodedKey::App(AppShortcut)` variant** added to the existing `Pty(Vec<u8>)`/`Mux(MuxCommand)` enum.
- **`match_app_shortcut(key, mods)`** — gated on `mods.cmd && !mods.ctrl && !mods.alt`; reads `Key::Character(s)` and dispatches on `s` with the Shift branch routed first. Returns `None` if the key isn't a chrome binding.
- **`encode()` precedence chain extended:** `match_mux_command` → `match_app_shortcut` → `encode_pty`. The Mux check still runs first, so all Phase-4 shortcuts continue unchanged (verified by regression tests).

### vector-input::lib

- Re-export updated: `pub use keymap::{encode, encode_key, AppShortcut, EncodedKey, MuxCommand};`

### tests/chrome_shortcuts.rs (new — 7 tests)

| Test                                    | Asserts                                                  |
| --------------------------------------- | -------------------------------------------------------- |
| `cmd_n_spawns_new_window`               | `Cmd-N` → `EncodedKey::App(SpawnNewWindow)`              |
| `cmd_f_toggles_search`                  | `Cmd-F` → `EncodedKey::App(ToggleSearch)`                |
| `cmd_shift_p_opens_profile_picker`      | `Cmd-Shift-P` and `Cmd-Shift-p` → `OpenProfilePicker`    |
| `cmd_shift_r_reloads_config`            | `Cmd-Shift-R` and `Cmd-Shift-r` → `ReloadConfig`         |
| `plain_n_still_goes_to_pty`             | `n` (no mods) → `EncodedKey::Pty(b"n")`                  |
| `cmd_t_still_returns_mux_new_tab`       | Phase-4 regression: `Cmd-T` → `MuxCommand::NewTab`       |
| `cmd_shift_d_still_returns_mux_split_vertical` | Phase-4 regression: `Cmd-Shift-D` → `MuxCommand::SplitVertical` |

Each App-shortcut test implicitly asserts ZERO bytes leak to PTY: the returned `EncodedKey` is `App(_)`, never `Pty(_)`, so `app.rs::send_bytes` is never reached for these keys.

## Verification

- `cargo test -p vector-input --test chrome_shortcuts` → 7/7 pass
- `cargo test -p vector-input` (full suite) → 100/100 pass (no Phase-4 regression on xterm_key_table, bracketed_paste_wrap, clipboard, no_tokio_main, selection_string)
- `cargo build -p vector-input --release` → clean
- `cargo clippy -p vector-input --all-targets -- -D warnings` → exit 0
- `cargo build -p vector-app` → clean (placeholder arm already present from Plan 05-11; see LOW-1 below)

## Acceptance Criteria

| Criterion                                                                                          | Status |
| -------------------------------------------------------------------------------------------------- | ------ |
| `grep "pub enum AppShortcut"` returns a match                                                      | PASS (line 40)  |
| `grep "EncodedKey::App"` returns ≥2 matches (variant decl + return site)                           | PASS (variant `App(AppShortcut)` line 21 + return site line 78) |
| All four `AppShortcut::*` variants present                                                         | PASS (grep -c = 4) |
| `cargo test -p vector-input --test chrome_shortcuts` passes 7/7                                    | PASS   |
| `cargo test -p vector-input` (full suite) passes, no Phase-4 regression                            | PASS (100/100) |
| `cargo build -p vector-input --release` succeeds                                                   | PASS   |
| LOW-1 warning-count invariant on `cargo build -p vector-app`                                       | DIVERGED — see below |
| For each of Cmd-N/F/Shift-P/Shift-R: encode returns `EncodedKey::App(...)`, never reaches encode_pty fallthrough | PASS (proved by tests + match precedence audit) |

## LOW-1 Compile-Warning Invariant — Divergence

**Expected:** exactly 1 `non-exhaustive patterns` warning on `cargo build -p vector-app`, closed by Plan 05-14.

**Actual:** 0 warnings, 0 errors. `cargo build -p vector-app` is clean.

**Why:** This plan ran in parallel with Plan 05-11. Plan 05-11's executor pre-emptively added a placeholder arm to `crates/vector-app/src/app.rs:843-846`:

```rust
// Plan 05-11 Rule-3 deviation: Plan 05-13 added EncodedKey::App;
// Plan 05-14 wires the App-shortcut dispatch. Until then, ignore
// so the build stays green.
Some(EncodedKey::App(_)) => {}
```

This is the exact "0 means an arm was added prematurely" path that LOW-1 contemplates. The cause is benign — parallel executors cannot coordinate, and Plan 05-11 needed vector-app to compile to land its own changes. Plan 05-14 will replace the empty body with real dispatch to `UserEvent::{SpawnNewWindow, ToggleSearch, OpenProfilePicker, ReloadConfig}`, so the placeholder is short-lived.

**Note (not a deviation in this plan):** Rust treats non-exhaustive `match` on a non-`#[non_exhaustive]` enum as a hard ERROR (E0004), never a warning. Without Plan 05-11's placeholder, `cargo build -p vector-app` would have failed compilation outright, not emitted a warning. The plan's LOW-1 invariant was framed as "warning count" but the underlying mechanism is "error count" — the spirit of the invariant (one observable diagnostic until 05-14 closes the arm) holds, just at the error level rather than warning level. Plan 05-14's acceptance criteria should drop the `non-exhaustive patterns` count from 0 to 0 (no change), while ADDING real handler bodies.

## Deviations from Plan

### Auto-fixed Issues

None. Plan 05-13 executed exactly as written.

### Documented Divergences

**1. LOW-1 warning-count is 0, not 1.** Caused by Plan 05-11's parallel-execution placeholder arm in app.rs. Documented above. No code change required in this plan.

## Forward Dependency Notes

**Plan 05-14 (Wave 4, follows this plan and 05-11):**
- Will replace `Some(EncodedKey::App(_)) => {}` placeholder in `vector-app/src/app.rs:843-846` with a real match arm:
  ```rust
  Some(EncodedKey::App(AppShortcut::SpawnNewWindow))    => self.handle_spawn_new_window(event_loop),
  Some(EncodedKey::App(AppShortcut::ToggleSearch))      => self.handle_toggle_search(id),
  Some(EncodedKey::App(AppShortcut::OpenProfilePicker)) => self.handle_open_profile_picker(id),
  Some(EncodedKey::App(AppShortcut::ReloadConfig))      => self.handle_reload_config(),
  ```
- Will import `AppShortcut` alongside the existing `EncodedKey, MuxCommand` imports at `vector-app/src/app.rs:9`.
- Will route to existing `UserEvent` variants (already declared in `vector-app/src/lib.rs`).

**Plan 05-11 (parallel):** Already absorbed `EncodedKey::App(_)` via placeholder; will not regress when 05-14 closes the arm.

## Commits

| Commit  | Type | Description                                              |
| ------- | ---- | -------------------------------------------------------- |
| 8b9b855 | test | RED — failing chrome shortcut tests (7 tests)            |
| 1a67085 | feat | GREEN — AppShortcut + EncodedKey::App + match_app_shortcut |

## Self-Check: PASSED

- `crates/vector-input/tests/chrome_shortcuts.rs` — FOUND
- `crates/vector-input/src/keymap.rs` (AppShortcut + EncodedKey::App) — FOUND
- `crates/vector-input/src/lib.rs` (re-export) — FOUND
- commit `8b9b855` — FOUND
- commit `1a67085` — FOUND
