---
phase: 04-mux-tabs-splits
plan: 01
subsystem: vector-mux
tags: [wave-0, mux-ids, spawned-pane, libproc, win-04, arch-lint, d-38, d-67]

# Dependency graph
requires:
  - phase: 02-headless-terminal-core
    plan: 04
    provides: LocalDomain + LocalTransport + D-38 Domain/PtyTransport traits (FINAL, untouched here)
  - phase: 03-gpu-renderer-first-paint
    plan: 01
    provides: Wave-0 stub-seeding precedent (17 stubs across vector-render/fonts/input/app)
provides:
  - "Workspace dep: libproc 0.14 pinned in [workspace.dependencies]"
  - "vector-mux: PaneId / TabId / WindowId Copy+Hash newtypes + IdAllocator (D-67)"
  - "vector-mux: SpawnedPane { transport, pid: Option<i32>, master_fd: Option<RawFd> } — Phase-4-internal return shape"
  - "vector-mux: LocalDomain::spawn_local(SpawnCommand) -> Result<SpawnedPane> — inherent method, NOT a trait method"
  - "vector-pty: LocalPty::child_pid() -> Option<i32> + LocalPty::master_raw_fd() -> Option<RawFd> accessors"
  - "10 vector-mux integration test stubs (Plans 04-02 + 04-03 own un-ignores)"
  - "vector-term/tests/no_transport_discrimination.rs WIN-04 grep arch-lint (Plan 04-02 un-ignores)"
  - "vector-render/tests/active_pane_border.rs D-66 stub (Plan 04-04)"
  - "vector-app/tests/multi_window_tabbing.rs D-56 stub (Plan 04-04)"
  - "vector-input/tests/xterm_key_table.rs extended with 14 Cmd-* stubs pre-named to Plan 04-04 MuxCommand assertion targets"
  - "Arch-lint count: 15 -> 16 (no_transport_discrimination.rs added)"
affects: [04-02 (un-ignores 7 stubs + WIN-04), 04-03 (un-ignores 4 stubs + real-PTY proc tracking + cwd), 04-04 (un-ignores xterm Cmd-* + active_pane_border + multi_window_tabbing)]

# Tech tracking
tech-stack:
  added:
    - "libproc 0.14.11 (workspace) — D-57 fg-process tracking + D-63 cwd inheritance for Plan 04-03"
  patterns:
    - "Non-trait extension point: LocalDomain::spawn_local inherent method coexists with Domain::spawn trait method — D-38 trait surface byte-identical, D-67 Mux gets pid + master_fd without touching the seam"
    - "SpawnedPane field types follow underlying primitive Options (Option<RawFd>, Option<i32>) rather than panic-on-None — Codespace/DevTunnel (Phases 7/8) will produce pid=None naturally"
    - "Wave-0 stub seeding via #[ignore = \"Wave-0 stub: Plan 04-NN\"] reason strings (workspace clippy::ignore_without_reason holds)"
    - "Pre-naming Plan-04-04 keymap tests to MuxCommand assertion targets so un-ignoring is a 1-line annotation flip + assertion body rewrite"

key-files:
  created:
    - crates/vector-mux/src/ids.rs
    - crates/vector-mux/src/spawned_pane.rs
    - crates/vector-mux/tests/mux_topology.rs
    - crates/vector-mux/tests/mux_tab_cycle.rs
    - crates/vector-mux/tests/mux_close_cascade.rs
    - crates/vector-mux/tests/split_tree.rs
    - crates/vector-mux/tests/directional_focus.rs
    - crates/vector-mux/tests/split_resize_nudge.rs
    - crates/vector-mux/tests/pane_resize_propagates.rs
    - crates/vector-mux/tests/proc_name_tracking.rs
    - crates/vector-mux/tests/cwd_inheritance.rs
    - crates/vector-mux/tests/cwd_fallback.rs
    - crates/vector-term/tests/no_transport_discrimination.rs
    - crates/vector-render/tests/active_pane_border.rs
    - crates/vector-app/tests/multi_window_tabbing.rs
  modified:
    - Cargo.toml (workspace libproc dep)
    - Cargo.lock
    - crates/vector-mux/Cargo.toml (libproc + parking_lot)
    - crates/vector-mux/src/lib.rs (re-export ids + spawned_pane modules)
    - crates/vector-mux/src/local_domain.rs (spawn_local inherent method)
    - crates/vector-pty/src/local.rs (child_pid + master_raw_fd accessors)
    - crates/vector-input/tests/xterm_key_table.rs (14 Cmd-* stubs)

key-decisions:
  - "Workspace libproc dep is the ONLY new dep — Wave-0 sets the floor."
  - "SpawnedPane.master_fd is Option<RawFd> not bare RawFd: portable_pty::MasterPty::as_raw_fd returns Option<RawFd>. Plan's <interfaces> sketch said bare RawFd; we follow the underlying primitive truthfully. Plan 04-03's tcgetpgrp call site will short-circuit on None (trace-log + fall back to D-64 $HOME) rather than panic."
  - "SpawnedPane.pid is Option<i32> for symmetry: Codespace/DevTunnel (Phases 7/8) inherently have no local child PID, and the same shape carries through. LocalPty::child_pid() casts portable_pty's u32 -> i32 via try_from for libc::pid_t parity."
  - "LocalDomain::spawn_local kept as `async fn` (#[allow(clippy::unused_async)]) to mirror Domain::spawn signature — Phase 7 CodespaceDomain::spawn_local equivalent will be truly async."
  - "Domain trait impl (`LocalDomain::spawn`) kept exactly as Plan 02-04 shipped — NOT refactored to call spawn_local. Refactor would risk Plan 02-04's 8 trait_object_safety.rs tests; the duplication is ~5 lines of pty construction and is acceptable."

patterns-established:
  - "Phase 4 has a parallel pair: trait surface (Domain::spawn -> Box<dyn PtyTransport>) for downstream phases AND inherent extension (LocalDomain::spawn_local -> SpawnedPane) for Phase 4's own Mux consumers. Phase 7/8 will follow the same pattern: CodespaceDomain::spawn_codespace -> SpawnedPane equivalent without touching the D-38 trait."
  - "All Wave-0 stub test bodies use `panic!(\"Wave-0 stub — implemented by Plan 04-NN\")` not `assert!(false, ...)` — panic gives a single-line traceback when accidentally un-ignored, no `unreachable_code` lint risk."

requirements-completed: []
# WIN-02 / WIN-03 / WIN-04 progress: stubs seeded; un-ignored by Plans 04-02..04-04.

# Metrics
duration: 4min
completed: 2026-05-12
---

# Phase 4 Plan 01: Wave-0 mux scaffold + libproc dep + stub seeding Summary

**Pin libproc 0.14, add PaneId/TabId/WindowId/IdAllocator/SpawnedPane in vector-mux without touching the D-38 trait surface, expose LocalPty::child_pid() + master_raw_fd() accessors, add LocalDomain::spawn_local() as an inherent (non-trait) method that returns SpawnedPane, seed all 12 Wave-0 stub test files plus extend xterm_key_table.rs with 14 Cmd-* keymap stubs pre-named to Plan 04-04's MuxCommand assertion targets, and ship the WIN-04 grep arch-lint test in red so Plan 04-02 can flip it green. Workspace stays at 176 passing tests (was 175; +1 from ids.rs unit test); ignored count rises 0 -> 27 (13 new stub files + 14 xterm_key_table cases). Arch-lint file count 15 -> 16 via the new no_transport_discrimination.rs. D-38 Domain/PtyTransport trait files byte-identical to Phase 2.**

## Performance

- **Duration:** ~4 min (242s wall clock)
- **Started:** 2026-05-12T03:02:42Z
- **Completed:** 2026-05-12T03:06:44Z
- **Tasks:** 2 (each committed atomically)
- **Test count:** 176 passing / 0 failed / 27 ignored (baseline was 175/0/0)

## Accomplishments

- `libproc 0.14` pinned in `[workspace.dependencies]` (Cargo.toml line 36, alphabetically between `etagere` and `objc2`).
- `vector-mux` declares `libproc.workspace = true` + `parking_lot.workspace = true` in `[dependencies]`.
- `crates/vector-mux/src/ids.rs` exports `PaneId(pub u64)`, `TabId(pub u64)`, `WindowId(pub u64)` — all `Copy + Hash + Eq + Debug` per D-67 — plus an `IdAllocator { next: AtomicU64 }` shared monotonic allocator with `allocate_pane`/`allocate_tab`/`allocate_window`. One unit test asserts monotonic distinctness.
- `crates/vector-mux/src/spawned_pane.rs` ships `pub struct SpawnedPane { pub transport: Box<dyn PtyTransport>, pub pid: Option<i32>, pub master_fd: Option<RawFd> }` — the universal Phase-4-internal return shape.
- `LocalDomain::spawn_local(SpawnCommand) -> Result<SpawnedPane>` added as an inherent method (NOT a trait method); the existing `impl Domain for LocalDomain { async fn spawn(...) -> Box<dyn PtyTransport> }` is byte-identical to Plan 02-04.
- `LocalPty::child_pid() -> Option<i32>` and `LocalPty::master_raw_fd() -> Option<RawFd>` added — sourced directly from `portable_pty::Child::process_id()` and `MasterPty::as_raw_fd()`.
- `crates/vector-mux/src/lib.rs` re-exports `PaneId, TabId, WindowId, IdAllocator, SpawnedPane` at crate root; existing `Domain`/`PtyTransport`/`LocalDomain` re-exports untouched.
- 12 new test files (10 in vector-mux/tests/, 1 in vector-term/tests/, 1 in vector-render/tests/, 1 in vector-app/tests/) all `#[ignore = "Wave-0 stub: Plan 04-NN"]`'d with the plan that owns each un-ignore. **Note: 12 = 10 + 1 + 1 + 1 - 1 = 12 (mux=10, term=1, render=1, app=1) confirmed by `find` count = 13 includes the WIN-04 grep test that lives in vector-term, total new files = 12 + 1 vector-term grep = 13; the "12 new files" wording in the plan groups them as "12 new + WIN-04 grep file = 13 total new files".** Final breakdown: 13 new test files created on disk this plan + 14 stub cases appended to xterm_key_table.rs (existing file).
- `crates/vector-input/tests/xterm_key_table.rs` extended with 14 new `#[ignore = "Wave-0 stub: Plan 04-04"]` stubs, each named to Plan 04-04's expected MuxCommand assertion target (cmd_t_returns_mux_new_tab, cmd_d_returns_mux_split_horizontal, cmd_shift_d_returns_mux_split_vertical, cmd_w_returns_mux_close_pane, cmd_shift_close_bracket_returns_mux_next_tab, cmd_shift_open_bracket_returns_mux_prev_tab, cmd_opt_{left,right,up,down}_returns_mux_focus_{dir}, cmd_shift_{left,right,up,down}_returns_mux_resize_nudge_{dir}).
- WIN-04 arch-lint test `crates/vector-term/tests/no_transport_discrimination.rs` ships with the verbatim FORBIDDEN array from RESEARCH.md §"Example 3" (7 patterns: enum PaneSource, TransportKind::Local|Codespace|DevTunnel, transport.kind(), .kind() == TransportKind, match transport.kind) + recursive walker. `#[ignore = "Wave-0 stub: Plan 04-02 un-ignores"]` until Plan 04-02 audits vector-term.
- Arch-lint count delta: 15 → 16. `find crates -name 'no_tokio_main.rs' -o -name 'no_transport_discrimination.rs' | wc -l` returns 16.
- `cargo build --workspace --tests` clean. `cargo clippy --workspace --all-targets -- -D warnings` clean. `cargo fmt --all -- --check` clean. `cargo test --workspace --tests -q` reports 176 passed / 0 failed / 27 ignored (was 175/0/0 at the close of Phase 3).
- D-38 invariant held: `git diff` of `crates/vector-mux/src/domain.rs` and `crates/vector-mux/src/transport.rs` against pre-Plan-04-01 HEAD shows zero hunks.

## Wave-0 Stub Map

| File | Owning Plan | Test name | Test type |
|------|-------------|-----------|-----------|
| crates/vector-mux/tests/mux_topology.rs | 04-02 | create_tab_allocates_unique_ids | unit |
| crates/vector-mux/tests/mux_tab_cycle.rs | 04-02 | tab_cycle_next_prev_wraps | unit |
| crates/vector-mux/tests/mux_close_cascade.rs | 04-02 | cmd_w_cascade_pane_tab_window_quit | unit |
| crates/vector-mux/tests/split_tree.rs | 04-02 | split_horizontal_then_vertical_mutates_tree | unit |
| crates/vector-mux/tests/directional_focus.rs | 04-02 | get_pane_direction_right_returns_neighbor | unit |
| crates/vector-mux/tests/split_resize_nudge.rs | 04-02 | cmd_shift_arrow_nudges_ratio_one_cell | unit |
| crates/vector-term/tests/no_transport_discrimination.rs | 04-02 | vector_term_does_not_discriminate_on_transport_kind | grep arch-lint |
| crates/vector-mux/tests/pane_resize_propagates.rs | 04-03 | tput_cols_round_trip_after_split | integration (real PTY) |
| crates/vector-mux/tests/proc_name_tracking.rs | 04-03 | fg_process_name_transitions_zsh_to_sleep | integration (real PTY) |
| crates/vector-mux/tests/cwd_inheritance.rs | 04-03 | pidcwd_returns_shell_pwd | integration (real PTY) |
| crates/vector-mux/tests/cwd_fallback.rs | 04-03 | falls_back_to_home_on_pidcwd_err | unit |
| crates/vector-render/tests/active_pane_border.rs | 04-04 | border_color_some_renders_one_px_border | offscreen pixel snapshot |
| crates/vector-app/tests/multi_window_tabbing.rs | 04-04 | set_tabbing_identifier_called_on_cmd_t | mock-driven unit |
| crates/vector-input/tests/xterm_key_table.rs (14 stubs) | 04-04 | cmd_t/d/shift_d/w + shift_close_bracket/shift_open_bracket + cmd_opt_{l,r,u,d} + cmd_shift_{l,r,u,d} | keymap unit |

Total: 13 new test files + 14 Cmd-* stub cases in xterm_key_table.rs.

## SpawnedPane Rationale (D-38 + D-67 fidelity)

Plan 04-02..04 callers need three things from a freshly-spawned local pane: the transport (for I/O), the child PID (for D-57 tcgetpgrp / D-63 libproc::pidcwd), and the master PTY fd (for D-57 tcgetpgrp on the *master* side, used to discover the foreground process group regardless of who the child currently is).

The Phase-2 D-38 contract returns `Box<dyn PtyTransport>` — it does NOT carry pid or master_fd. Three options were considered:

1. **Extend the Domain trait** (e.g., return `(Box<dyn PtyTransport>, Option<i32>, Option<RawFd>)` or a struct). Rejected: Phase 7 CodespaceDomain inherently has no local pid/fd; the trait would need an `Option<...>` shape that's only meaningful for `LocalDomain`. D-38 was locked as "Phases 7/8/9 fill bodies, not reshape".
2. **Downcast `&dyn PtyTransport` to `&LocalTransport`** at the Mux call site. Rejected: `Any` + `downcast_ref` against a trait-object adds runtime cost and clippy noise; also fails for Phase 7 transports.
3. **Add an inherent method on LocalDomain that returns SpawnedPane, separate from the trait method.** Adopted. The trait `Domain::spawn` stays D-38-final; Mux call sites that need local-specific data call `LocalDomain::spawn_local` directly (it's a non-trait method on the concrete type). Codespace/DevTunnel will follow the same pattern in Phase 7/8 with their own `spawn_codespace` / `spawn_dev_tunnel` equivalents — each returning a SpawnedPane with `pid: None` and `master_fd: None`.

This preserves CONTEXT.md D-67's "never touches the traits" promise.

## LocalPty Field-Touchpoint Notes for Plan 04-03

While wiring `child_pid()` + `master_raw_fd()`:

- **`MasterPty::as_raw_fd()` returns `Option<RawFd>`**, not bare `RawFd`. Documented in portable-pty 0.9.0's `lib.rs:114`: "If get_termios() and process_group_leader() are both implemented and return Some, then as_raw_fd() should return the same underlying fd". On macOS / Unix native PTY this returns Some; on platforms without a Unix fd it returns None. **Plan 04-03 should handle `master_fd: None` as a tracking-impossible state (trace-log, fall back to "shell" as the process name).**
- **`Child::process_id()` returns `Option<u32>`** that becomes None after `Child::wait()` consumes the child. **Plan 04-03's polling loop should re-check pid each tick** — pid going None means the pane exited and the foreground-process tracker should stop polling for that pane.
- **`LocalPty.child` is `Option<Box<dyn Child + Send + Sync>>`** (not bare Box) because `wait()` does `self.child.take()`. The new `child_pid()` accessor reads `self.child.as_ref().and_then(|c| c.process_id())` to gracefully handle the post-wait state.
- **No new `child_pid: Option<i32>` cached field added on LocalPty.** The plan's Task 1 step 3 suggested "if `child` is wrapped in `Mutex`, pull the pid out at spawn time and cache it". Since `child` is just `Option<Box<_>>` (no Mutex), the as_ref() path is fine. **Plan 04-03 can rely on `child_pid()` being cheap to call.**

## Decisions Made

- **`SpawnedPane.master_fd: Option<RawFd>` instead of bare `RawFd` (plan's `<interfaces>` sketch).** Forced by portable-pty's `MasterPty::as_raw_fd() -> Option<RawFd>` underlying signature. Returning Option upstream is cleaner than `expect()`ing in `spawn_local`; Plan 04-03's call sites will short-circuit on None.
- **`SpawnedPane.pid: Option<i32>` for symmetry with the Codespace/DevTunnel future** (which have no local PID); also lets the field gracefully reflect post-`wait()` state.
- **`LocalDomain::spawn_local` kept as `async fn`** (with `#[allow(clippy::unused_async)]`) to mirror `Domain::spawn`'s signature. Phase 7's `CodespaceDomain::spawn_*` equivalent will be truly async (network calls). Keeping `spawn_local` async maintains call-site symmetry.
- **No refactor of `Domain::spawn` to delegate to `spawn_local`.** The plan said optional; we kept the existing impl byte-identical to minimize risk to Plan 02-04's 8 `trait_object_safety.rs` tests. The two methods duplicate ~5 lines of `PtySpawnCommand` construction and `Box::new(LocalTransport(pty))` — acceptable.
- **`IdAllocator` is a single shared `AtomicU64` for now.** The plan called out per-kind counters as a Plan-04-02 refinement; current shape gives "ID-N is the Nth allocation regardless of kind" semantics, which is sufficient for compile-time wiring.

## Task Commits

1. **Task 1: Workspace + crate deps + LocalPty/LocalDomain extension + SpawnedPane** — `d7d5b94` (feat)
2. **Task 2: Seed 12 Wave-0 stub files + 14 Cmd-* keymap stubs + WIN-04 grep** — `75ac3d3` (test)

## Files Created/Modified

### Created (15)

- `crates/vector-mux/src/ids.rs`
- `crates/vector-mux/src/spawned_pane.rs`
- `crates/vector-mux/tests/mux_topology.rs`
- `crates/vector-mux/tests/mux_tab_cycle.rs`
- `crates/vector-mux/tests/mux_close_cascade.rs`
- `crates/vector-mux/tests/split_tree.rs`
- `crates/vector-mux/tests/directional_focus.rs`
- `crates/vector-mux/tests/split_resize_nudge.rs`
- `crates/vector-mux/tests/pane_resize_propagates.rs`
- `crates/vector-mux/tests/proc_name_tracking.rs`
- `crates/vector-mux/tests/cwd_inheritance.rs`
- `crates/vector-mux/tests/cwd_fallback.rs`
- `crates/vector-term/tests/no_transport_discrimination.rs`
- `crates/vector-render/tests/active_pane_border.rs`
- `crates/vector-app/tests/multi_window_tabbing.rs`

### Modified (6 + Cargo.lock)

- `Cargo.toml` — added `libproc = "0.14"` to `[workspace.dependencies]`
- `crates/vector-mux/Cargo.toml` — added `libproc.workspace = true` + `parking_lot.workspace = true`
- `crates/vector-mux/src/lib.rs` — added `pub mod ids; pub mod spawned_pane;` + re-exports
- `crates/vector-mux/src/local_domain.rs` — added inherent `spawn_local` method
- `crates/vector-pty/src/local.rs` — added `child_pid()` + `master_raw_fd()` accessors
- `crates/vector-input/tests/xterm_key_table.rs` — appended 14 Cmd-* stub cases
- `Cargo.lock` — libproc 0.14.11 + transitive deps resolved

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `MasterPty::as_raw_fd()` returns `Option<RawFd>`, not bare `RawFd`**

- **Found during:** Task 1, after writing `LocalPty::master_raw_fd() -> RawFd { self.master.as_raw_fd() }` per the plan's `<interfaces>` sketch.
- **Issue:** portable-pty 0.9.0's `MasterPty::as_raw_fd(&self)` is typed `-> Option<RawFd>` (verified via `cargo doc -p portable-pty`). The plan's sketched signature `RawFd` would have required an `expect()` that panics on platforms where the fd isn't exposable.
- **Fix:** Changed `LocalPty::master_raw_fd()` to return `Option<RawFd>`, and made `SpawnedPane.master_fd` an `Option<RawFd>` field. Plan 04-03's tcgetpgrp call site will trace-log + fall back to D-64 $HOME on `None` instead of panicking.
- **Files modified:** `crates/vector-pty/src/local.rs`, `crates/vector-mux/src/spawned_pane.rs`
- **Committed in:** `d7d5b94`

**2. [Rule 1 - Bug] Clippy `unused_async` on `LocalDomain::spawn_local`**

- **Found during:** Task 1 clippy run.
- **Issue:** Workspace `clippy::pedantic` warns about `async fn` with no `await`. `spawn_local`'s current body is fully synchronous (PTY spawn is blocking-friendly via portable-pty 0.9).
- **Fix:** Added `#[allow(clippy::unused_async)]` with a doc comment explaining the `async` keyword preserves call-site symmetry with `Domain::spawn` (which is async-trait-bound and will be truly async for Phase 7 CodespaceDomain).
- **Files modified:** `crates/vector-mux/src/local_domain.rs`
- **Committed in:** `d7d5b94`

**3. [Rule 1 - Bug] rustfmt rewraps `child_pid()`'s chained `.and_then()` calls**

- **Found during:** Task 1 `cargo fmt --all -- --check`.
- **Issue:** Single-line `self.child.as_ref().and_then(|c| c.process_id()).and_then(|u| i32::try_from(u).ok())` exceeded rustfmt max width.
- **Fix:** Let `cargo fmt --all` apply its 4-line wrap.
- **Files modified:** `crates/vector-pty/src/local.rs`
- **Committed in:** `d7d5b94`

---

**Total deviations:** 3 auto-fixed (1 Rule 3 underlying-API-shape, 2 Rule 1 lint/format compliance).

**Impact on plan:** Deviation #1 is substantive — `SpawnedPane.master_fd: Option<RawFd>` vs the plan's `RawFd`. Documented in Decisions and the Plan-04-03 hand-off section.

## Issues Encountered

None blocking. The portable-pty `Option<RawFd>` discovery was the only genuine integration surprise — caught at compile time, fixed in <1 minute.

## Verification Results

```
cargo build --workspace --tests                                    ✓ clean
cargo clippy --workspace --all-targets -- -D warnings              ✓ clean
cargo fmt --all -- --check                                          ✓ clean
cargo test --workspace --tests -q                                  ✓ 176 passed / 0 failed / 27 ignored
git diff HEAD~2 -- crates/vector-mux/src/domain.rs crates/vector-mux/src/transport.rs   ✓ zero hunks (D-38 invariant)
find crates -name 'no_tokio_main.rs' -o -name 'no_transport_discrimination.rs' | wc -l   ✓ 16
grep -c '^libproc' Cargo.toml                                       ✓ 1
grep -c 'libproc.workspace' crates/vector-mux/Cargo.toml            ✓ 1
grep -n 'pub fn child_pid' crates/vector-pty/src/local.rs           ✓ 1 match
grep -n 'pub fn master_raw_fd' crates/vector-pty/src/local.rs       ✓ 1 match
grep -n 'pub async fn spawn_local' crates/vector-mux/src/local_domain.rs   ✓ 1 match
grep -n 'pub struct SpawnedPane' crates/vector-mux/src/spawned_pane.rs     ✓ 1 match
grep -c 'ignore = "Wave-0 stub: Plan 04-04"' crates/vector-input/tests/xterm_key_table.rs   ✓ 14
```

## Hand-off Notes for Downstream Plans

### Plan 04-02 (Wave 2: split tree + mux topology + WIN-04 audit)

- **Un-ignore 7 stubs:**
  - vector-mux/tests/mux_topology.rs
  - vector-mux/tests/mux_tab_cycle.rs
  - vector-mux/tests/mux_close_cascade.rs
  - vector-mux/tests/split_tree.rs
  - vector-mux/tests/directional_focus.rs
  - vector-mux/tests/split_resize_nudge.rs
  - vector-term/tests/no_transport_discrimination.rs (the WIN-04 grep — flip green)
- **Construct Mux, Window, Tab, PaneNode** atop the `PaneId/TabId/WindowId/IdAllocator` already exported here.
- **The 14 Cmd-* xterm_key_table stubs are NOT yours** — Plan 04-04 owns the keymap encoding work. Leave them ignored.

### Plan 04-03 (Wave 3: per-pane PTY actors + proc tracking + cwd inheritance)

- **Un-ignore 4 stubs:**
  - vector-mux/tests/pane_resize_propagates.rs (real PTY, `-- --include-ignored`)
  - vector-mux/tests/proc_name_tracking.rs (real PTY, `-- --include-ignored`)
  - vector-mux/tests/cwd_inheritance.rs (real PTY, `-- --include-ignored`)
  - vector-mux/tests/cwd_fallback.rs (unit, mocked pidcwd)
- **`SpawnedPane.master_fd` is `Option<RawFd>`, not bare `RawFd`.** On `None`: trace-log + fall back to D-64 (`$HOME` cwd, "shell" as the process name).
- **`SpawnedPane.pid` becomes None after `transport.wait()` consumes the child.** Polling loops should treat pid going None as "pane exited, stop polling".
- **libproc 0.14 is already at workspace level** — declare `libproc.workspace = true` in any new crate that consumes it; vector-mux already has it.
- **Use `LocalDomain::spawn_local`** (not `Domain::spawn`) when constructing local panes inside Mux — you need the pid + master_fd that only `SpawnedPane` carries.

### Plan 04-04 (Wave 4: keymap + active-pane border + multi-window tabbing)

- **Un-ignore 16 stubs:**
  - vector-render/tests/active_pane_border.rs
  - vector-app/tests/multi_window_tabbing.rs
  - All 14 Cmd-* stubs in vector-input/tests/xterm_key_table.rs (names already match your MuxCommand assertion targets)
- **The 14 stubs panic until you rewrite each body to** `assert_eq!(encode(...), Some(EncodedKey::Mux(MuxCommand::*)))`. The `EncodedKey`/`MuxCommand`/`Direction` types don't exist yet — your Task 1 introduces them in vector-input.

### Plan 04-05 (Wave 5: manual smoke matrix sign-off)

- **No stubs to un-ignore.** Your `checkpoint:human-verify` runs the 9-item smoke matrix from VALIDATION.md against the cumulative Plan-04-01..04 implementation.

## Next Phase Readiness

- Plan 04-01 closes Phase 4 Wave 1. Plans 04-02..05 can start from green-bar (176 passed, 0 failed, 27 cleanly-ignored).
- D-38 invariant held (zero hunks in `domain.rs` / `transport.rs` since Plan 02-04).
- Arch-lint count at the new Phase-4 target of 16.
- No blockers identified.

## Self-Check: PASSED

All claimed files exist:

- crates/vector-mux/src/ids.rs — FOUND
- crates/vector-mux/src/spawned_pane.rs — FOUND
- crates/vector-mux/tests/mux_topology.rs — FOUND
- crates/vector-mux/tests/mux_tab_cycle.rs — FOUND
- crates/vector-mux/tests/mux_close_cascade.rs — FOUND
- crates/vector-mux/tests/split_tree.rs — FOUND
- crates/vector-mux/tests/directional_focus.rs — FOUND
- crates/vector-mux/tests/split_resize_nudge.rs — FOUND
- crates/vector-mux/tests/pane_resize_propagates.rs — FOUND
- crates/vector-mux/tests/proc_name_tracking.rs — FOUND
- crates/vector-mux/tests/cwd_inheritance.rs — FOUND
- crates/vector-mux/tests/cwd_fallback.rs — FOUND
- crates/vector-term/tests/no_transport_discrimination.rs — FOUND
- crates/vector-render/tests/active_pane_border.rs — FOUND
- crates/vector-app/tests/multi_window_tabbing.rs — FOUND
- Cargo.toml (modified) — FOUND
- crates/vector-mux/Cargo.toml (modified) — FOUND
- crates/vector-mux/src/lib.rs (modified) — FOUND
- crates/vector-mux/src/local_domain.rs (modified) — FOUND
- crates/vector-pty/src/local.rs (modified) — FOUND
- crates/vector-input/tests/xterm_key_table.rs (modified) — FOUND

All claimed commits exist:

- d7d5b94 — FOUND (Task 1)
- 75ac3d3 — FOUND (Task 2)

---
*Phase: 04-mux-tabs-splits*
*Plan: 01*
*Completed: 2026-05-12*
