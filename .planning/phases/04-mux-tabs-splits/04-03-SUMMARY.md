---
phase: 04-mux-tabs-splits
plan: 03
subsystem: vector-mux + vector-app
tags: [wave-3, per-pane-pty-actor, joinset, coalesce-buffer, proc-tracker, cwd-inheritance, d-57, d-63, d-64, win-02, win-03]

# Dependency graph
requires:
  - phase: 04-mux-tabs-splits
    plan: 01
    provides: SpawnedPane + LocalDomain::spawn_local + LocalPty::child_pid/master_raw_fd + libproc workspace dep
  - phase: 04-mux-tabs-splits
    plan: 02
    provides: Mux singleton + Window/Tab/PaneNode topology + split_tree pure algorithms + Pane::take_transport
provides:
  - "vector-mux::cwd::inherit_cwd(parent_pid) + inherit_cwd_with(pid, home_env) seam (D-63 / D-64)"
  - "vector-mux::cwd::pidcwd cfg(target_os) shim — libc::proc_pidinfo+PROC_PIDVNODEPATHINFO on macOS; libproc::pidcwd on Linux. Compensates for libproc 0.14's pidcwd-not-implemented-for-macos limitation."
  - "vector-mux::proc_tracker::proc_name_poll_loop generic over FnMut(PaneId, String) emit callback (avoids winit dep in vector-mux)"
  - "vector-mux::proc_tracker::spawn_proc_tracker tokio task spawn helper"
  - "vector-mux::Mux::create_tab_async / split_pane_async / resize_window — async I/O wrappers around LocalDomain::spawn_local + split_tree::redistribute"
  - "vector-mux::Pane::shell_pid / master_fd accessors"
  - "vector-app::PtyActorRouter — tokio::task::JoinSet<PaneId> + per-pane mpsc senders + per-pane CoalesceBuffer"
  - "vector-app::UserEvent migrated to PaneId-keyed shape: PaneOutput { pane_id, bytes } / PaneResized { pane_id, rows, cols } / PaneExited(PaneId) / PaneTitleChanged { pane_id, label }"
  - "frame_tick_loop generalized to per-pane (takes PaneId, emits PaneOutput)"
  - "Workspace dep: libc 0.2"
affects: [04-04 (Plan 04-04 replaces app.rs single-pane shim with PaneId routing across the Mux; reuses PtyActorRouter for Cmd-T / Cmd-D handler paths), 04-05 (smoke matrix exercises the per-pane PTY + proc_tracker + cwd-inheritance end-to-end)]

# Tech tracking
tech-stack:
  added:
    - "libc 0.2 (workspace) — for libc::tcgetpgrp (proc_tracker) + libc::proc_pidinfo (cwd::pidcwd macOS shim)"
  patterns:
    - "Per-pane PTY actor topology: one tokio::task::JoinSet<PaneId>::spawn per pane; the task body returns its PaneId so JoinSet::join_next surfaces PaneExited to the App layer naturally (Pitfall C avoidance — no centralized round-robin pump)"
    - "Per-pane biased select! ordering: resize > write > read so SIGWINCH never starves; carries forward from Plan 02-05 / Plan 03-04's single-pane shape"
    - "Per-pane CoalesceBuffer + frame_tick_loop spawn alongside each pane's I/O task; backpressure isolated per pane"
    - "Generic callback in proc_tracker (FnMut(PaneId, String)) keeps vector-mux winit-free; vector-app glue bridges to EventLoopProxy::send_event(UserEvent::PaneTitleChanged)"
    - "Cross-platform pidcwd shim via cfg(target_os): macOS uses libc::proc_pidinfo with PROC_PIDVNODEPATHINFO + proc_vnodepathinfo struct; Linux delegates to libproc::proc_pid::pidcwd"
    - "inherit_cwd test seam: inherit_cwd_with(parent_pid, home_env: Option<&str>) lets unit tests drive the libproc-err -> $HOME -> / fallback chain deterministically without mutating std::env"
    - "Async Mux helpers release-then-acquire locks: .await on LocalDomain::spawn_local completes BEFORE Mux.windows.write() / panes.write() is taken (Pitfall B compliance; clippy::await_holding_lock=deny holds)"

key-files:
  created:
    - crates/vector-mux/src/cwd.rs
    - crates/vector-mux/src/proc_tracker.rs
  modified:
    - Cargo.toml (workspace libc 0.2 dep)
    - Cargo.lock
    - crates/vector-mux/Cargo.toml (libc dep + libproc dev-dep)
    - crates/vector-mux/src/lib.rs (pub mod cwd + proc_tracker + re-exports)
    - crates/vector-mux/src/mux.rs (create_tab_async / split_pane_async / resize_window)
    - crates/vector-mux/src/pane.rs (shell_pid + master_fd accessors)
    - crates/vector-app/Cargo.toml (async-trait dev-dep)
    - crates/vector-app/src/main.rs (UserEvent migration + Mux::install bootstrap)
    - crates/vector-app/src/pty_actor.rs (REWRITE — PtyActorRouter + pane_io_loop)
    - crates/vector-app/src/frame_tick.rs (per-pane signature: PaneId + PaneOutput emit)
    - crates/vector-app/src/app.rs (UserEvent arm renames + Plan-04-04-deferred logging)
    - crates/vector-mux/tests/cwd_fallback.rs (un-ignored, 4 unit tests)
    - crates/vector-mux/tests/cwd_inheritance.rs (un-ignored, real-PTY integration)
    - crates/vector-mux/tests/proc_name_tracking.rs (un-ignored, real-PTY integration)
    - crates/vector-mux/tests/pane_resize_propagates.rs (un-ignored, real-PTY integration)

key-decisions:
  - "libproc 0.14's pidcwd() is documented as 'not implemented for macos' — discovered at first cwd_inheritance test run. Auto-fixed (Rule 1) by adding a vector_mux::cwd::pidcwd shim that calls Darwin libc::proc_pidinfo with PROC_PIDVNODEPATHINFO directly and parses proc_vnodepathinfo.pvi_cdir.vip_path. Plan's <interfaces> sketch said `libproc::proc_pid::pidcwd(pid)` — we keep the upstream call on Linux and route macOS through our own shim. One additional unit test (pidcwd_of_self_matches_current_dir) exercises the shim independent of real PTY spawning."
  - "Plan's <interfaces> sketch said `transport.resize(...).await` in the pane_io_loop. PtyTransport::resize is actually sync (returns Result<(), _>) — write is the only async method. The implemented loop matches the trait: `transport.resize(rows, cols, 0, 0)` returns Result synchronously and is logged on Err."
  - "proc_tracker chose generic FnMut(PaneId, String) emit callback over a winit-typed EventLoopProxy<UserEvent>. Rationale: vector-mux must not depend on winit (it's a model crate; the trait surface is D-38). vector-app glue closure bridges into EventLoopProxy::send_event(PaneTitleChanged) at startup. Trade-off: callers must wrap the callback for thread safety (`Send + 'static`), which they were already doing for the proxy."
  - "Per-pane frame_tick_loop (one task per pane) over a single multiplexed loop that iterates a HashMap<PaneId, Arc<CoalesceBuffer>> each tick. Per-pane keeps backpressure isolated and parallels the per-pane PTY actor model; the cost (one extra tokio task per pane) is negligible vs the wakeup chatter a multiplexed loop would generate when most panes are idle."
  - "Plan-04-03 App.rs deliberately treats PaneId as a discarded `let _ = pane_id;` — single-pane semantics, Plan 04-04 replaces the shim with PaneId routing. PaneExited and PaneTitleChanged are logged via `tracing::info!` for now; Plan 04-04 attaches them to window title + sentinel-line rendering."
  - "Mux::create_tab_async / split_pane_async take `cwd: Option<PathBuf>` and resolve None via inherit_cwd(parent_pid). create_tab_async passes parent_pid=None (the bootstrap tab has no parent) which routes to $HOME via the D-64 fallback chain. split_pane_async pulls the parent pane's shell_pid() and forwards it."
  - "Mux::resize_window walks tabs, calls split_tree::redistribute, then compute_layout, and returns Vec<(PaneId, rows, cols)>. The App is responsible for relaying through PtyActorRouter::send_resize (Plan 04-04 wires the call site)."
  - "PtyActorRouter wraps the tokio JoinSet + per-pane sender HashMaps in a single struct. send_write / send_resize do try_send (non-blocking) so keystrokes never stall main; on full/closed channels we trace::warn and drop. join_next_exited / shutdown_pane exist for Plan 04-04's pane-exit handler + Cmd-W path."
  - "main.rs single-pane glue: the App's (write_tx, resize_tx) channels feed two relay tasks that forward into the bootstrap pane's router channels. This is the Plan-04-03 shim — Plan 04-04 replaces with PaneId routing keyed on the active pane."

patterns-established:
  - "Phase 4 PTY actor topology — JoinSet<PaneId> + per-pane biased select! over (resize_rx, write_rx, reader) with the actor returning PaneId on transport.wait completion. Plan 04-04 will reuse PtyActorRouter for Cmd-D / Cmd-T spawn paths; Plan 04-05 smoke-tests it end-to-end. Phase 7 CodespaceDomain plugs into the SAME shape — the only difference is `spawn_codespace` instead of `spawn_local` upstream of `router.spawn_pane`."
  - "cfg(target_os) shim for libproc upstream gaps — when an upstream crate is missing macOS support, replicate the kernel API call in our own crate behind the same fn signature. Future similar gaps (e.g., process listing) can follow the same pattern without forking libproc."

requirements-completed: []
# WIN-02 / WIN-03 enabled at the I/O layer here (per-pane PTY actor + resize propagation + cwd inheritance), but ROADMAP marks them complete only after Plan 04-04 wires the keyboard + Cmd-D / Cmd-T handlers.

# Metrics
duration: ~20min
completed: 2026-05-12
---

# Phase 4 Plan 03: Per-pane PTY Actor + Mux Async Helpers + D-57/D-63 Tracking Summary

**Wire the Plan 04-02 Mux topology to live PTY I/O. One tokio task per pane via `JoinSet<PaneId>` (biased `select!` over resize / write / read), per-pane `CoalesceBuffer` drained by a per-pane `frame_tick_loop`, async Mux helpers (`create_tab_async`, `split_pane_async`, `resize_window`) that drive `LocalDomain::spawn_local`, foreground-process polling (D-57) at 1Hz that emits `PaneTitleChanged` only on transitions, cwd inheritance (D-63/D-64) through a `libc::proc_pidinfo` shim that compensates for libproc 0.14's missing macOS `pidcwd`. The 3 real-PTY integration tests + 1 unit test all pass; the App still launches with one working pane and the proc_tracker emits `PaneTitleChanged { pane_id: PaneId(1), label: "zsh" }` live within 1s of startup. Workspace test count rises 201 → 212 (+11: 4 cwd_fallback + 5 cwd unit + 2 pty_actor unit; the 3 integration tests stay `#[ignore = "real-PTY"]` and add to the include-ignored count). D-38 invariant held: zero diff in `domain.rs` / `transport.rs`.**

## Performance

- **Duration:** ~20 min (1200 s wall clock)
- **Started:** 2026-05-12T03:22:00Z
- **Completed:** 2026-05-12T03:40:00Z
- **Tasks:** 2 (each committed atomically)
- **Test count:** 212 passed / 0 failed / 19 ignored (baseline 201/0/20 at Plan 04-02 close)
  - +1 ignored: 3 new integration tests un-ignored to `real-PTY` ignore string from Wave-0 stub
  - −2 stubs from Plan 04-03 ownership (cwd_fallback +1 panic-stub removed)

## Accomplishments

### vector-mux

- **`cwd.rs`** — `inherit_cwd(parent_pid) -> PathBuf` + `inherit_cwd_with(parent_pid, home_env)` test seam. macOS `pidcwd` shim calls `libc::proc_pidinfo(pid, PROC_PIDVNODEPATHINFO, ...)` and parses `proc_vnodepathinfo.pvi_cdir.vip_path` as a NUL-terminated C string. On Err, the fallback chain emits a `tracing::warn!` with the err and pid, then tries `$HOME`, then `/`. 5 unit tests cover the chain (returns home on pid None, returns / on home unset, returns / on home empty, pid 0 falls back to home, pidcwd self matches current_dir).
- **`proc_tracker.rs`** — `proc_name_poll_loop<F: FnMut(PaneId, String) + Send + 'static>` runs forever at 1Hz (`MissedTickBehavior::Skip` — RENDER-03), walks `Mux::get().panes_snapshot()`, calls `unsafe { libc::tcgetpgrp(master_fd) }` for each pane, resolves via `libproc::proc_pid::pidpath`, takes the file_name basename, and invokes the callback only on transitions (`last_seen: HashMap<PaneId, String>` diff). `spawn_proc_tracker(emit)` wraps in `tokio::spawn` and returns the JoinHandle.
- **`Mux::create_tab_async(window_id, cwd, rows, cols) -> Result<(TabId, PaneId)>`** calls `default_domain.spawn_local(...)`, constructs a fresh `Arc<Mutex<Term>>` and `Pane` from the returned `SpawnedPane`, then `install_tab`. The `.await` precedes any RwLock write — Pitfall B compliance.
- **`Mux::split_pane_async(pane_id, dir, cwd) -> Result<PaneId>`** looks up the parent pane's shell_pid + the tab's last (rows, cols), resolves cwd via `inherit_cwd(parent_pid)` if caller passed None, spawns, then `split_pane`.
- **`Mux::resize_window(window_id, rows, cols) -> Vec<(PaneId, rows, cols)>`** walks each tab, updates `Tab.last_rows/cols`, calls `split_tree::redistribute(&mut tab.root, viewport)`, then iterates `compute_layout` to produce per-pane (rows, cols). The App layer relays each tuple through `PtyActorRouter::send_resize` — kernel SIGWINCH reaches child shells via the existing `PtyTransport::resize` (CORE-04 reuse from Phase 2).
- **`Pane::shell_pid()` + `Pane::master_fd()`** — read accessors for the `pid` + `master_fd` fields (cheap, no lock).

### vector-app

- **`pty_actor.rs` (REWRITE)** — `PtyActorRouter { proxy, lpm_flag, pane_writers: HashMap<PaneId, mpsc::Sender<Vec<u8>>>, pane_resizers: HashMap<PaneId, mpsc::Sender<(u16,u16)>>, coalesce_buffers: HashMap<PaneId, Arc<CoalesceBuffer>>, join_set: JoinSet<PaneId> }`. `spawn_pane(pane_id, transport)` wires three channels, spawns the per-pane `frame_tick_loop`, spawns the per-pane `pane_io_loop` into the JoinSet. `send_write` / `send_resize` do `try_send` (non-blocking); `join_next_exited` awaits the next pane's exit and returns its PaneId; `shutdown_pane` drops a pane's channels (so the actor's select! observes channel close and the loop breaks). 2 unit tests cover the JoinSet + take_reader-twice semantics.
- **`pane_io_loop`** — private per-pane task body. Biased `tokio::select!` over `resize_rx > write_rx > reader.recv()` matches Plan 02-05's single-pane shape. On transport.wait() completion, emits `UserEvent::PaneExited(pane_id)` and returns `pane_id` from the task body so `JoinSet::join_next` surfaces it.
- **`frame_tick.rs`** — `frame_tick_loop(pane_id, coalesce, proxy, lpm)` signature change. Emit becomes `UserEvent::PaneOutput { pane_id, bytes }` instead of `UserEvent::PtyOutput(bytes)`.
- **`main.rs` UserEvent migration** — `PtyOutput(Vec<u8>)` → `PaneOutput { pane_id, bytes }`; `Resized { rows, cols }` → `PaneResized { pane_id, rows, cols }`; added `PaneExited(PaneId)` + `PaneTitleChanged { pane_id, label }`. `LpmChanged(bool)` unchanged. Bootstrap creates `LocalDomain::new()` + `Mux::new() + Mux::install`, then `create_tab_async(window_id, None, 24, 80)`, then `PtyActorRouter::spawn_pane`. `spawn_proc_tracker` spawned with a closure that bridges (PaneId, String) → `EventLoopProxy::send_event(PaneTitleChanged)`.
- **`app.rs` user_event arms** — Phase-3 single-Term + single-Compositor pipeline preserved as a shim (`let _ = pane_id;`). `PaneExited` + `PaneTitleChanged` logged via `tracing::info!`; Plan 04-04 will wire them to sentinel-line rendering + tab title updates.

### Tests

- **`cwd_fallback.rs` (un-ignored, unit)** — 4 tests: home-when-pid-none, slash-when-pid-none-and-home-unset, slash-when-home-empty, pid-zero-falls-back-to-home.
- **`cwd_inheritance.rs` (real-PTY integration)** — spawn shell with cwd=/tmp, sleep 300ms, call `vector_mux::cwd::pidcwd(p1_pid)` → assert `/tmp` or `/private/tmp` (macOS symlink). Split, sleep 300ms, call pidcwd on p2 → assert same. Wall-clock ~0.6s.
- **`proc_name_tracking.rs` (real-PTY integration)** — spawn shell, drain banner, assert initial fg name in `[sh, zsh, bash, dash]`. Write `exec sleep 30\n`. Poll `tcgetpgrp + libproc::pidpath` every 200ms for up to 3s. Assert `"sleep"` observed. Wall-clock ~0.6s.
- **`pane_resize_propagates.rs` (real-PTY integration)** — Phase 1: bare `LocalDomain::spawn_local` round-trip — write `tput cols\n`, parse, assert ~80; resize 80→160 via `transport.resize`, assert tput sees ~160. Phase 2: Mux split — `create_tab_async` + `split_pane_async`, `resize_window(80)` redistributes 80 → 40/39, `tput cols` in each pane reports its share; sum is 79 (80 minus divider). Wall-clock ~3.3s.

## libproc::pidcwd macOS Gap (Deviation Rule 1 — Bug)

**Found during:** Task 2, first run of `cwd_inheritance` test.

**Issue:** `libproc 0.14.11`'s `proc_pid::pidcwd(pid)` is documented as `Err("pidcwd is not implemented for macos".into())` — the function exists but always errors on macOS. The Plan's `<interfaces>` block and the upstream pidpath docs implied it worked. Plan 04-01's research and SUMMARY hand-off both assumed `libproc::pidcwd` would deliver the cwd.

**Fix:** Implemented `vector_mux::cwd::pidcwd(pid) -> Result<PathBuf, String>` directly:
- `cfg(target_os = "macos")`: call `libc::proc_pidinfo(pid, libc::PROC_PIDVNODEPATHINFO, 0, &mut info, size)`, parse `proc_vnodepathinfo.pvi_cdir.vip_path` (a NUL-terminated `[c_char; MAXPATHLEN]` represented in libc as `[[c_char; 32]; 32]` for older rustc).
- `cfg(not(target_os = "macos"))`: delegate to `libproc::proc_pid::pidcwd` (works on Linux via `/proc/<pid>/cwd` readlink).

`inherit_cwd_with` now calls `pidcwd(pid)` instead of `libproc::proc_pid::pidcwd(pid)`. The fallback chain on Err is unchanged. Added a sanity unit test `pidcwd_of_self_matches_current_dir` that exercises the shim without a real PTY.

**Files modified:** `crates/vector-mux/src/cwd.rs`, `crates/vector-mux/tests/cwd_inheritance.rs`.

**Committed in:** `a47670e`.

**Impact:** Substantive deviation. Plan 04-04 (which uses inherit_cwd via Mux::split_pane_async) and Plan 04-05 (smoke matrix #4: Cmd-D in `~/personal/vector` -> new pane prompts in `~/personal/vector`) are unaffected at the API boundary — they call `vector_mux::cwd::inherit_cwd` which routes through the shim transparently. Plan 04-01's docs that said "libproc::pidcwd happy path" should be re-read as "vector_mux::cwd::pidcwd happy path" going forward.

## Other Deviations

### Auto-fixed (Rule 1)

**1. [Rule 1 - Format] rustfmt rewraps `tracing::warn!` macro args + closure body**
- Found during Task 1 fmt check. rustfmt prefers multi-line `tracing::warn!` for >100ch and prefers `.and_then(|x| call(x))` over a wrapped block.
- Fixed via `cargo fmt --all`.

**2. [Rule 1 - Clippy] `cast_possible_wrap` on `mem::size_of::<>() as c_int` + `process::id() as i32`**
- Found during Task 2 clippy run.
- Fixed: `i32::try_from(...).expect("fits")` + `libc::c_int::try_from(mem::size_of::<>()).expect("fits")`.

**3. [Rule 1 - Clippy] `match_same_arms` on `Ok(None) => break; Err(_) => break`**
- Found during Task 2 clippy run.
- Fixed: `Ok(None) | Err(_) => break`.

**Total deviations:** 4 auto-fixed (1 Rule 1 bug — libproc upstream gap, 1 Rule 1 format, 2 Rule 1 clippy compliance).

## Authentication Gates

None — Plan 04-03 is fully local (no GitHub / Codespaces / DevTunnels). The first such gate lands in Phase 6.

## Verification Results

```
cargo build --workspace --tests                                                ✓ clean
cargo clippy --workspace --all-targets -- -D warnings                          ✓ clean
cargo fmt --all -- --check                                                     ✓ clean
cargo test --workspace --tests -q                                              ✓ 212 passed / 0 failed / 19 ignored
cargo test -p vector-mux --test cwd_fallback                                   ✓ 4 passed
cargo test -p vector-mux --test cwd_inheritance       -- --include-ignored     ✓ 1 passed (real PTY, ~0.6s)
cargo test -p vector-mux --test proc_name_tracking    -- --include-ignored     ✓ 1 passed (real PTY, ~0.6s)
cargo test -p vector-mux --test pane_resize_propagates -- --include-ignored    ✓ 1 passed (real PTY, ~3.3s)
3x stability loop on all three                                                  ✓ non-flaky
cargo build -p vector-app --release && SIGTERM after 3s                        ✓ exit=143 (clean SIGTERM); proc_tracker emitted live PaneTitleChanged
git diff HEAD~2 -- crates/vector-mux/src/domain.rs crates/vector-mux/src/transport.rs   ✓ zero hunks (D-38 invariant)
ps aux | grep -E '(sleep 30|/bin/sh|/bin/zsh)' | grep $USER | grep -v grep    ✓ no zombies
grep -n 'pub enum UserEvent' crates/vector-app/src/main.rs                     ✓ matches new Pane-keyed shape
grep -nE 'PtyOutput\(|Resized \{ '  crates/vector-app/src/main.rs              ✓ 0 matches (old variants gone)
grep -n 'pub struct PtyActorRouter' crates/vector-app/src/pty_actor.rs         ✓ 1 match
grep -n 'JoinSet<PaneId>'           crates/vector-app/src/pty_actor.rs         ✓ 2 matches (field + test)
grep -n 'pub async fn create_tab_async\|pub async fn split_pane_async\|pub fn resize_window' crates/vector-mux/src/mux.rs   ✓ 3 matches
grep -n 'pub fn inherit_cwd'        crates/vector-mux/src/cwd.rs               ✓ 1 match
grep -n 'pub async fn proc_name_poll_loop' crates/vector-mux/src/proc_tracker.rs   ✓ 1 match
```

## Task Commits

1. **Task 1: PtyActorRouter + Mux async helpers + cwd + proc_tracker** — `a5b3a10` (feat)
2. **Task 2: 3 real-PTY integration tests + pidcwd macOS shim** — `a47670e` (test)

## Files Created/Modified

### Created (2)

- `crates/vector-mux/src/cwd.rs`
- `crates/vector-mux/src/proc_tracker.rs`

### Modified (13 + Cargo.lock)

- `Cargo.toml` (workspace libc 0.2)
- `crates/vector-mux/Cargo.toml` (libc dep + libproc dev-dep)
- `crates/vector-mux/src/lib.rs` (new modules + re-exports)
- `crates/vector-mux/src/mux.rs` (3 async helpers + redistribute call site)
- `crates/vector-mux/src/pane.rs` (shell_pid + master_fd accessors)
- `crates/vector-mux/tests/cwd_fallback.rs` (un-ignored, 4 tests)
- `crates/vector-mux/tests/cwd_inheritance.rs` (un-ignored, real-PTY)
- `crates/vector-mux/tests/proc_name_tracking.rs` (un-ignored, real-PTY)
- `crates/vector-mux/tests/pane_resize_propagates.rs` (un-ignored, real-PTY)
- `crates/vector-app/Cargo.toml` (async-trait dev-dep)
- `crates/vector-app/src/main.rs` (UserEvent + Mux bootstrap)
- `crates/vector-app/src/pty_actor.rs` (REWRITE — router)
- `crates/vector-app/src/frame_tick.rs` (per-pane signature)
- `crates/vector-app/src/app.rs` (event arm renames)

## Hand-off to Plan 04-04

- **Un-ignore the 16 Plan-04-04 stubs** (14 xterm_key_table Cmd-* keymap cases + 1 multi_window_tabbing + 1 active_pane_border).
- **`PtyActorRouter`** is the per-pane router; reuse for Cmd-T (new tab) + Cmd-D (split) handler paths: `mux.create_tab_async(...)` or `mux.split_pane_async(...)` returns `(TabId, PaneId)` / `PaneId`; then `router.spawn_pane(pane_id, pane.take_transport().unwrap())`. The router carries `lpm_flag` so the per-pane frame_tick respects D-46.
- **App.rs single-pane shim** — `let _ = pane_id;` arms must be replaced with PaneId-keyed routing across the Mux: look up the target `Pane`, lock its `Arc<Mutex<Term>>`, feed bytes there. `PaneExited` should mark the pane as exited (Plan 04-02 already provides the `exited: AtomicBool` field); on close cascade, route via `mux.close_pane(pane_id) -> CloseResult` and react in the App (drop winit Window on `WindowClosed`, exit loop on `LastWindowClosed`). `PaneTitleChanged` should propagate to the NSWindow title (the `D-56` NSWindowTabbingMode-managed window will reflect it in the system tab bar).
- **`vector_mux::cwd::inherit_cwd(parent_pid)`** is the canonical cwd resolver. When users hit Cmd-D in `~/personal/vector`, Plan 04-04's split handler must call `mux.split_pane_async(active_pane, dir, None)` — passing None invokes `inherit_cwd(parent.shell_pid())` internally, which our macOS shim resolves via `proc_pidinfo`.
- **D-38 invariant** — `crates/vector-mux/src/domain.rs` + `transport.rs` are byte-identical to Phase 2 Plan 02-04. Phase 7 / 8 will add their domains by impl'ing `Domain` + `PtyTransport`. Do NOT touch these files in Plan 04-04 either.
- **WIN-04 grep arch-lint** — still green; no new files in `vector-term/src/`.

## Hand-off to Plan 04-05

- **Smoke matrix item #4 (Cmd-D in `~/personal/vector` → new pane prompts in `~/personal/vector`)** has automated coverage now via `cwd_inheritance` integration test + the manual matrix item asserts the visual end-to-end behavior. The cwd shim works on macOS (verified live: `/tmp` and `/private/tmp` accepted).
- **Smoke matrix item for `tput cols` round-trip after split** has automated coverage via `pane_resize_propagates`. The manual matrix can spot-check WindowEvent::Resized → split-tree redistribute → per-pane transport.resize → child shell SIGWINCH.
- **D-57 fg-process tracking** has automated coverage via `proc_name_tracking`. The manual matrix can confirm that running `vim` in a pane updates the system tab bar title within 1s (Plan 04-04 wires the NSWindow title).

## Issues Encountered

1. **libproc 0.14 pidcwd unimplemented on macOS** — caught at first integration-test run; Rule 1 auto-fix added the shim. No blocker; ~30 minutes of detour.

No other issues.

## Self-Check: PASSED

All claimed files exist:

- crates/vector-mux/src/cwd.rs — FOUND
- crates/vector-mux/src/proc_tracker.rs — FOUND
- crates/vector-mux/Cargo.toml (modified) — FOUND
- crates/vector-mux/src/lib.rs (modified) — FOUND
- crates/vector-mux/src/mux.rs (modified) — FOUND
- crates/vector-mux/src/pane.rs (modified) — FOUND
- crates/vector-mux/tests/cwd_fallback.rs (modified) — FOUND
- crates/vector-mux/tests/cwd_inheritance.rs (modified) — FOUND
- crates/vector-mux/tests/proc_name_tracking.rs (modified) — FOUND
- crates/vector-mux/tests/pane_resize_propagates.rs (modified) — FOUND
- crates/vector-app/Cargo.toml (modified) — FOUND
- crates/vector-app/src/main.rs (modified) — FOUND
- crates/vector-app/src/pty_actor.rs (modified) — FOUND
- crates/vector-app/src/frame_tick.rs (modified) — FOUND
- crates/vector-app/src/app.rs (modified) — FOUND
- Cargo.toml (modified) — FOUND
- Cargo.lock (modified) — FOUND

All claimed commits exist:

- a5b3a10 — FOUND (Task 1)
- a47670e — FOUND (Task 2)

---
*Phase: 04-mux-tabs-splits*
*Plan: 03*
*Completed: 2026-05-12*
