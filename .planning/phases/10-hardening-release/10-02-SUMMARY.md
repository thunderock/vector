---
phase: 10-hardening-release
plan: 02
subsystem: vector-term + vector-render
tags: [harden-02, vt-conformance, perf-gate, ci, macos-14]
one_liner: "HARDEN-02 ships an 8-file VT conformance corpus (17 tests, runs in 0.01s) and a custom perf-probe example feeding two new ci.yml jobs — vt-conformance as a hard merge gate, perf-gate split arm64 hard / intel advisory per D-23."
requires:
  - alacritty_terminal::Term + TermMode
  - vector_term::Term::feed / Term::mode / Term::cursor / Term::grid / Term::with_channels
  - vector_render::RenderContext::new_offscreen
  - vector_render::Compositor::new_with + render_offscreen_with
  - vector_fonts::FontStack::load_bundled
provides:
  - HARDEN-02 vt-conformance CI gate (hard, macos-14)
  - HARDEN-02 perf-gate CI gate (arm64 hard / intel advisory per D-23)
  - reusable perf-probe binary emitting deterministic JSON for CI threshold gates
affects:
  - .github/workflows/ci.yml (+ vt-conformance + perf-gate jobs)
  - crates/vector-render/Cargo.toml (+ libc dev-dep, [[example]] block)
tech_stack:
  added:
    - "libc 0.2 as vector-render dev-dep (for getrusage in perf_probe)"
  patterns:
    - "VT corpus drives alacritty_terminal::Term directly — no PTY, no GPU, no winit (D-08)"
    - "Cargo integration test with parent `vt_conformance.rs` + sibling `vt_conformance/` subdir via #[path]"
    - "Custom probe binary over criterion for threshold gates (Research Pattern 3): deterministic numbers, no statistical flapping"
    - "CI matrix with continue-on-error: ${{ matrix.advisory }} for D-23 split arm64-hard / intel-advisory"
key_files:
  created:
    - crates/vector-term/tests/vt_conformance.rs
    - crates/vector-term/tests/vt_conformance/mod.rs
    - crates/vector-term/tests/vt_conformance/alt_screen_1049.rs
    - crates/vector-term/tests/vt_conformance/scroll_regions.rs
    - crates/vector-term/tests/vt_conformance/tab_stops.rs
    - crates/vector-term/tests/vt_conformance/ed_el_erase.rs
    - crates/vector-term/tests/vt_conformance/mouse_1006.rs
    - crates/vector-term/tests/vt_conformance/osc52_round_trip.rs
    - crates/vector-term/tests/vt_conformance/bracketed_paste.rs
    - crates/vector-term/tests/vt_conformance/decscusr.rs
    - crates/vector-render/examples/perf_probe.rs
  modified:
    - .github/workflows/ci.yml (+ vt-conformance + perf-gate jobs)
    - crates/vector-render/Cargo.toml (+ libc dev-dep + [[example]] block)
    - Cargo.lock (libc resolution)
    - crates/vector-term/tests/alt_screen_1049.rs (NOTE: comment)
    - crates/vector-term/tests/dcs_dispatch.rs (NOTE: comment)
    - crates/vector-term/tests/decstbm_scroll_region.rs (NOTE: comment)
    - crates/vector-term/tests/ed_el_erase.rs (NOTE: comment)
    - crates/vector-term/tests/osc52.rs (NOTE: comment)
decisions:
  - "Corpus uses `cargo test -p vector-term --test vt_conformance` (singular --test). The plan's verify/acceptance grep used `--tests` (plural) + `vt_conformance` as a name filter — that combination matches zero test fns. Bug in plan spec auto-fixed (Rule 1) with an inline comment in ci.yml documenting why singular is correct."
  - "Precedent files in tests/*.rs kept in place (D-28). Each precedent gained a one-line NOTE: comment pointing at its corpus mirror — grep-friendly when looking at either file."
  - "Perf-probe lints required scoped `#![allow(unsafe_code)]` (for getrusage FFI), `#[allow(clippy::cast_precision_loss)]` on the rusage helper (tv_sec is i64, expected loss for seconds-since-boot is non-issue), and `&raw mut` instead of implicit borrow."
  - "Idle-CPU measurement uses libc::getrusage(RUSAGE_SELF) over a 5s wall window (deterministic, no `ps`/`top` shell-out). Paste-render FPS runs 60 frames of offscreen render against ~1.1 MB of pre-fed text — `n=60` chosen so per-frame overhead doesn't dominate (Apple Silicon observed ~125 fps locally, ~2× the 55 fps gate)."
metrics:
  duration: 7m 14s
  completed: 2026-05-26
  tasks: 3
  files_touched: 15
  commits: 3
---

# Phase 10 Plan 02: HARDEN-02 — VT Conformance Corpus + Perf Gate — Summary

Lock down VT parser + grid + cursor + mode pipeline against regression. New `crates/vector-term/tests/vt_conformance/` subdirectory holds eight scenario files mapped 1:1 to ROADMAP success criterion #2; the integration-test entrypoint `tests/vt_conformance.rs` pulls them in via `#[path]`. A companion `crates/vector-render/examples/perf_probe.rs` example binary emits one-line JSON for two CI gates: a hard merge gate on `vt-conformance` (macos-14 arm64) and a split arm64-hard / intel-advisory `perf-gate` per D-23. Precedent VT tests in `tests/*.rs` remain in place as a safety net; each carries a `NOTE:` comment pointing at its corpus mirror per D-28.

## Tasks

| # | Title | Commit |
|---|-------|--------|
| 1 | Build vt_conformance corpus — 8 scenario files + entrypoint + precedent NOTEs | `c23f6c3` |
| 2 | Author perf_probe example binary + libc dev-dep + [[example]] block | `0045ce9` |
| 3 | Add vt-conformance + perf-gate jobs to ci.yml (D-23 split) | `baae5b0` |

## Corpus Inventory

| Scenario | File | #[test] fns | Maps to PITFALL | Drives |
|----------|------|-------------:|------------------|--------|
| 1. alt-screen 1049 | `tests/vt_conformance/alt_screen_1049.rs` | 2 | Pitfall 1 | `Term::feed` + `Term::cursor` + `Term::grid` |
| 2. scroll regions  | `tests/vt_conformance/scroll_regions.rs`   | 2 | Pitfall 1 | DECSTBM + reset, grid assertions |
| 3. tab stops       | `tests/vt_conformance/tab_stops.rs`        | 2 | Pitfall 1 | HTS / CHT / TBC, cursor assertions |
| 4. ED/EL erase     | `tests/vt_conformance/ed_el_erase.rs`      | 3 | Pitfall 1 | ED 2 + EL 0 + EL 1, grid + scrollback search |
| 5. mouse 1006      | `tests/vt_conformance/mouse_1006.rs`       | 2 | Pitfall 8 | TermMode SGR_MOUSE + MOUSE_REPORT_CLICK |
| 6. OSC 52 round-trip | `tests/vt_conformance/osc52_round_trip.rs` | 2 | Pitfall 8 | raw + DCS-wrapped, ClipboardEvent::Store |
| 7. bracketed paste | `tests/vt_conformance/bracketed_paste.rs`  | 2 | Pitfall 8 | TermMode BRACKETED_PASTE |
| 8. DECSCUSR        | `tests/vt_conformance/decscusr.rs`         | 2 | Pitfall 1 | parser dispatch + post-cursor placement |
| **TOTAL**          |                                            | **17** | | |

Run time: 0.01s (well under the 2s D-08 budget and the 5s plan acceptance cap).

## Perf-probe Output (local Apple Silicon)

Three back-to-back runs:

```
{"idle_cpu_pct":0.038,"paste_render_fps":128.46}
{"idle_cpu_pct":0.031,"paste_render_fps":128.03}
{"idle_cpu_pct":0.035,"paste_render_fps":125.57}
```

- **idle_cpu_pct** ≈ 0.03% — far below the 1.0% hard gate.
- **paste_render_fps** ≈ 125 fps — ~2× the 55 fps hard gate. The probe disables vsync (offscreen render, no surface), so this measures raw render capability on the M-series Metal driver rather than vsync-capped frame pacing.

No threshold tuning was needed. On Apple Silicon the probe has comfortable headroom; CI runner jitter is the actual risk the D-23 advisory-on-intel split exists to absorb.

## ci.yml Additions

Two new jobs inserted after the existing `snapshot-suite` job (which 10-01 added). Both placed before `commitlint`:

- **`vt-conformance`** (macos-14, hard gate): `cargo test -p vector-term --test vt_conformance --release`. No `continue-on-error`.
- **`perf-gate`** (matrix arm64+intel):
  - arm64 (macos-14): `advisory: false`, threshold-enforcing python3 check (`idle < 1.0 and fps >= 55.0`).
  - intel (macos-15-intel): `advisory: true`, `continue-on-error: true`, logs probe JSON as a workflow notice without failing.
  - Both runners upload `target/perf.json` as `perf-probe-<runner>` artifact.

`continue-on-error: ${{ matrix.advisory }}` is set at the perf-gate job level (driven by matrix); vt-conformance has no `continue-on-error` so it stays a hard gate.

Approximate ci.yml diff: +72 lines, inserted after line ~51 (the snapshot-suite job tail) and before the `commitlint` job.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug in plan spec] `--tests vt_conformance` filter is wrong; use `--test vt_conformance`.**
- **Found during:** Task 3 — first run of the literal CI command from the plan acceptance criteria.
- **Issue:** The plan's `<action>` block and `<acceptance_criteria>` use `cargo test -p vector-term --tests vt_conformance`. `--tests` (plural) selects ALL integration tests; the trailing `vt_conformance` becomes a test-fn-name filter. The corpus's test fns are named `decset_1049_alt_screen_isolates_primary` etc., not `vt_conformance` — so `--tests vt_conformance` filtered out all 17 tests and "passed" vacuously.
- **Fix:** ci.yml uses `cargo test -p vector-term --test vt_conformance --release` (singular `--test` selects the integration-test binary, runs all 17 tests in it). Added an inline yaml comment in the job step explaining the choice so the next person doesn't "fix" it back.
- **Files modified:** `.github/workflows/ci.yml`.
- **Commit:** `baae5b0`.

**2. [Rule 3 - Blocking issue] Workspace `unsafe_code = "deny"` lint blocks getrusage FFI in perf_probe.**
- **Found during:** Task 2 first `cargo build`.
- **Issue:** `libc::getrusage` requires `unsafe` (FFI), but the workspace lints set `unsafe_code = "deny"`.
- **Fix:** Added `#![allow(unsafe_code)]` at the top of `examples/perf_probe.rs` with a comment explaining the FFI need. This mirrors the existing `vector-app::src/lib.rs::#![allow(unsafe_code)]` pattern for AppKit FFI.
- **Files modified:** `crates/vector-render/examples/perf_probe.rs`.
- **Commit:** `0045ce9`.

**3. [Rule 3 - Blocking issue] Clippy pedantic lints (cast_precision_loss, cast_lossless, implicit borrow as raw pointer, let-else) blocked the perf_probe build.**
- **Found during:** Task 2 `cargo clippy` after the unsafe_code allow landed.
- **Issue:** Workspace clippy has `pedantic = { level = "warn", priority = -1 }` and `-D warnings` is enforced in CI. Eight pedantic warnings turned into errors:
  - 3× `let_else_pattern`: simple `match ... return -1.0` chains.
  - 1× `implicit_borrow_as_raw_ptr`: `libc::getrusage(libc::RUSAGE_SELF, &mut ru)`.
  - 2× `cast_lossless` on `tv_usec as f64` (it's `i32`, has `From`).
  - 2× `cast_precision_loss` on `tv_sec as f64` (it's `i64` on macOS; precision loss is real but harmless for seconds-since-boot CPU-time values that won't exceed 2^52 anytime soon).
- **Fix:** Rewrote the three error returns as `let-else`; used `&raw mut ru` per the lint's `unsafe_op_in_unsafe_fn` suggestion; replaced `as f64` with `f64::from(...)` for `tv_usec` (i32 → f64 is infallible); added scoped `#[allow(clippy::cast_precision_loss)]` on `self_cpu_seconds` with a comment for the `tv_sec` casts. Final result is lint-clean under `-D warnings`.
- **Files modified:** `crates/vector-render/examples/perf_probe.rs`.
- **Commit:** `0045ce9`.

**4. [Rule 3 - Blocking issue] `cargo fmt` rewrote `vt_conformance/scroll_regions.rs`.**
- **Found during:** Task 1 lint pass.
- **Issue:** `cargo fmt --all -- --check` flagged formatting drift in two new files. The fmt action reflowed an inline `// reset to full screen` comment so it sits after the `term.feed(...)` call.
- **Fix:** `cargo fmt --all` (auto-applied). No semantic change.
- **Files modified:** `crates/vector-term/tests/vt_conformance/scroll_regions.rs`.
- **Commit:** Already included in `c23f6c3` (fmt ran before the commit).

### Non-deviations (just FYI)

- **Plan called for "AT LEAST 2 `#[test]` functions per scenario."** Met for all 8 (1 file has 3, one has 2 `#[tokio::test]`, the rest have 2 `#[test]`). Total = 17 tests, exceeding the plan's stated minimum of 16.
- **Plan called for `tests/vt_conformance/mod.rs` "as an empty docs-only file (mod.rs is optional given the explicit `#[path]` lines above — include it for grep-ability)."** Included as a 2-line docs-only file.
- **Plan called for "Run time check: `time cargo test -p vector-term --tests vt_conformance --release` real time < 5s."** Local run: 0.01s of test execution; cold-cache release compile dominates wall time (~50s), but that's compilation cost amortized across the whole suite — the test budget itself is met by a factor of 500.

## Authentication Gates

None. All work was filesystem + cargo + git.

## Acceptance Criteria Status

### Task 1
| Criterion | Status |
|-----------|--------|
| `ls crates/vector-term/tests/vt_conformance/*.rs \| wc -l` = 9 | PASS |
| Sum of `#[test]`/`#[tokio::test]` lines ≥ 16 | PASS (17) |
| `cargo test -p vector-term --test vt_conformance` exits 0 | PASS |
| Run time < 5s (release) | PASS (0.01s) |
| All 8 expected scenario filenames present | PASS |
| Pitfall references in 8+ files | PASS (8 of 8) |
| Precedent files unchanged behaviorally | PASS — 5 precedent test binaries still green |

### Task 2
| Criterion | Status |
|-----------|--------|
| `crates/vector-render/examples/perf_probe.rs` exists | PASS |
| `cargo build --release -p vector-render --example perf_probe` exits 0 | PASS |
| Output JSON has both `idle_cpu_pct` and `paste_render_fps` | PASS |
| Apple Silicon local: idle < 5% | PASS (~0.03%) |
| Apple Silicon local: fps ≥ 55 | PASS (~125 fps) |

### Task 3
| Criterion | Status |
|-----------|--------|
| `vt-conformance:` job present | PASS |
| `perf-gate:` job present | PASS |
| `cargo run --release -p vector-render --example perf_probe` invoked | PASS |
| `macos-15-intel` runner in matrix | PASS |
| `advisory: true` and `advisory: false` both present | PASS |
| `continue-on-error: ${{ matrix.advisory }}` present (perf-gate only) | PASS |
| vt-conformance has NO `continue-on-error` (hard gate) | PASS (verified via yaml.safe_load) |
| `python3 -c "import yaml; yaml.safe_load(...)"` valid | PASS |

### Plan-level
| Success Criterion | Status |
|-------------------|--------|
| HARDEN-02 acceptance line: VT corpus in CI + perf gate (idle <1%, vsync cap) | MET |
| D-23: arm64 hard gate + intel advisory log-only | MET |
| D-07: 8 scenarios 1:1 to ROADMAP, one file per scenario | MET |
| D-08: corpus drives alacritty_terminal::Term, <5s, no GPU/winit | MET (0.01s) |
| D-09: no vttest corpus, no e2e binary spawn | HONORED |
| D-28: precedents relocated (mirrored), not rewritten | HONORED |

## Self-Check: PASSED

All claimed files exist on disk:

```
crates/vector-term/tests/vt_conformance.rs                      FOUND
crates/vector-term/tests/vt_conformance/mod.rs                  FOUND
crates/vector-term/tests/vt_conformance/alt_screen_1049.rs      FOUND
crates/vector-term/tests/vt_conformance/scroll_regions.rs       FOUND
crates/vector-term/tests/vt_conformance/tab_stops.rs            FOUND
crates/vector-term/tests/vt_conformance/ed_el_erase.rs          FOUND
crates/vector-term/tests/vt_conformance/mouse_1006.rs           FOUND
crates/vector-term/tests/vt_conformance/osc52_round_trip.rs     FOUND
crates/vector-term/tests/vt_conformance/bracketed_paste.rs      FOUND
crates/vector-term/tests/vt_conformance/decscusr.rs             FOUND
crates/vector-render/examples/perf_probe.rs                     FOUND
```

All three task commits in `git log`:

```
baae5b0 ci(10-02): add vt-conformance + perf-gate jobs (HARDEN-02 D-23)
0045ce9 feat(10-02): add perf_probe example for HARDEN-02 perf gate (D-10)
c23f6c3 test(10-02): add HARDEN-02 VT conformance corpus — 8 scenarios, 17 tests
```
