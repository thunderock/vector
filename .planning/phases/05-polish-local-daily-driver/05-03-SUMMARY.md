---
phase: 05-polish-local-daily-driver
plan: 03
subsystem: theme
tags: [palette, itermcolors, plist, appearance, chrome-tokens, polish-03, d-72, d-73, ui-spec-9]

# Dependency graph
requires:
  - phase: 05-polish-local-daily-driver
    provides: "vector-config::Appearance enum (System | Light | Dark) — pre-landed
      by this plan if 05-02 hadn't yet, otherwise consumed via re-export from
      vector_config::schema"
provides:
  - "vector-theme::Palette { ansi[16], fg, bg, cursor, selection, bold, chrome }"
  - "vector-theme::ChromePalette with 10 locked tokens per UI-SPEC §9.1"
  - "vector-theme::vector_dark() + vector_light() builtin palettes"
  - "vector-theme::parse_itermcolors(&[u8]) -> Result<Palette, ThemeError>"
  - "vector-theme::resolve_palette(Appearance, system_is_dark: bool) -> Palette (D-72 resolver)"
  - "UI-SPEC §9.2 contract: .itermcolors overlay never overrides chrome — chrome
    is always sourced from the active appearance's builtin"
affects:
  - "05-04 (toast surface) — consumes Palette.chrome.surface/divider/button/warning"
  - "05-07 (search bar) — consumes Palette.chrome.search_highlight + danger_subtle"
  - "05-08 (tint stripe + picker) — consumes Palette.chrome.selection + fg_muted"
  - "05-09 (IME preedit underline) — consumes Palette.chrome.fg_muted"
  - "vector-app (config-apply pipeline) — replaces result.chrome from parse_itermcolors
    with the active-appearance's chrome before paint"

# Tech tracking
tech-stack:
  added:
    - "plist 1.9 (workspace.dependencies; declared by Plan 05-01)"
  patterns:
    - "Chrome-token contract: chrome surface colors live in ChromePalette and are
      sourced from `resolve_palette(appearance, system_is_dark)` only — never from
      an imported .itermcolors plist (UI-SPEC §9.2)."
    - "Hex color literals: 24-bit `0xRRGGBB` is canonical in builtins; module-level
      `#![allow(clippy::unreadable_literal)]` because splitter underscores hurt
      readability of color values."
    - "f64 → u8 sRGB component conversion: clamp([0,1]) + *255 + .round() inside a
      scoped helper with `#[allow(clippy::cast_possible_truncation, cast_sign_loss)]`
      — truncation/sign-loss impossible after clamp."

key-files:
  created:
    - crates/vector-theme/src/palette.rs
    - crates/vector-theme/src/builtins.rs
    - crates/vector-theme/src/appearance.rs
    - crates/vector-theme/src/error.rs
    - crates/vector-theme/src/itermcolors.rs
    - crates/vector-theme/tests/builtins.rs
    - crates/vector-theme/tests/appearance.rs
    - crates/vector-theme/tests/itermcolors.rs
    - crates/vector-theme/tests/fixtures/Solarized-Dark.itermcolors
  modified:
    - crates/vector-theme/Cargo.toml
    - crates/vector-theme/src/lib.rs

key-decisions:
  - "ChromePalette derives Copy (10 small primitives); Palette is Clone-only (carries
    a 16-element Rgb array and an embedded ChromePalette — copy semantics would
    silently clone the array)."
  - "parse_itermcolors seeds the result palette from vector_dark() so the chrome
    field carries a valid default. vector-app's config-apply pipeline replaces
    result.chrome with the active appearance's chrome before paint (per UI-SPEC §9.2)."
  - "iTerm chrome-ish keys (Cursor Text / Selected Text / Tab / Underline / Link /
    Badge) are intentionally dropped at tracing::debug level — they would silently
    fight the chrome-token contract if respected."

patterns-established:
  - "Workspace path-deps with explicit `version =` (D-83 #2): vector-theme depends
    on vector-config via `path = \"../vector-config\", version = \"2026.5.10\"`."
  - "Test fixture XML plists checked in alongside `tests/` (`tests/fixtures/*.itermcolors`)
    and consumed via `include_bytes!`. `xmllint` + `plistlib` in CI validate fixture
    integrity at the acceptance-criteria level."

requirements-completed: [POLISH-03]

# Metrics
duration: 12min
completed: 2026-05-12
---

# Phase 5 Plan 03: vector-theme palette + builtins + .itermcolors + appearance Summary

**Locked the UI-SPEC §9 chrome-token contract — Vector Light/Dark builtins (10 chrome tokens each) + Solarized-Dark `.itermcolors` parser that respects the §9.2 "chrome stays from appearance, not plist" rule + pure-Rust D-72 appearance resolver.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-05-12T17:37:00Z (approx)
- **Completed:** 2026-05-12T17:49:00Z
- **Tasks:** 2 (TDD: 2 RED + 2 GREEN = 4 commits)
- **Files modified/created:** 11

## Accomplishments

- `Palette` / `ChromePalette` / `Rgb` / `Rgba` types ship with the exact UI-SPEC §9.1 contract — 10 chrome tokens (`surface`, `divider`, `button`, `button_hover`, `selection`, `search_highlight`, `warning`, `danger_subtle`, `link`, `fg_muted`).
- `vector_dark()` + `vector_light()` builtins with locked hex values: dark search-highlight `#ffd60a` (yellow), light `#ff9500` (orange); chrome surface alpha 230 (`0xe6`) on both.
- `resolve_palette(Appearance, system_is_dark)` honors D-72: `Light`/`Dark` are hard overrides; `System` flips on the `system_is_dark` bool (which vector-app sources from `NSApplication.effectiveAppearance`).
- `.itermcolors` parser maps 16 ANSI + Foreground/Background/Cursor/Selection/Bold; unknown keys + chrome-ish iTerm keys (`Tab Color`, `Link Color`, etc.) are skipped at `tracing::debug` so they cannot fight the chrome-token contract.
- UI-SPEC §9.2 chrome-not-overridden contract is asserted directly in `parses_full_scheme`: after parsing Solarized-Dark, `palette.chrome.search_highlight == vector_dark().chrome.search_highlight` and `palette.chrome.surface == vector_dark().chrome.surface`.

## Task Commits

Each task followed TDD (RED → GREEN):

1. **Task 1 RED: failing tests for builtins + appearance** — `3d6b1ac` (test)
2. **Task 1 GREEN: palette types + Vector Light/Dark + resolve_palette** — `dc578b1` (feat)
3. **Task 2 RED: failing tests for .itermcolors importer** — `51adfbf` (test)
4. **Task 2 GREEN: parse_itermcolors implementation** — `240f9da` (feat)

_Note: Plan 05-02 ran in parallel and landed `dac8f5c` (cargo-deny/CI bits) + `9649e7e` + `9bf3c8c` (vector-config loader/schema) between my commits — those are 05-02's scope, not mine. My commits target only `crates/vector-theme/**`._

## Files Created/Modified

**Created:**
- `crates/vector-theme/src/palette.rs` — Rgb, Rgba, Palette, ChromePalette types per UI-SPEC §9.1.
- `crates/vector-theme/src/builtins.rs` — vector_dark()/vector_light() with `#![allow(clippy::unreadable_literal)]` for hex color values.
- `crates/vector-theme/src/appearance.rs` — resolve_palette() over vector_config::Appearance.
- `crates/vector-theme/src/error.rs` — ThemeError wraps plist::Error + io::Error + NotADict + Field.
- `crates/vector-theme/src/itermcolors.rs` — parse_itermcolors with clamp-to-[0,1] + scoped f_to_u8 helper.
- `crates/vector-theme/tests/builtins.rs` — `builtins_loadable` test.
- `crates/vector-theme/tests/appearance.rs` — `dark_light_flip` test.
- `crates/vector-theme/tests/itermcolors.rs` — `parses_full_scheme` + `unknown_key_warns` tests.
- `crates/vector-theme/tests/fixtures/Solarized-Dark.itermcolors` — full canonical Solarized-Dark plist + one `Bogus Color` key for unknown-key path.

**Modified:**
- `crates/vector-theme/Cargo.toml` — added `plist.workspace = true` + `vector-config = { path = "../vector-config", version = "2026.5.10" }`.
- `crates/vector-theme/src/lib.rs` — wired module tree + re-exports.

## Decisions Made

- **Chrome-token bake-in:** Hardcoded the 10 chrome tokens per UI-SPEC §9.1 directly in `builtins.rs` rather than synthesizing from a smaller seed. Reason: UI-SPEC §9.1 explicitly enumerates each token by name and exact hex value; downstream chrome surfaces (toast, search bar, picker, tint stripe) need to grep this file as the single source of truth.
- **`Palette: Clone` only, `ChromePalette: Copy`:** Palette wraps a 16-element Rgb array (48 bytes) plus an embedded ChromePalette (~76 bytes). Marking it `Copy` would mean callers silently move ~124-byte values around; explicit `Clone` keeps the cost visible at call sites.
- **`parse_itermcolors` seeds chrome from `vector_dark()`:** This guarantees `Palette.chrome` is always populated even before vector-app's config-apply pipeline runs. The pipeline replaces `result.chrome` with the active-appearance's chrome before paint, so the seed value is never observed at runtime; it exists so the type system can't end up with a half-initialized Palette.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug / Tooling] clippy::unreadable_literal on 24-bit hex color literals**
- **Found during:** Task 1 verify (clippy gate after Task 1 GREEN)
- **Issue:** Workspace clippy.pedantic level fires `unreadable_literal` on every `0xRRGGBB` color literal in `builtins.rs` + tests/builtins.rs (12 lints in builtins.rs, 2 in the test). Clippy suggests `0x00ff_d60a`-style underscored variants.
- **Fix:** Added `#![allow(clippy::unreadable_literal)]` at module scope in both files. 24-bit hex color literals (`0xffd60a`) are canonical in UI-SPEC §9.1 and in the iTerm2 color-scheme ecosystem; splitter underscores hurt readability and make grep-by-color-value fail.
- **Files modified:** crates/vector-theme/src/builtins.rs, crates/vector-theme/tests/builtins.rs
- **Verification:** `cargo clippy -p vector-theme --all-targets -- -D warnings` exits 0.
- **Committed in:** dc578b1 (Task 1 GREEN commit)

**2. [Rule 1 - Bug / Tooling] clippy::cast_possible_truncation + cast_sign_loss on sRGB f64→u8 conversion**
- **Found during:** Task 2 verify (clippy gate after Task 2 GREEN)
- **Issue:** Workspace clippy.pedantic flags the three `(v.clamp(0.0, 1.0) * 255.0).round() as u8` casts inside `read_rgb()`. The casts are provably safe (input is clamped to `[0,1]`, output of `*255.0` is `[0,255]`, `.round()` cannot produce NaN), but clippy doesn't reason past the cast.
- **Fix:** Extracted a `fn f_to_u8(v: f64) -> u8` helper with scoped `#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]`. Pattern mirrors the helper-with-scoped-allow approach used in `vector-fonts` per STATE.md.
- **Files modified:** crates/vector-theme/src/itermcolors.rs
- **Verification:** `cargo clippy -p vector-theme --all-targets -- -D warnings` exits 0; both tests still pass.
- **Committed in:** 240f9da (Task 2 GREEN commit)

**3. [Rule 3 - Blocking] vector-config::Appearance dependency landed mid-plan**
- **Found during:** Task 1 (resolve_palette uses `vector_config::Appearance`)
- **Issue:** Plan 05-03's `depends_on: [05-01]` does not formally cover the `Appearance` enum — that enum is created by Plan 05-02 in parallel with this plan. When I first wrote `crates/vector-config/src/lib.rs` to pre-land a minimal `Appearance` enum (Rule 3 fix for the blocking dep), Plan 05-02 had simultaneously written a full schema.rs/loader.rs tree that re-exports `Appearance` from `schema::*`.
- **Fix:** Plan 05-02 won the race — my edit to `vector-config/src/lib.rs` was overwritten by 05-02's full module tree that includes a richer `Appearance` enum in `schema.rs` and re-exports it. Since 05-02's enum has the same variants (`System | Light | Dark`) and the same `serde(rename_all="lowercase")` attribute, my `vector-theme::resolve_palette` continues to work unchanged.
- **Files modified:** none of mine — the workspace state was settled by 05-02.
- **Verification:** `cargo test -p vector-theme --test appearance dark_light_flip` exits 0 against 05-02's `vector_config::Appearance`.
- **Committed in:** N/A (no longer in my diff; this is a parallel-execution coordination note for the verifier).

---

**Total deviations:** 3 auto-fixed (2 Rule 1 tooling, 1 Rule 3 blocking)
**Impact on plan:** All three are infrastructure-level (clippy strict lints + parallel-agent coordination). No scope creep. No deviation from the UI-SPEC §9.1/§9.2 contract.

## Issues Encountered

- **Parallel-agent workspace state churn:** Plan 05-01 + 05-02 were running concurrently with 05-03. Mid-execution, the workspace `vector-config/src/lib.rs` was rewritten by 05-02 (added schema.rs/loader.rs/error.rs modules); the workspace's `vector-arch-tests/` member appeared from 05-01; `Cargo.toml` workspace deps grew. None of this broke 05-03's scope (vector-theme only), but the first `cargo test -p vector-theme --no-run` failed because the workspace metadata was momentarily inconsistent (vector-app's Cargo.toml had a transient `[lints]workspace = true` + explicit `[lints.rust]/[lints.clippy]` conflict from 05-01's in-flight edit). The condition resolved on its own once the parallel agent's commit landed.

## User Setup Required

None.

## Next Phase Readiness

- **05-04 (toast + bell + Cmd-T tab title):** Can grep `vector_theme::Palette.chrome.surface/divider/button/warning` for the toast surface. The chrome-token contract is data-encoded.
- **05-07 (search bar):** Can grep `chrome.search_highlight` (rendered at alpha 0.40) + `chrome.danger_subtle` (no-match bar tint at alpha 0.20).
- **05-08 (tint stripe + picker):** Can grep `chrome.selection` (picker selected-row bg) + `chrome.fg_muted` (disabled rows).
- **05-09 (IME preedit underline):** Can grep `chrome.fg_muted`.
- **vector-app config-apply pipeline (whenever it lands):** Must call `parse_itermcolors(bytes)` to get a `Palette` whose grid colors are from the user's `.itermcolors`, then **replace** `result.chrome` with `resolve_palette(profile.appearance, system_is_dark).chrome` before handing the Palette to the renderer. This is the locked UI-SPEC §9.2 contract.

## Known Stubs

None. All implementation paths are concrete:
- `vector_dark()` + `vector_light()` return populated Palettes (no `Default::default()` placeholders).
- `parse_itermcolors()` is fully implemented (no `unimplemented!()` remains; the Task 1 stub was replaced in Task 2).
- `resolve_palette()` handles all three Appearance variants explicitly.

## Self-Check: PASSED

All 11 created/modified files verified on disk; all 4 task commits (`3d6b1ac` Task 1 RED, `dc578b1` Task 1 GREEN, `51adfbf` Task 2 RED, `240f9da` Task 2 GREEN) verified in `git log`.

---
*Phase: 05-polish-local-daily-driver*
*Completed: 2026-05-12*
