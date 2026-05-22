---
phase: 09-persistence-reconnect-tmux-auto-attach
plan: 04
subsystem: ui
tags: [wgpu, reconnect-ui, format_tab_title, vector-render, vector-mux]

requires:
  - phase: 09-persistence-reconnect-tmux-auto-attach
    provides: "Plan 09-01 Wave-0 scaffolds: PaneReconnecting/PaneReconnected UserEvent variants + reconnect_pass_render.rs ignored test stubs"
provides:
  - "ReconnectPass wgpu pipeline (24 px bar, 120/200 ms fade, 250 ms debounce) at crates/vector-render/src/reconnect_pass.rs"
  - "format_reconnect_text(profile, attempt, content_cells) helper implementing the UI-SPEC truncation contract + (attempt 9+) cap"
  - "PaneUiState enum (Active | Reconnecting) re-exported from vector-mux"
  - "format_tab_title extended with PaneUiState — DevTunnel + Reconnecting → [reconnecting]; Local panes never emit either badge"
  - "ChromePipelines.reconnect field ready for the Plan 09-05 render hook"
affects: [09-05, 09-06]

tech-stack:
  added: []
  patterns:
    - "Reconnect pipeline cloned (not shared) from ToastPass — single-mode struct, no `kind` branch; consistent with UI-SPEC anti-pattern note"
    - "format_reconnect_text emits U+2026 ellipsis (`\\u{2026}`) explicitly via escape, never three ASCII dots"
    - "format_tab_title takes PaneUiState as a 4th param so Plan 09-05 only needs to flip one arg at each call site"

key-files:
  created:
    - crates/vector-render/src/reconnect_pass.rs
  modified:
    - crates/vector-render/src/lib.rs
    - crates/vector-app/src/chrome.rs
    - crates/vector-app/src/app.rs
    - crates/vector-mux/src/pane.rs
    - crates/vector-mux/src/lib.rs
    - crates/vector-mux/tests/osc7_consumer.rs
    - crates/vector-app/tests/reconnect_pass_render.rs

key-decisions:
  - "ReconnectPass is a separate pipeline from ToastPass (UI-SPEC anti-pattern: sharing would force a kind branch through both)"
  - "format_reconnect_text returns Option<String> — None when content_cells < 18 so callers can skip render"
  - "Attempt cap shown as the literal string `9+` once attempt >= 10 (UI-SPEC §Copywriting overflow rule)"
  - "format_tab_title takes ui_state as positional 4th arg (not Option<PaneUiState>) so the type system forces every caller to declare intent"

patterns-established:
  - "Reconnect alpha curve mirrors toast_pass::alpha_at with reconnect-specific constants (no shared helper to keep the two motion identities independently tunable)"
  - "Bar background takes bg_rgba from the caller so light/dark theme swap happens at the render hook, not inside the pipeline"

requirements-completed: [PERSIST-01]

duration: 3min
completed: 2026-05-22
---

# Phase 9 Plan 04: Reconnect UI primitives Summary

**ReconnectPass wgpu pipeline, format_reconnect_text truncation helper, PaneUiState-aware format_tab_title, and ChromePipelines.reconnect wiring — all the visual primitives Plan 09-05 needs to render the inline status bar.**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-05-22
- **Completed:** 2026-05-22
- **Tasks:** 3
- **Files modified:** 7 (1 created, 6 modified)

## Accomplishments

- New `ReconnectPass` pipeline in `crates/vector-render/src/reconnect_pass.rs` (~150 lines): bar background via `ChromeQuadPipeline`, `reconnect_layout()`, `alpha_at()` curve, four `pub const`s matching UI-SPEC.
- `format_reconnect_text(profile, attempt, content_cells) -> Option<String>` enforces every truncation rule in UI-SPEC §Copywriting (≥40 cells full profile, 28..40 middle-truncate, 18..28 omit profile, <18 None) + `(attempt 9+)` cap.
- `PaneUiState { Active, Reconnecting }` enum added to `vector-mux` and re-exported alongside `format_tab_title`.
- `format_tab_title` extended with the new `ui_state` arg; every workspace call site (vector-app/app.rs:1720, vector-mux tests) passes `PaneUiState::Active` to preserve existing `[remote]` behavior.
- `ChromePipelines.reconnect: ReconnectPass` field initialized in `ChromePipelines::new`.
- UI test suite in `crates/vector-app/tests/reconnect_pass_render.rs` rewritten: 6 active tests (UI-T1..T6) + 2 still-ignored placeholders for Plan 09-05 (input-lock + tab-badge integration).

## Task Commits

1. **Task 1: ReconnectPass + format_reconnect_text + re-exports** — `7a72f60` (feat)
2. **Task 2: ChromePipelines.reconnect + PaneUiState + format_tab_title** — `83226e2` (feat)
3. **Task 3: reconnect_pass_render.rs UI tests** — `432703f` (test)

## Files Created/Modified

- `crates/vector-render/src/reconnect_pass.rs` (NEW) — ReconnectPass pipeline, layout, alpha curve, format_reconnect_text
- `crates/vector-render/src/lib.rs` — module declaration + re-exports
- `crates/vector-app/src/chrome.rs` — ChromePipelines.reconnect field
- `crates/vector-app/src/app.rs` — line 1720 call site now passes PaneUiState::Active
- `crates/vector-mux/src/pane.rs` — PaneUiState enum, format_tab_title signature change, 3 new tests
- `crates/vector-mux/src/lib.rs` — re-export PaneUiState
- `crates/vector-mux/tests/osc7_consumer.rs` — 3 call sites updated
- `crates/vector-app/tests/reconnect_pass_render.rs` — 6 active + 2 ignored UI tests

## Decisions Made

- ReconnectPass kept separate from ToastPass (UI-SPEC §S1 anti-pattern). Both have nearly the same shape but different lifecycles (window-scoped/auto-dismiss vs pane-scoped/persistent).
- `bg_rgba` is a parameter to `ReconnectPass::update` rather than baked-in. Lets the caller pick dark/light surface color from `ChromePalette` per-frame without recreating the pipeline.
- `PaneUiState::Active` made the positional default at every existing call site (rather than `Default` or `Option`) so the type system catches a missing flip in Plan 09-05.

## Deviations from Plan

None — plan executed exactly as written. The plan's `format_reconnect_text` reference implementation was used verbatim (only swapped `…` literals for `\u{2026}` escapes for grep-safety).

## Issues Encountered

None.

## User Setup Required

None.

## Next Phase Readiness

Plan 09-05 (wave 4) can now:
1. Call `chrome_pipelines.reconnect.update(...)` + `.draw(...)` from the per-pane render loop.
2. Call `format_reconnect_text(profile, attempt, content_cells)` for glyph compositing on top of the bar.
3. Flip `PaneUiState::Active` ↔ `PaneUiState::Reconnecting` at the `format_tab_title` call site in `app.rs` when `PaneReconnecting`/`PaneReconnected` UserEvents fire.

The two still-`#[ignore]`d tests in `reconnect_pass_render.rs` (`input_locked_in_reconnecting_state`, `tab_badge_during_reconnect`) are Plan 09-05's deliverables.

## Self-Check: PASSED

- `crates/vector-render/src/reconnect_pass.rs` FOUND (~150 lines)
- Commit `7a72f60` FOUND
- Commit `83226e2` FOUND
- Commit `432703f` FOUND
- `cargo build --workspace` green
- `cargo test -p vector-render reconnect_pass::constants_tests` 1 passed
- `cargo test -p vector-mux pane::tests::format_tab_title` 6 passed
- `cargo test -p vector-app --test reconnect_pass_render` 6 passed / 2 ignored

---
*Phase: 09-persistence-reconnect-tmux-auto-attach*
*Completed: 2026-05-22*
