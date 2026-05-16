---
phase: 4
slug: mux-tabs-splits
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-11
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test --workspace` over per-crate `tests/*.rs` integration files (matches Phase 1/2/3 conventions) |
| **Config file** | `Cargo.toml` (workspace) + per-crate `Cargo.toml` |
| **Quick run command** | `cargo test --workspace --tests -q` |
| **Full suite command** | `cargo test --workspace --tests --release` |
| **Estimated runtime** | ~35 s quick / ~70 s full (Phase 3 baseline was ~25 s; 12 new test files + 1 extension add ~10 s of integration time) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --workspace --tests -q`
- **After every plan wave:** Run `cargo test --workspace --tests -q` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo fmt --all -- --check` + arch-lint count `find crates -name "no_tokio_main.rs" -o -name "no_transport_discrimination.rs" | wc -l == 16`
- **Before `/gsd:verify-work`:** Full suite must be green + 9-item Phase 4 smoke matrix signed off
- **Max feedback latency:** ~35 seconds

---

## Per-Task Verification Map

> Plan IDs follow `04-NN`; task IDs are placeholders refined by `gsd-planner` in 04-NN-PLAN.md frontmatter.

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 04-01-01 | 01 | 1 | (infra) | wave-0 stubs | `cargo test --workspace --tests -q` | ❌ W0 (creates 12 stubs) | ⬜ pending |
| 04-01-02 | 01 | 1 | (infra) | arch-lint count | `find crates -name 'no_*.rs' \\| wc -l == 16` | ❌ W0 | ⬜ pending |
| 04-02-* | 02 | 2 | WIN-02 (mux types) | unit | `cargo test -p vector-mux --test mux_topology` | ❌ W0 | ⬜ pending |
| 04-02-* | 02 | 2 | WIN-02 (cascade) | unit | `cargo test -p vector-mux --test mux_close_cascade` | ❌ W0 | ⬜ pending |
| 04-02-* | 02 | 2 | WIN-02 (tab cycle) | unit | `cargo test -p vector-mux --test mux_tab_cycle` | ❌ W0 | ⬜ pending |
| 04-02-* | 02 | 2 | WIN-03 (split tree) | unit | `cargo test -p vector-mux --test split_tree` | ❌ W0 | ⬜ pending |
| 04-02-* | 02 | 2 | WIN-03 (focus dir) | unit | `cargo test -p vector-mux --test directional_focus` | ❌ W0 | ⬜ pending |
| 04-02-* | 02 | 2 | WIN-03 (nudge) | unit | `cargo test -p vector-mux --test split_resize_nudge` | ❌ W0 | ⬜ pending |
| 04-02-* | 02 | 2 | WIN-04 | arch-lint | `cargo test -p vector-term --test no_transport_discrimination` | ❌ W0 | ⬜ pending |
| 04-03-* | 03 | 3 | D-57 fg-process | integration (real PTY) | `cargo test -p vector-mux --test proc_name_tracking -- --include-ignored` | ❌ W0 | ⬜ pending |
| 04-03-* | 03 | 3 | D-63 cwd inherit | integration (real PTY) | `cargo test -p vector-mux --test cwd_inheritance -- --include-ignored` | ❌ W0 | ⬜ pending |
| 04-03-* | 03 | 3 | D-64 cwd fallback | unit | `cargo test -p vector-mux --test cwd_fallback` | ❌ W0 | ⬜ pending |
| 04-03-* | 03 | 3 | WIN-03 #3 | integration (real PTY + tput) | `cargo test -p vector-mux --test pane_resize_propagates -- --include-ignored` | ❌ W0 | ⬜ pending |
| 04-04-* | 04 | 4 | D-59/60/61/62 | unit (keymap) | extend `cargo test -p vector-input --test xterm_key_table` | ✅ (Phase 3 file) | ⬜ pending |
| 04-04-* | 04 | 4 | D-56 tabbing | mock-driven unit | `cargo test -p vector-app --test multi_window_tabbing` | ❌ W0 | ⬜ pending |
| 04-04-* | 04 | 4 | D-66 border | snapshot (offscreen) | `cargo test -p vector-render --test active_pane_border` | ❌ W0 | ⬜ pending |
| 04-05-* | 05 | 5 | all (sign-off) | manual smoke matrix | `checkpoint:human-verify` against the 9 items below | n/a | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

12 new test files seeded with `#[ignore = "Wave-0 stub"]` markers in Plan 04-01 (matching Phase 3 Plan 03-01 pattern); un-ignored as later plans land features.

- [ ] `crates/vector-mux/tests/mux_topology.rs` — WIN-02 (Cmd-T → tab/pane allocation invariants)
- [ ] `crates/vector-mux/tests/mux_tab_cycle.rs` — WIN-02 (Cmd-Shift-]/[ next/prev)
- [ ] `crates/vector-mux/tests/mux_close_cascade.rs` — WIN-02 (Cmd-W pane → tab → window → quit)
- [ ] `crates/vector-mux/tests/split_tree.rs` — WIN-03 (Cmd-D / Cmd-Shift-D tree mutation)
- [ ] `crates/vector-mux/tests/directional_focus.rs` — WIN-03 (Cmd-Opt-Arrow `get_pane_direction`)
- [ ] `crates/vector-mux/tests/split_resize_nudge.rs` — WIN-03 (Cmd-Shift-Arrow 1-cell ratio shift)
- [ ] `crates/vector-mux/tests/pane_resize_propagates.rs` — WIN-03 #3 (real PTY `tput cols` round-trip)
- [ ] `crates/vector-mux/tests/proc_name_tracking.rs` — D-57 (foreground process name via `tcgetpgrp`+`libproc::pidpath`)
- [ ] `crates/vector-mux/tests/cwd_inheritance.rs` — D-63 (`libproc::pidcwd` happy path)
- [ ] `crates/vector-mux/tests/cwd_fallback.rs` — D-64 ($HOME fallback when `pidcwd` errors)
- [ ] `crates/vector-term/tests/no_transport_discrimination.rs` — WIN-04 (grep invariant: zero `enum PaneSource`, zero `match transport.kind()`, zero `TransportKind::Local =>` in `vector-term/src/`)
- [ ] `crates/vector-render/tests/active_pane_border.rs` — D-66 (offscreen pixel snapshot showing 1-px border on viewport edge)
- [ ] `crates/vector-app/tests/multi_window_tabbing.rs` — D-56 (mock-asserts `set_tabbing_identifier` invoked on every Cmd-T window; visual is manual)
- [ ] Extend `crates/vector-input/tests/xterm_key_table.rs` (existing) — assert Cmd-Opt-Arrow / Cmd-Shift-Arrow / Cmd-D / Cmd-Shift-D / Cmd-T / Cmd-W / Cmd-Shift-]/[ return `None` from keymap (i.e., NOT sent to PTY; handled at app layer)

**Total new test files: 13** (12 new + 1 existing-file extension). Workspace test count target after Phase 4: ~210+ passing.

---

## Manual-Only Verifications

Plan 04-05 `checkpoint:human-verify` — 9-item smoke matrix (Phase 3 had its own 9-item; Phase 4 extends with tabs/splits and reaffirms 2 carryover items):

| # | Behavior | Requirement | Why Manual | Test Instructions |
|---|----------|-------------|------------|-------------------|
| 1 | Cmd-T spawns native NSWindow tab | WIN-02, D-56 | Visual: AppKit's tab bar rendering and grouping behavior is OS-controlled and can't be unit-tested | Launch Vector; press Cmd-T; confirm a new tab appears in the same NSWindow's tab group (not a separate window). Switch tabs via tab-bar click and Cmd-Shift-]. Note winit issue #2238 fallback: if first dynamic window doesn't group, manual NSWindow setTabbingMode kicks in transparently — verify behavior, not implementation. |
| 2 | Cmd-W cascade closes pane → tab → window → app | WIN-02, D-61 | Multi-step user interaction across distinct mux scopes | (a) Single pane in single tab in single window → Cmd-W should quit app. (b) Split horizontally → Cmd-W closes the focused pane only. (c) Two tabs, one pane each → Cmd-W on first tab leaves the window with one tab. |
| 3 | Cmd-D horizontal + Cmd-Shift-D vertical split + Cmd-Opt-Arrow focus | WIN-03, D-59 | Visual + tactile: split divider position + focus border movement | Cmd-D twice → 3 panes side-by-side; Cmd-Shift-D in middle → middle pane splits vertically; Cmd-Opt-Right / Cmd-Opt-Down routes focus directionally; border lights up on newly-focused pane. |
| 4 | `tput cols` round-trip after split + window resize | WIN-03 #3 | Real PTY behavior under live SIGWINCH | Open Vector, Cmd-D, run `tput cols` in each pane → numbers split roughly evenly. Drag window corner → re-run `tput cols` → numbers reflect new window width. |
| 5 | cwd inheritance via `proc_pidinfo` | D-63 | Real cwd lookup against a live shell PID | `cd ~/personal/vector` in pane 1; Cmd-D; new pane's prompt is in `~/personal/vector` (`pwd` confirms). Cmd-T from there; new tab also inherits. |
| 6 | N-pane idle CPU stays < 1% | RENDER-03 reaffirm under N panes | Activity Monitor reading over 60 s window | Open 4 splits; idle 60 s; Activity Monitor → Vector CPU < 1%. (Phase 3's RENDER-03 was single-pane; Phase 4 reaffirms with N panes.) |
| 7 | Tab title tracks foreground process | D-57 | Real `tcgetpgrp` + libproc polling timing visible only at runtime | Open zsh; tab title shows "zsh"; run `vim` → tab title becomes "vim" within 2 s; quit vim → returns to "zsh" within 2 s. |
| 8 | Active-pane border visible against dark + light backgrounds | D-66 | Visual contrast judgment vs accent color | With dark theme, focused pane shows 1–2 px accent border; click another pane → border moves. Inactive cursor renders as hollow outline (per Claude's-discretion default). |
| 9 | DPR change (Retina ↔ external monitor) with N panes open | RENDER-04 reaffirm under N panes | Hardware change required; tests atlas invalidation under multiple Compositors | Open 3 panes; drag window from built-in Retina to external non-Retina display (or vice versa); all panes re-rasterize sharp within a frame. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (13 stubs ready in Plan 04-01)
- [ ] No watch-mode flags (`cargo test` runs once and exits)
- [ ] Feedback latency < 35 s (workspace --tests -q)
- [ ] `nyquist_compliant: true` set in frontmatter after planner finalizes Plan 04-NN tasks
- [ ] Arch-lint count target: **16** (was 15; +1 for `no_transport_discrimination.rs`)

**Approval:** pending
