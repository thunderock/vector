---
phase: 05-polish-local-daily-driver
plan: 07
subsystem: input + fonts + app-shell
tags: [polish-02, polish-06, d-53, d-54, d-69, d-76, d-77, pitfall-8, ligatures, nerd-font, search-bar, selection]

# Dependency graph
requires:
  - phase: 03-gpu-renderer-first-paint
    provides: "vector_input::SelectionRange (anchor/cursor (col,row) u16 pairs; row-major
      cells() enumerator) from Plan 03-04 D-54"
  - phase: 03-gpu-renderer-first-paint
    provides: "vector_fonts::FontStack::{load_bundled, rasterize, cell_metrics} from
      Plan 03-02 D-40/D-41/D-50 (crossfont 0.9 + bundled JetBrains Mono TTF)"
  - phase: 02-headless-terminal-core
    provides: "vector_term::Term::search(&Regex) -> Vec<Match> from Plan 02-08 D-39
      (Match: start_row/start_col/end_row/end_col)"
provides:
  - "vector_input::selection_to_string<G: GridAccess>(&SelectionRange, &G, SelectionMode) -> String"
  - "vector_input::GridAccess trait (cell_char / cell_is_wide_spacer / cols)"
  - "vector_input::SelectionMode { Stream, Rectangular }"
  - "vector_fonts::FontStack::set_ligatures(bool) + ligatures_enabled() -> bool (POLISH-02 D-69 Pattern 5)"
  - "vector_app::search_bar::SearchBar state machine (open_with / close / set_query)"
  - "vector_app::search_bar::MatchCache (1000-cap with OverThousand flag; next/prev wrap; counter)"
  - "vector_app::search_bar::smart_case_regex(&str) -> Regex (D-77)"
  - "vector_app::search_bar::MAX_CACHED_MATCHES: usize = 1000 const (D-77)"
affects:
  - "05-08 (Cmd-C / Cmd-F render wiring) — consumes selection_to_string for NSPasteboard route
    and consumes SearchBar state for the search-bar viewport overlay."
  - "05-04 (apply pipeline) — LiveChange::Ligatures(bool) now has a destination
    (FontStack::set_ligatures) to push the new value."

# Tech tracking
tech-stack:
  added:
    - "regex 1 (workspace) declared as direct dep of vector-app for smart_case_regex"
  patterns:
    - "Wide-char selection extraction skips WIDE_CHAR_SPACER cells per Pitfall 8.
      Trait-based GridAccess abstraction lets tests use a MockGrid without spinning
      up a real vector_term::Term grid."
    - "Always-regex query compilation with literal-escape fallback. smart_case_regex
      tries the raw query first (so users can type ranges like `[a-z]+`) and falls
      back to regex::escape if the pattern is malformed — ensures the search bar
      never panics on malformed input."
    - "Toggle-without-shaping pattern: ligatures_enabled is a runtime boolean that
      gates a deferred contiguous-run shaper path. CoreText shapes JetBrains Mono
      ligatures at glyph-lookup time unconditionally in v1; the toggle is plumbed
      for future use by 05-04's LiveChange::Ligatures."

key-files:
  created:
    - crates/vector-input/src/selection_string.rs
    - crates/vector-app/src/search_bar.rs
  modified:
    - crates/vector-input/src/lib.rs
    - crates/vector-input/tests/selection_string.rs
    - crates/vector-fonts/src/loader.rs
    - crates/vector-fonts/tests/ligatures.rs
    - crates/vector-app/Cargo.toml
    - crates/vector-app/src/lib.rs
    - crates/vector-app/tests/search_bar.rs

key-decisions:
  - "GridAccess trait over `&vector_term::Term` direct dep: keeps vector-input free of
    any vector-term coupling for the Cmd-C path. The vector-term Term impl of GridAccess
    can land in either crate's seam in Plan 05-08 (likely a `impl GridAccess for &Term`
    in vector-app or a thin adapter in vector-term — deferred so this plan ships
    without expanding vector-input's coupling surface)."
  - "Selection-string lives in a new module `selection_string.rs` (not appended to
    existing `selection.rs`) — the existing module is the cell-coordinate state machine
    (Idle/Dragging/Selected with mouse_down/move/up); selection_to_string is a pure
    grid walker. Separate concerns, separate module."
  - "smart_case_regex returns `Regex` directly (not `Result<Regex>`). Rationale: the
    fallback path (regex::escape literal) always compiles, so the function is
    infallible; callers don't have to plumb a Result through the SearchBar state machine."
  - "MAX_CACHED_MATCHES = 1000 as a public const, not a magic number. Plan 05-08's
    render layer can read the same const for the `1000+` overflow badge text."

patterns-established:
  - "Trait-mock testing: GridAccess is implemented by a private MockGrid struct in
    tests/selection_string.rs — proves the trait surface is sufficient for the Cmd-C
    extraction path without dragging vector-term into vector-input's test graph."

requirements-completed: [POLISH-02, POLISH-06]

# Metrics
duration: 6min
completed: 2026-05-12
---

# Phase 5 Plan 07: Selection-string + ligatures + search bar Summary

**Closed POLISH-02 (ligature toggle + Nerd Font glyph rasterization) + POLISH-06 (search bar logic — smart-case + 1000-cap cache + esc-restore) + D-53/D-54 Cmd-C selection-to-string carry; three loosely-coupled features wrapped in one plan because they share Wave 2 timing and don't touch each other's surfaces. 10 Wave-0 stubs un-ignored and green across vector-input, vector-fonts, vector-app.**

## Performance

- **Duration:** ~6 min (parallel with Plan 05-04)
- **Started:** 2026-05-12T18:50:22Z
- **Completed:** 2026-05-12T18:56:40Z
- **Tasks:** 3 (TDD: 3 RED + 3 GREEN = 6 task commits)
- **Files modified/created:** 9 (2 created + 7 modified)

## Accomplishments

- **`selection_to_string<G: GridAccess>(&SelectionRange, &G, SelectionMode) -> String`** ships in `crates/vector-input/src/selection_string.rs` with:
  - **Pitfall 8 honored:** WIDE_CHAR_SPACER cells skipped (verified by `wide_chars_collapse` — "你好" with `[char, spacer, char, spacer]` cells emits `"你好"`, not `"你 好 "`).
  - **Per-row trailing-ws strip** via `trim_end()` (verified by `trailing_ws_stripped` — `"hello   "` → `"hello"`).
  - **Stream vs Rectangular modes:** stream mode = first row [anchor_col, cols), middle rows full, last row [0, cursor_col+1); rectangular = every row [min_col, max_col+1) joined with `\n` (verified by `rect_uses_newline` 2x2 grid → `"ab\ncd"`).
  - **`GridAccess` trait** with `cell_char(row,col)` / `cell_is_wide_spacer(row,col)` / `cols()` — implementer-agnostic so vector-input stays decoupled from vector-term.
- **`FontStack::set_ligatures(bool)` + `FontStack::ligatures_enabled() -> bool`** ship in `crates/vector-fonts/src/loader.rs`. Field defaults to `true`; runtime no-op for v1 (CoreText shapes JetBrains Mono ligatures unconditionally at glyph-lookup time per Pattern 5 in 05-RESEARCH.md). The toggle is plumbed for the deferred contiguous-run shaper path; Plan 05-04's `LiveChange::Ligatures(bool)` now has a destination.
- **3 ligature tests green** — `ligature_glyph_present` rasterizes `>`, `-`, `=`, `<` individually with ligatures enabled; `ligature_toggle_off` verifies the toggle read-back + that rasterization keeps working with the toggle off; `nerd_font_codepoint_renders` rasterizes U+E0A0 (Powerline branch) successfully via CoreText's fallback chain.
- **`SearchBar` state machine** in `crates/vector-app/src/search_bar.rs`:
  - `open_with(prior_selection)`: stores saved_selection, clears query, sets `open=true`.
  - `close() -> Option<SelectionRange>`: returns saved_selection so caller can restore it (D-76 Esc-restore).
  - `set_query(&str, &Term)`: compiles smart-case regex, calls `Term::search(&Regex) -> Vec<Match>`, wraps in MatchCache.
- **`smart_case_regex(&str) -> Regex`** (D-77): all-lowercase → `(?i)` prefix (case-insensitive); any uppercase → case-sensitive raw query. Fallback to `regex::escape` literal pattern if the query isn't a valid regex — ensures the function is infallible so the state machine doesn't need to plumb a `Result`.
- **`MatchCache`** with `MAX_CACHED_MATCHES = 1000` cap (D-77):
  - `from_matches(Vec<Match>)` truncates at 1000 and flips `overflow` to `MatchOverflow::OverThousand`; ≤1000 records `MatchOverflow::Bounded(len)`.
  - `next()/prev()` wrap-around; `counter() -> (usize, MatchOverflow)` returns 1-based active index for the search-bar HUD.
- **4 search-bar tests green** — `smart_case_lower`/`smart_case_upper` verify the D-77 case-fold contract; `cache_1000_lazy` builds 1500 dummy matches and asserts cache length = 1000 + `OverThousand` flag; `esc_restores_selection` verifies the Esc-close → return-saved-selection round-trip.

## Task Commits

Each task followed TDD (RED → GREEN):

1. **Task 1 RED — failing selection_to_string tests:** `360615b` (test)
2. **Task 1 GREEN — selection_to_string impl + GridAccess + SelectionMode:** `0620f6a` (feat)
3. **Task 2 RED — failing ligature/Nerd Font tests:** `bd9e584` (test)
4. **Task 2 GREEN — ligatures_enabled toggle plumbing on FontStack:** `3448678` (feat)
5. **Task 3 RED — failing SearchBar/smart_case/cache tests:** `39b4153` (test)
6. **Task 3 GREEN — SearchBar state machine + smart_case_regex + MatchCache:** `45bf82a` (feat)

_Note: Plan 05-04 ran in parallel and landed `21189de`, `fc2245d`, `10f398e` between my commits — those are 05-04's scope (notify-debouncer-full + apply pipeline + POLISH-01/02 hot-reload), not mine. My commits target only `crates/vector-input/**`, `crates/vector-fonts/**`, `crates/vector-app/**` per the plan's `files_modified` list._

## Files Created/Modified

**Created:**
- `crates/vector-input/src/selection_string.rs` — selection_to_string + GridAccess + SelectionMode + iter_rows (stream + rect dispatch).
- `crates/vector-app/src/search_bar.rs` — SearchBar + MatchCache + MatchOverflow + smart_case_regex + MAX_CACHED_MATCHES.

**Modified:**
- `crates/vector-input/src/lib.rs` — wired `mod selection_string;` + re-exports.
- `crates/vector-input/tests/selection_string.rs` — 3 stubs un-ignored + MockGrid impl.
- `crates/vector-fonts/src/loader.rs` — added `ligatures_enabled: bool` field + set_ligatures/ligatures_enabled accessors.
- `crates/vector-fonts/tests/ligatures.rs` — 3 stubs un-ignored.
- `crates/vector-app/Cargo.toml` — added `regex.workspace = true` direct dep.
- `crates/vector-app/src/lib.rs` — wired `pub mod search_bar;`.
- `crates/vector-app/tests/search_bar.rs` — 4 stubs un-ignored.

## Decisions Made

- **`GridAccess` trait over a `vector_term::Term` direct dep.** Keeps `vector-input` free of any vector-term coupling for the Cmd-C path. A future `impl GridAccess for &Term` adapter lives in either vector-app's seam or a thin adapter module in vector-term — deferred so this plan doesn't expand vector-input's coupling surface. Tests verify the contract via a MockGrid.
- **Selection-string in a new module, not appended to `selection.rs`.** `selection.rs` is the cell-coordinate state machine (Idle / Dragging / Selected with mouse_down/move/up); `selection_string.rs` is a pure grid walker. Separate concerns, separate file.
- **`smart_case_regex` returns `Regex` directly, not `Result<Regex>`.** The literal-escape fallback always compiles, so the function is infallible. Callers don't have to thread a Result through the SearchBar state machine for the (rare) case of a malformed regex.
- **`MAX_CACHED_MATCHES` exposed as a public const.** Plan 05-08's render layer reads the same const for the `1000+` overflow badge text — no magic number duplication.
- **`MatchOverflow::Bounded(usize)` variant carries the count, even though it duplicates `cache.matches().len()`.** Lets the HUD render `"N matches"` without re-querying `.matches().len()` (and avoids a `.len()` call from inside the cache borrow).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Tooling] clippy::trivially_copy_pass_by_ref on &SelectionRange**

- **Found during:** Task 1 verify (clippy gate)
- **Issue:** `SelectionRange` is `Copy` and only 8 bytes; workspace clippy.pedantic flags `&SelectionRange` as inefficient (suggests pass-by-value). However the plan's normative contract for `selection_to_string` mandates `range: &SelectionRange` so the call site can pass `&state.range()` directly without `.copied()`.
- **Fix:** Added `#[allow(clippy::trivially_copy_pass_by_ref)]` on both `selection_to_string` and the private `iter_rows` helper. Documented the rationale inline: "contract: matches plan signature".
- **Files modified:** `crates/vector-input/src/selection_string.rs`
- **Verification:** `cargo clippy -p vector-input --all-targets -- -D warnings` exits 0.
- **Committed in:** 0620f6a (Task 1 GREEN)

**2. [Rule 1 - Tooling] clippy::map_unwrap_or on MockGrid::cell_is_wide_spacer**

- **Found during:** Task 1 verify (clippy gate, test file)
- **Issue:** Workspace pedantic flags `.map(|c| c.1).unwrap_or(false)` on Option chains; suggests `.is_some_and(|c| c.1)`.
- **Fix:** Mechanical conversion; behavior preserved.
- **Files modified:** `crates/vector-input/tests/selection_string.rs`
- **Committed in:** 0620f6a (folded into Task 1 GREEN since the test file is in the same commit as the impl).

**3. [Rule 1 - Tooling] clippy::assigning_clones on SearchBar::set_query**

- **Found during:** Task 3 verify (clippy gate)
- **Issue:** `self.query = q.to_owned();` flagged as inefficient; clippy suggests `q.clone_into(&mut self.query)` which reuses the destination allocation.
- **Fix:** Mechanical conversion to `q.clone_into(&mut self.query)`.
- **Files modified:** `crates/vector-app/src/search_bar.rs`
- **Verification:** `cargo clippy -p vector-app --all-targets -- -D warnings` exits 0.
- **Committed in:** 45bf82a (folded into Task 3 GREEN; commit body amended to reflect the final clippy-clean state).

---

**Total deviations:** 3 (all Rule 1 mechanical pedantic-clippy lint fixes — no scope creep, no contract changes)
**Impact on plan:** Zero. Public API and test contracts match the plan exactly.

## Issues Encountered

- **Parallel-agent workspace state churn (cosmetic):** Plan 05-04 was running concurrently and landed `21189de` (apply pipeline) + `fc2245d` (Cargo.lock for notify-debouncer-full) + `10f398e` (POLISH-04 SUMMARY) interleaved with my task commits. None of this affected my scope (vector-input + vector-fonts + vector-app) — the crates Plan 05-04 modifies (vector-config + vector-theme + vector-app::apply) are disjoint from this plan's `files_modified` list.

## User Setup Required

None. All three features are pure-Rust and tested headlessly.

## Next Phase Readiness

- **05-08 (Cmd-C / Cmd-F / picker render wiring):** Inherits `selection_to_string` for the NSPasteboard route and the entire `SearchBar` state machine + smart_case_regex for the search-bar viewport overlay. The render side wires the Cmd-F keystroke (open_with current selection), text input → `set_query`, Up/Down → `cache.next/prev`, Esc → `close()` → restore returned selection. The `1000+` badge text reads `MAX_CACHED_MATCHES` directly.
- **05-04 (apply pipeline):** `LiveChange::Ligatures(bool)` now has a destination — Plan 05-04's apply code can call `font_stack.set_ligatures(new_value)` without any further plumbing.
- **The `impl GridAccess for &Term` adapter** lands in Plan 05-08 (or a thin adapter in vector-term) — deferred from this plan to keep vector-input's coupling surface unchanged.

## Known Stubs

None. All implementation paths are concrete:
- `selection_to_string` is a complete, correct grid walker with both stream and rect modes wired and tested.
- `FontStack::set_ligatures` / `ligatures_enabled` are real read/write boolean accessors; runtime behavior (per Pattern 5) is "no-op because CoreText already shapes ligatures unconditionally". The plan acknowledges this is the v1 contract — _not_ a stub but a deferred contiguous-run shaper path.
- `SearchBar` / `MatchCache` / `smart_case_regex` are fully implemented with no `unimplemented!()` or `todo!()` remaining.

## Self-Check: PASSED

All claimed files verified on disk:

```
FOUND: crates/vector-input/src/selection_string.rs
FOUND: crates/vector-app/src/search_bar.rs
FOUND: crates/vector-input/tests/selection_string.rs (un-ignored)
FOUND: crates/vector-fonts/tests/ligatures.rs (un-ignored)
FOUND: crates/vector-app/tests/search_bar.rs (un-ignored)
```

All claimed commits verified in `git log`:

```
FOUND: 360615b (Task 1 RED: test selection_to_string)
FOUND: 0620f6a (Task 1 GREEN: selection_to_string impl)
FOUND: bd9e584 (Task 2 RED: test ligature toggle + Nerd Font)
FOUND: 3448678 (Task 2 GREEN: FontStack::set_ligatures plumbing)
FOUND: 39b4153 (Task 3 RED: test SearchBar smart-case + cache + esc)
FOUND: 45bf82a (Task 3 GREEN: SearchBar state machine + smart_case_regex)
```

Acceptance criteria:
- All 10 Wave-0 stubs un-ignored and green (3 vector-input + 3 vector-fonts + 4 vector-app).
- `grep -q "pub fn selection_to_string" crates/vector-input/src/selection_string.rs` ✓
- `grep -q "is_wide_spacer\|WIDE_CHAR_SPACER" crates/vector-input/src/selection_string.rs` ✓ (both — Pitfall 8 named in module doc + trait method)
- `grep -q "trim_end" crates/vector-input/src/selection_string.rs` ✓
- `grep -q "ligatures_enabled\|set_ligatures" crates/vector-fonts/src/loader.rs` ✓
- `grep -q "pub fn smart_case_regex" crates/vector-app/src/search_bar.rs` ✓
- `grep -q "MAX_CACHED_MATCHES: usize = 1000" crates/vector-app/src/search_bar.rs` ✓
- `grep -q "MatchOverflow::OverThousand" crates/vector-app/src/search_bar.rs` ✓
- `cargo clippy --workspace --all-targets -- -D warnings` exits 0 ✓
- Workspace tests: 270 passed / 0 failed / 14 ignored (baseline 234 + plan-05-04 deltas + 10 from plan-07).

---
*Phase: 05-polish-local-daily-driver*
*Completed: 2026-05-12*
