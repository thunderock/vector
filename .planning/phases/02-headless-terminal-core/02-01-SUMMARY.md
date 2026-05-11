---
phase: 02-headless-terminal-core
plan: 01
subsystem: infra
tags: [alacritty_terminal, portable-pty, async-trait, regex, workspace-scaffold, architecture-lint]

# Dependency graph
requires:
  - phase: 01-foundation-ci-dmg-pipeline
    provides: workspace + 14 crate stubs + per-crate no_tokio_main architecture-lint + ci.yml file-count guard
provides:
  - 15th workspace member crate `vector-headless` (binary + stub main.rs)
  - Workspace deps `alacritty_terminal 0.26`, `portable-pty 0.9`, `regex 1`, `async-trait 0.1`
  - 13 scaffolded #[ignore] test files (10 vector-term + 2 vector-pty + 1 vector-mux)
  - Resolved alacritty_terminal 0.26 import paths (`02-01-API-SPIKE.md`)
  - vector-term direct deps on alacritty_terminal + regex; dev-deps regex
  - vector-pty dev-deps on tokio (rt-multi-thread+process) + anyhow
affects: [02-02 vector-term wrapper, 02-03 vector-pty lifecycle, 02-04 vector-mux traits + LocalDomain, 02-05 vector-headless pass-through proxy]

# Tech tracking
tech-stack:
  added: [alacritty_terminal 0.26, portable-pty 0.9, regex 1, async-trait 0.1, crossterm 0.29 (binary-local), clap 4 (binary-local), scopeguard 1 (binary-local)]
  patterns: [
    "Architecture-lint inheritance via tests/no_tokio_main.rs copied verbatim per new crate (D-08)",
    "Per-crate file-count guard auto-tracks via `ls -d crates/vector-*` (CI invariant)",
    "Wave 0 = compile-clean #[ignore] test scaffolds; later waves un-ignore as implementations land",
    "API drift spike before implementation — `_api_probe` mod in lib.rs commits resolved import paths",
    "Hand-rolled `VectorDims` impl of `alacritty_terminal::grid::Dimensions` instead of test-module `TermSize`"
  ]

key-files:
  created:
    - crates/vector-headless/Cargo.toml
    - crates/vector-headless/src/main.rs
    - crates/vector-headless/tests/no_tokio_main.rs
    - crates/vector-term/tests/{csi_dispatch,osc_dispatch,dcs_dispatch,partial_utf8,alt_screen_1049,decstbm_scroll_region,ed_el_erase,sgr_truecolor,grapheme_width,scrollback_search}.rs
    - crates/vector-pty/tests/{lifecycle,term_env_advertise}.rs
    - crates/vector-mux/tests/trait_object_safety.rs
    - .planning/phases/02-headless-terminal-core/02-01-API-SPIKE.md
  modified:
    - Cargo.toml (workspace deps + members list)
    - crates/vector-term/Cargo.toml (alacritty_terminal + regex deps + dev-deps)
    - crates/vector-term/src/lib.rs (added `_api_probe` module)
    - crates/vector-pty/Cargo.toml (tokio + anyhow dev-deps)

key-decisions:
  - "Hand-roll `VectorDims` impl of `Dimensions` rather than depend on `term::test::TermSize` (test-module signal even though pub)"
  - "Mode-state CORE-06 tests live in `dcs_dispatch.rs` (3 functions appended) to keep file count at 10 for vector-term"
  - "`_api_probe` module committed in vector-term/src/lib.rs as a load-bearing compile check — Plan 02-02 replaces it with the real `Term` wrapper, but the probe ensures any future alacritty_terminal version bump breaks compilation visibly"
  - "Workspace deps re-sorted alphabetically while inserting 4 new entries (was unsorted in Phase 1)"

patterns-established:
  - "Wave-0 scaffolding pattern: add deps, scaffold #[ignore] tests, run API spike before any feature code — locks contracts for parallel downstream waves"
  - "API spike doc lives at .planning/phases/{NN}/{NN}-{plan}-API-SPIKE.md — committed alongside a compile-checked `_api_probe` module"

requirements-completed: [CORE-01, CORE-02, CORE-03, CORE-04, CORE-05, CORE-06]

# Metrics
duration: 7min
completed: 2026-05-11
---

# Phase 2 Plan 01: Headless Terminal Core — Wave 0 Foundation Summary

**Workspace scaffolding for Phase 2: new `vector-headless` binary crate (15th member), 4 workspace deps (`alacritty_terminal 0.26`, `portable-pty 0.9`, `regex 1`, `async-trait 0.1`), 13 `#[ignore]` test scaffolds covering CORE-01..06 + D-38, and a confirmed-import-paths API spike (`Processor` re-exported via `alacritty_terminal::vte::ansi`, `Color::Spec(Rgb)` variant, `Config.scrolling_history: usize` default 10000, hand-rolled `VectorDims` over `Dimensions`).**

## Performance

- **Duration:** ~7 min (399 s wall clock from Task 1 start to Task 3 commit)
- **Started:** 2026-05-11T15:56:33Z
- **Completed:** 2026-05-11T16:03:12Z
- **Tasks:** 3
- **Files modified:** 4 modified + 17 created (3 vector-headless crate files, 13 test scaffolds, 1 spike doc) = 21 total

## Accomplishments

- New `vector-headless` binary crate is workspace member #15; arch-lint per-crate file-count guard now 15==15.
- Four Phase-2 workspace deps declared at `[workspace.dependencies]` and alphabetized: `alacritty_terminal 0.26`, `async-trait 0.1`, `portable-pty 0.9`, `regex 1`.
- alacritty_terminal 0.26 API drift resolved by direct source inspection — every Open Question (1–3) and the bonus `Config.scrolling_history` field confirmed. RESEARCH.md placeholders held up under 0.26; no path revisions needed in code examples.
- 13 `#[ignore]` test scaffolds compile and run green (75 reported as `ignored` across the workspace) — Plans 02-02, 02-03, 02-04 can un-ignore them as implementations land without authoring scaffolding.
- `_api_probe` module in `crates/vector-term/src/lib.rs` is a load-bearing compile check that proves the spike findings against real 0.26.

## Task Commits

1. **Task 1: Add workspace deps + scaffold vector-headless crate** — `70dd49b` (feat)
2. **Task 2: API spike — resolve alacritty_terminal 0.26 import paths** — `c565208` (chore)
3. **Task 3: Scaffold 13 #[ignore] test files (CORE-01..06 + D-38)** — `6ea3131` (test)

## Files Created/Modified

### Created

- `crates/vector-headless/Cargo.toml` — new binary crate manifest with path deps on vector-{term,pty,mux}, workspace tokio/anyhow/tracing/async-trait/regex, binary-local clap+crossterm+scopeguard, `[lints] workspace = true`.
- `crates/vector-headless/src/main.rs` — stub `fn main() -> anyhow::Result<()>` printing "not yet implemented (Plan 02-05)". No `block_on`, no `tokio::main`.
- `crates/vector-headless/tests/no_tokio_main.rs` — verbatim copy of vector-app pattern; `BLOCK_ON_ALLOWLIST = &["main.rs"]` reserves space for Plan 02-05's `rt.block_on(...)`.
- `crates/vector-term/tests/csi_dispatch.rs` — CORE-01: 3 stubs (`echo_hello_lands_in_cell_0_0`, `cursor_position_csi_h`, `cursor_movement_csi_abcd`).
- `crates/vector-term/tests/osc_dispatch.rs` — CORE-01: 2 stubs (`osc_2_sets_window_title`, `osc_10_11_query_default_colors`).
- `crates/vector-term/tests/dcs_dispatch.rs` — CORE-01 + CORE-06: 4 stubs (`dcs_passes_through_without_corrupting_following_csi`, `bracketed_paste_mode_2004_sets_state`, `mouse_mode_1006_sgr_sets_state`, `decscusr_cursor_shape_sets_state`). Note: CORE-06 mode tests live here, not in `alt_screen_1049.rs`.
- `crates/vector-term/tests/partial_utf8.rs` — CORE-01: 2 stubs.
- `crates/vector-term/tests/alt_screen_1049.rs` — CORE-01: 2 stubs.
- `crates/vector-term/tests/decstbm_scroll_region.rs` — CORE-01: 2 stubs.
- `crates/vector-term/tests/ed_el_erase.rs` — CORE-01: 2 stubs.
- `crates/vector-term/tests/sgr_truecolor.rs` — CORE-02: 3 stubs.
- `crates/vector-term/tests/grapheme_width.rs` — CORE-02: 2 stubs.
- `crates/vector-term/tests/scrollback_search.rs` — CORE-03: 2 stubs.
- `crates/vector-pty/tests/lifecycle.rs` — CORE-04: 4 stubs.
- `crates/vector-pty/tests/term_env_advertise.rs` — CORE-05: 1 stub.
- `crates/vector-mux/tests/trait_object_safety.rs` — D-38: 2 stubs.
- `.planning/phases/02-headless-terminal-core/02-01-API-SPIKE.md` — confirmed import paths + hand-rolled `VectorDims` reference impl.

### Modified

- `Cargo.toml` — added `crates/vector-headless` to members; alphabetized `[workspace.dependencies]` and inserted 4 new entries.
- `crates/vector-term/Cargo.toml` — added `alacritty_terminal` + `regex` deps; added `[dev-dependencies] regex` (for fixture test code in Plan 02-02).
- `crates/vector-term/src/lib.rs` — added `_api_probe` module proving spike-resolved paths compile (replaced by Plan 02-02's real `Term` wrapper).
- `crates/vector-pty/Cargo.toml` — added `[dev-dependencies]` for `tokio` (rt-multi-thread + macros + time + sync + process features) and `anyhow` — required by Plan 02-03's lifecycle integration tests that hand-roll a runtime.
- `Cargo.lock` — regenerated from new deps (alacritty_terminal + transitive: vte, polling, rustix-openpty, etc.).

## API Spike Findings (Confirmed Import Paths)

| Path | Result |
|------|--------|
| `alacritty_terminal::vte::ansi::Processor` | CONFIRMED. `Processor` lives in `vte 0.15`'s `ansi` module; alacritty_terminal re-exports `pub use vte;` at lib.rs:20. Signature: `advance<H: Handler>(&mut self, &mut H, &[u8])`. |
| `Dimensions` trait | CONFIRMED at `alacritty_terminal::grid::Dimensions`. Required methods: `total_lines`, `screen_lines`, `columns`. |
| `term::test::TermSize` | EXPOSED-IN-TEST-MODULE. `pub mod test` makes it accessible at runtime, but the `test` namespace signals intent. **Decision:** hand-roll 5-line `VectorDims` impl. |
| `Color::Spec(Rgb)` | CONFIRMED at `vte::ansi::Color` (re-exported). Variants: `Named(NamedColor)`, `Spec(Rgb)`, `Indexed(u8)`. `Cell.fg: Color` per `term/cell.rs:136`. |
| `Config.scrolling_history` | CONFIRMED — field exists, type `usize`, default 10000 (`term/mod.rs:336,359`). |
| `term::search::RegexSearch` | CONFIRMED — unchanged from research. |
| `index::Point`, `index::Direction` | CONFIRMED — unchanged. |

No RESEARCH.md placeholder required correction; the speculative paths were accurate.

## Decisions Made

- **Hand-roll `VectorDims` over `term::test::TermSize`** — RESEARCH Open Question 2 resolution. TermSize is technically `pub` in the `test` module but the namespace signals intent. A 5-line struct + impl keeps vector-term's surface decoupled from a test helper.
- **CORE-06 mode-state tests live in `dcs_dispatch.rs`** — Plan instructs appending three CORE-06 stubs (`bracketed_paste_mode_2004_sets_state`, `mouse_mode_1006_sgr_sets_state`, `decscusr_cursor_shape_sets_state`) to that file to avoid creating an 11th vector-term test file. Justified by file-count discipline (the VALIDATION.md test-map row "Wave 0: CORE-01 DCS pass-through + CORE-06 mode flags" pairs them deliberately).
- **`_api_probe` module commits as a load-bearing compile-time anchor** — anything that breaks alacritty_terminal 0.26's surface compatibility (future minor bumps, refactors during Plan 02-02) trips a compile error in `vector-term`. Plan 02-02 replaces it with the real `Term` wrapper.
- **Workspace `[workspace.dependencies]` re-alphabetized** — Phase 1 left it unsorted; inserting 4 new entries was a natural moment to fix. No behavior change.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed clippy::no_effect_underscore_binding on `_api_probe`**

- **Found during:** Task 3 (after running `cargo clippy --workspace --all-targets -- -D warnings` from the plan's `<verification>` block)
- **Issue:** The probe used `let _config = Config::default();` and `let _dims = VectorDims { ... };`. Workspace clippy pedantic flags `_`-prefixed bindings with no side-effect.
- **Fix:** Replaced both with `let _ = …;` (anonymous discard pattern).
- **Files modified:** `crates/vector-term/src/lib.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` exits 0.
- **Committed in:** `6ea3131` (Task 3 commit — bundled with test scaffolds since the lint surfaced during Task 3's verification, not Task 2's narrower check).

**2. [Rule 1 - Bug] Reformatted `_api_probe` per rustfmt**

- **Found during:** Task 3 (after running `cargo fmt --all -- --check`)
- **Issue:** Imports weren't in rustfmt-canonical alphabetical order, and a multi-field struct literal was on one line.
- **Fix:** `cargo fmt --all` reordered imports + wrapped the `Config { scrolling_history: …, ..Config::default() }` literal across lines.
- **Files modified:** `crates/vector-term/src/lib.rs`
- **Verification:** `cargo fmt --all -- --check` exits 0.
- **Committed in:** `6ea3131` (Task 3 commit).

---

**Total deviations:** 2 auto-fixed (2 lint bugs, both Rule 1).
**Impact on plan:** Both auto-fixes are mechanical lint compliance — no scope, behavior, or contract change. The plan's `<acceptance_criteria>` did not include `cargo clippy` / `cargo fmt`, but the `<verification>` block did; honoring it surfaced both lints.

## Issues Encountered

None — the plan executed exactly as written. The API spike landed cleanly: every speculative path in RESEARCH.md held up under direct inspection of `alacritty_terminal 0.26`'s source.

## Verification Results

Final state of plan-level verification (all green):

```
cargo build --workspace                                       ✓ all 15 crates compile
cargo test --workspace --tests                                ✓ 0 FAILED, 75 ignored stubs
cargo clippy --workspace --all-targets -- -D warnings         ✓ clean
cargo fmt --all -- --check                                    ✓ clean
ls crates/*/tests/no_tokio_main.rs | wc -l   == 15            ✓
ls -d crates/vector-* | wc -l                == 15            ✓
grep -q 'alacritty_terminal = "0.26"' Cargo.toml              ✓
grep -q 'portable-pty = "0.9"' Cargo.toml                     ✓
grep -q 'async-trait = "0.1"' Cargo.toml                      ✓
grep -q '"crates/vector-headless"' Cargo.toml                 ✓
test -f .../02-01-API-SPIKE.md                                ✓
```

## Hand-off Notes for Downstream Plans

### Plan 02-02 (vector-term wrapper, Wave 1)

- **Replace** the `_api_probe` module in `crates/vector-term/src/lib.rs` with the real `Term`/`Parser`/`Search` wrapper API. The probe's import block is the canonical reference for `use` lines.
- **Un-ignore** all 10 vector-term test files: csi_dispatch, osc_dispatch, dcs_dispatch (CORE-06 tests included here — un-ignore alongside DCS), partial_utf8, alt_screen_1049, decstbm_scroll_region, ed_el_erase, sgr_truecolor, grapheme_width, scrollback_search.
- **`Cell.fg` matching:** use `matches!(cell.fg, Color::Spec(rgb) if rgb.r == 255 ...)` per the API spike. For 256-color tests, match `Color::Indexed(n)`.
- **`Dimensions` impl:** copy the `VectorDims` struct from the probe (or the API-SPIKE.md reference impl) into `vector-term`'s real surface.
- **`Config::default()`** already sets `scrolling_history: 10_000` — the scrollback_search 10k-line test can use the default constructor; the field is `pub` if a larger window is needed for headroom.

### Plan 02-03 (vector-pty lifecycle, Wave 2)

- **Un-ignore** `crates/vector-pty/tests/lifecycle.rs` (4 stubs) and `crates/vector-pty/tests/term_env_advertise.rs` (1 stub).
- **Dev-deps already wired:** `tokio` with `rt-multi-thread + macros + time + sync + process` features + `anyhow`. Test bodies can build a runtime via `tokio::runtime::Builder::new_multi_thread().build()?.block_on(async { ... })` (the integration-test files under `tests/` are NOT scanned by `no_tokio_main.rs`).
- **`portable-pty 0.9`** is wired at workspace level; add `portable-pty = { workspace = true }` to `crates/vector-pty/Cargo.toml`'s `[dependencies]` (Plan 02-01 did NOT add it because no code under Plan 02-01 needed to construct a PTY — Plan 02-03 owns this).

### Plan 02-04 (vector-mux traits + LocalDomain, Wave 3)

- **Un-ignore** `crates/vector-mux/tests/trait_object_safety.rs` (2 stubs: `pty_transport_is_object_safe`, `domain_is_object_safe`). Bodies are compile-time checks — instantiate `Box<dyn PtyTransport>` and `Box<dyn Domain>` to force the trait surface to remain object-safe.
- **`async-trait 0.1`** is wired at workspace level; add `async-trait = { workspace = true }` to `crates/vector-mux/Cargo.toml` `[dependencies]`.

### Plan 02-05 (vector-headless pass-through proxy, Wave 4)

- Replace `crates/vector-headless/src/main.rs`'s stub with the real binary per D-36. The architecture-lint `BLOCK_ON_ALLOWLIST = &["main.rs"]` already reserves space for `rt.block_on(...)` per D-09.
- Binary-local deps (clap, crossterm, scopeguard) are wired in `Cargo.toml`; no `Cargo.toml` edit needed for the implementation.

## Next Phase Readiness

- Phase 2 Wave 0 is complete; Plans 02-02 / 02-03 / 02-04 can execute in parallel (Wave 1). Plan 02-02 modifies the same test files this plan scaffolded — orchestrator should run them sequentially or coordinate test-file ownership.
- API drift risk on `alacritty_terminal` is mitigated: `_api_probe` will fail to compile if the surface shifts.
- No blockers identified.

## Self-Check: PASSED

All claimed files exist:

- crates/vector-headless/Cargo.toml — FOUND
- crates/vector-headless/src/main.rs — FOUND
- crates/vector-headless/tests/no_tokio_main.rs — FOUND
- crates/vector-term/tests/csi_dispatch.rs — FOUND
- crates/vector-term/tests/osc_dispatch.rs — FOUND
- crates/vector-term/tests/dcs_dispatch.rs — FOUND
- crates/vector-term/tests/partial_utf8.rs — FOUND
- crates/vector-term/tests/alt_screen_1049.rs — FOUND
- crates/vector-term/tests/decstbm_scroll_region.rs — FOUND
- crates/vector-term/tests/ed_el_erase.rs — FOUND
- crates/vector-term/tests/sgr_truecolor.rs — FOUND
- crates/vector-term/tests/grapheme_width.rs — FOUND
- crates/vector-term/tests/scrollback_search.rs — FOUND
- crates/vector-pty/tests/lifecycle.rs — FOUND
- crates/vector-pty/tests/term_env_advertise.rs — FOUND
- crates/vector-mux/tests/trait_object_safety.rs — FOUND
- .planning/phases/02-headless-terminal-core/02-01-API-SPIKE.md — FOUND

All claimed commits exist:

- 70dd49b — FOUND
- c565208 — FOUND
- 6ea3131 — FOUND

---
*Phase: 02-headless-terminal-core*
*Plan: 01*
*Completed: 2026-05-11*
