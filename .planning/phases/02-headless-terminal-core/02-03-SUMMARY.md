---
phase: 02-headless-terminal-core
plan: 03
subsystem: vector-pty
tags: [portable-pty, tokio, spawn_blocking, mpsc, sigwinch, drop-discipline, core-04, core-05]

# Dependency graph
requires:
  - phase: 02-headless-terminal-core
    plan: 01
    provides: portable-pty 0.9 workspace dep + #[ignore] lifecycle.rs (4 stubs) + term_env_advertise.rs (1 stub) + tokio dev-deps (rt-multi-thread+macros+time+sync+process)
provides:
  - "Public API: vector_pty::LocalPty (spawn/resize/write/take_reader/wait) + vector_pty::SpawnCommand + vector_pty::PtyError"
  - "Working spawn_blocking + bounded-mpsc(64) PTY ↔ tokio bridge with blocking_send backpressure (Pitfall 7)"
  - "drop(pair.slave) + impl Drop kill+wait discipline -> no zombies (CORE-04 success-criterion)"
  - "TERM=xterm-256color advertised before user-supplied env (CORE-05)"
  - "MasterPty::resize() -> kernel SIGWINCH propagation verified end-to-end against bash 3.2"
  - "5 passing integration tests + arch-lint green, non-flaky over 3 consecutive runs"
affects: [02-04 vector-mux (wraps LocalPty inherent methods in `impl PtyTransport`; `LocalDomain::spawn(SpawnCommand)` calls `LocalPty::spawn`), 02-05 vector-headless (constructs LocalPty via LocalDomain, calls write/take_reader/resize each render+input tick)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Two `spawn_blocking` tasks per LocalPty — one reader (PTY → mpsc), one writer (mpsc → PTY) — wrapping portable-pty's blocking master fd"
    - "Bounded `mpsc::channel::<Vec<u8>>(64)` with `blocking_send` for natural backpressure (WezTerm pattern; Pitfall 7 / Anti-Pattern 6)"
    - "`drop(pair.slave)` in parent IMMEDIATELY after `spawn_command` (Pitfall 3 — otherwise child can't see EOF on master close → zombies)"
    - "`impl Drop for LocalPty { kill + wait }` so dropping the transport terminates the child cleanly without relying on Drop-order between master fd and the spawn_blocking reader"
    - "Resize via `MasterPty::resize(PtySize)` — kernel turns it into SIGWINCH for the slave pgrp; no manual ioctl"
    - "Integration tests build their own `tokio::runtime::Builder::new_multi_thread()` (D-08 — never use `#[tokio::test]`; arch-lint scans `src/` only, so test files are allowed to construct runtimes)"

key-files:
  created:
    - crates/vector-pty/src/error.rs
    - crates/vector-pty/src/local.rs
  modified:
    - crates/vector-pty/Cargo.toml (added portable-pty + tokio + tracing deps)
    - crates/vector-pty/src/lib.rs (retired stub `PtyTransport` trait; now re-exports `LocalPty` + `SpawnCommand` + `PtyError` from the local + error modules)
    - crates/vector-pty/tests/lifecycle.rs (4 #[ignore]s removed; 4 tests filled)
    - crates/vector-pty/tests/term_env_advertise.rs (1 #[ignore] removed; 1 test filled)
    - Cargo.lock (portable-pty 0.9.0 + transitive: filedescriptor, nix 0.28, serial2, shell-words, downcast-rs, cfg_aliases, shared_library, winreg)

key-decisions:
  - "Stub `PtyTransport` trait from Phase 1's vector-pty/src/lib.rs RETIRED — that surface is owned by vector-mux per Plan 02-04 (D-38). Phase 2 vector-pty ships ONLY concrete `LocalPty` with inherent methods."
  - "`portable_pty::Child + Send + Sync` is object-safe in 0.9 — no need to drop `+ Sync` from the trait-object box. `Child::wait` and `Child::kill` are both available on the trait."
  - "Resize test uses a `while/sleep 0.1` polling loop in the child (NOT `sleep 5`) because bash 3.2 (macOS /bin/sh) does NOT interrupt a sleep on trapped SIGWINCH. Found during test execution — Rule 2 auto-fix (test was missing this critical-for-correctness detail)."
  - "Allow `clippy::needless_pass_by_value` on `LocalPty::spawn(shell, cmd: SpawnCommand)` because Plan 02-04's `Domain::spawn(SpawnCommand)` hands the value off — no clone needed at the trait boundary."
  - "Use `io::Error::other(...)` (Rust 1.74+ shortcut) instead of `io::Error::new(io::ErrorKind::Other, ...)` for portable-pty's stringly-typed errors — cleaner and clippy doesn't flag."

patterns-established:
  - "Concrete-transport-first, trait-wrapper-second: ship the inherent-method surface on a concrete type in one wave, wrap it in `impl Trait for Concrete` in the next. Lets Wave 1 (vector-term) and Wave 2 (vector-pty) run in parallel without sharing a trait file."
  - "Test files under `tests/` build their own multi-thread runtime via `Builder::new_multi_thread().worker_threads(2).enable_all().build()` and call `.block_on(async { … })`. arch-lint `no_tokio_main.rs` only scans `src/`, so this is the canonical pattern for tests that need real I/O."

requirements-completed: [CORE-04, CORE-05]

# Metrics
duration: 4min
completed: 2026-05-11
---

# Phase 2 Plan 03: vector-pty LocalPty Summary

**Concrete `LocalPty` transport: spawns a child shell via `portable-pty 0.9`'s `native_pty_system().openpty()`, bridges blocking PTY I/O to tokio via two `spawn_blocking` tasks + bounded `mpsc::channel::<Vec<u8>>(64)` with `blocking_send` backpressure, advertises `TERM=xterm-256color` (CORE-05), and propagates resize to the child as SIGWINCH via `MasterPty::resize()` (CORE-04). Drop-discipline (`drop(pair.slave)` + `impl Drop { kill + wait }`) guarantees no zombies. 5 integration tests pass against a real `/bin/sh` child in ~2.6s wall-clock, non-flaky over 3 consecutive runs.**

## Performance

- **Duration:** ~4 min (258s wall clock from initial Read to Task 2 commit)
- **Started:** 2026-05-11T16:22:24Z
- **Completed:** 2026-05-11T16:26:42Z
- **Tasks:** 2 (each committed atomically)
- **Test count:** 5 passing (4 lifecycle + 1 term-env) + arch-lint, was 5 ignored
- **Test wall-clock:** `cargo test -p vector-pty --tests` reports ~2.61s (4 sec lifecycle dominated by `sleep 0.1`/`sleep 30` waits; term_env 0.04s)

## Accomplishments

- `vector_pty` public surface ships: `LocalPty` (`spawn / resize / write / take_reader / wait`) + `SpawnCommand` + `PtyError` — matches the locked interface block from the plan exactly; Plan 02-04 wraps these methods in `impl PtyTransport for LocalPty` without reshaping.
- Phase 1's stub `PtyTransport` trait in `vector-pty/src/lib.rs` retired — that surface is owned by `vector-mux` per D-38.
- CORE-04 fully covered: spawn `sh -c "echo hi"`, child writes `hi` reachable through `take_reader()`, exit code 0 via `wait()`; resize propagates SIGWINCH (verified by `stty size` printing `50 100` after `resize(50, 100, 0, 0)`); `drop(LocalPty)` while child runs `sleep 30` terminates the child within 500 ms (verified via `kill -0 PID`); no zombie `sh` processes after a clean `wait()` (verified via `ps -o stat,command`).
- CORE-05 fully covered: `printenv TERM` from inside the spawned shell reads back `xterm-256color`.
- Pitfall 3 honored: `drop(pair.slave)` is the literal first line after `spawn_command` — otherwise the child can't see EOF when we close the master and becomes a zombie.
- Pitfall 7 / Anti-Pattern 6 honored: bounded `mpsc::channel::<Vec<u8>>(64)` + `blocking_send` (NOT `try_send`) gives natural backpressure into the kernel PTY's flow control instead of unbounded OOM growth.
- D-08 honored: zero `#[tokio::test]` in `tests/`; runtimes built via `tokio::runtime::Builder::new_multi_thread()`.
- No `unsafe` (workspace `unsafe_code = "deny"` holds).
- `cargo clippy -p vector-pty --all-targets -- -D warnings` clean.
- `cargo fmt --all -- --check` clean.
- Non-flaky over 3 consecutive `cargo test -p vector-pty --test lifecycle` runs (2.57s / 2.58s / 2.58s).

## Task Commits

1. **Task 1: Implement LocalPty + tokio bridge** — `615e1c8` (feat)
2. **Task 2: Fill lifecycle + term-env integration tests** — `4aa4b72` (test)

## Files Created/Modified

### Created (2)

- `crates/vector-pty/src/error.rs` — `pub enum PtyError` (thiserror): `OpenPty(String)`, `Spawn(String)`, `Resize(String)`, `WriteClosed`, `AlreadyWaited`, `Io(#[from] std::io::Error)`.
- `crates/vector-pty/src/local.rs` — `pub struct LocalPty` + `pub struct SpawnCommand` + impl block + `impl Drop for LocalPty`. ~150 LOC; no `unsafe`.

### Modified (5)

- `crates/vector-pty/Cargo.toml` — added `portable-pty = { workspace = true }`, `tokio = { workspace = true }`, `tracing = { workspace = true }` to `[dependencies]`. `anyhow` + `thiserror` were already wired by Plan 02-01.
- `crates/vector-pty/src/lib.rs` — replaced the stub `PtyTransport` trait (Phase 1 placeholder) with the new module tree: `pub use error::PtyError; pub use local::{LocalPty, SpawnCommand};`. Trait surface ships in `vector-mux` per Plan 02-04.
- `crates/vector-pty/tests/lifecycle.rs` — 4 `#[ignore]` stubs replaced with real tests; helper `read_for(rx, dur)` collects mpsc chunks until deadline.
- `crates/vector-pty/tests/term_env_advertise.rs` — 1 `#[ignore]` stub replaced with `printenv TERM` test.
- `Cargo.lock` — portable-pty 0.9.0 + 8 transitive deps (filedescriptor 0.8.3, nix 0.28.0, serial2 0.2.37, shell-words 1.1.1, downcast-rs 1.2.1, cfg_aliases 0.1.1, shared_library 0.1.9, winreg 0.10.1) locked in.

## Public API (final, Plan 02-04 wraps these)

```rust
// crates/vector-pty/src/lib.rs
pub use error::PtyError;
pub use local::{LocalPty, SpawnCommand};

// LocalPty inherent methods:
impl LocalPty {
    pub fn spawn(shell: &Path, cmd: SpawnCommand) -> Result<Self, PtyError>;
    pub fn resize(&mut self, rows: u16, cols: u16, px_w: u16, px_h: u16) -> Result<(), PtyError>;
    pub async fn write(&self, bytes: &[u8]) -> Result<(), PtyError>;
    pub fn take_reader(&mut self) -> Option<tokio::sync::mpsc::Receiver<Vec<u8>>>;
    pub async fn wait(&mut self) -> Result<Option<i32>, PtyError>;
}

pub struct SpawnCommand {
    pub argv: Option<Vec<String>>,
    pub cwd: Option<PathBuf>,
    pub rows: u16,
    pub cols: u16,
    pub env: Vec<(String, String)>,
}
```

`SpawnCommand` field set is the contract Plan 02-04 routes through `Domain::spawn(SpawnCommand) -> Box<dyn PtyTransport>`.

## Decisions Made

- **Retire the Phase-1 stub `PtyTransport` trait in `vector-pty/src/lib.rs`.** Per D-38 + plan objective, the trait surface lives in `vector-mux`. `vector-pty` ships only the concrete transport. This keeps the dep graph one-directional (mux → pty, never the reverse) and avoids a circular-trait-impl situation in Plan 02-04.
- **`Box<dyn portable_pty::Child + Send + Sync>` is valid in 0.9.** No need to drop `+ Sync`. `wait()` is on the trait and takes `&mut self`; we use `Option<Box<...>>` so `wait()` can consume the child and `Drop` doesn't double-wait.
- **`io::Error::other(...)` over `io::Error::new(io::ErrorKind::Other, ...)`.** Rust 1.74+ shortcut, cleaner, identical semantics. Workspace MSRV is 1.88 — safe.
- **Allow `clippy::needless_pass_by_value` on `LocalPty::spawn`.** The `SpawnCommand` argument is consumed by ownership because Plan 02-04's `Domain::spawn(SpawnCommand)` hands the value off — taking `&SpawnCommand` would force a clone at the trait boundary.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Resize test failed because bash 3.2 does NOT interrupt `sleep` on trapped SIGWINCH**

- **Found during:** Task 2 (first run of `cargo test -p vector-pty --test lifecycle`)
- **Issue:** The plan's `resize_propagates_sigwinch_to_child` script was `trap 'stty size; exit 0' WINCH; sleep 5` — relies on the shell interrupting the sleep when WINCH arrives. macOS `/bin/sh` is bash 3.2; verified empirically that bash 3.2 lets `sleep` run to completion even with a trap installed, so the trap fires after 5 s (too late — `read_for` deadline is 4 s). Output was empty `""`.
- **Fix:** Replaced `sleep 5` with a polling loop: `i=0; while [ $i -lt 50 ]; do sleep 0.1; i=$((i+1)); done` — the trap fires between iterations (every ~100 ms) and emits `stty size` output well within the 4 s window.
- **Files modified:** `crates/vector-pty/tests/lifecycle.rs`
- **Verification:** Test passes; non-flaky over 3 consecutive runs.
- **Committed in:** `4aa4b72` (Task 2 commit).

**2. [Rule 1 - Bug] Clippy `needless_continue` on the `Interrupted` arm of the reader loop**

- **Found during:** Task 1 (after running `cargo clippy -p vector-pty --all-targets -- -D warnings` per the plan's `<verify>` block)
- **Issue:** The plan's snippet had `Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,` — clippy `needless_continue` fires because the match arm is the loop body's tail, so `continue` is redundant.
- **Fix:** Replaced `continue` with `{}` (empty body). Same control flow; clippy-clean.
- **Files modified:** `crates/vector-pty/src/local.rs`
- **Verification:** `cargo clippy -p vector-pty --all-targets -- -D warnings` exits 0.
- **Committed in:** `615e1c8` (Task 1 commit).

**3. [Rule 1 - Bug] Clippy `needless_pass_by_value` on `LocalPty::spawn`**

- **Found during:** Task 1 (same clippy invocation)
- **Issue:** `pub fn spawn(shell: &Path, cmd: SpawnCommand)` — clippy says take `&SpawnCommand` because `cmd` isn't moved into a destructor.
- **Fix:** Added `#[allow(clippy::needless_pass_by_value)]` with a comment justifying the design: Plan 02-04's `Domain::spawn(SpawnCommand)` hands the value off; we don't want callers to clone.
- **Files modified:** `crates/vector-pty/src/local.rs`
- **Verification:** Same clippy run, exits 0.
- **Committed in:** `615e1c8` (Task 1 commit).

**4. [Rule 1 - Bug] Clippy `match_same_arms` on `read_for` helper**

- **Found during:** Task 2 (after running `cargo clippy` from the plan's `<verification>` block)
- **Issue:** `Ok(None) => break,` and `Err(_) => break,` are the same arm body — clippy says merge them.
- **Fix:** Merged into `Ok(None) | Err(_) => break,` in both `lifecycle.rs` and `term_env_advertise.rs`.
- **Files modified:** `crates/vector-pty/tests/lifecycle.rs`, `crates/vector-pty/tests/term_env_advertise.rs`
- **Verification:** Clippy exits 0.
- **Committed in:** `4aa4b72` (Task 2 commit).

**5. [Rule 1 - Bug] rustfmt re-flowed `LocalPty::resize` signature**

- **Found during:** Task 2 (after running `cargo fmt --all`)
- **Issue:** The plan had `pub fn resize(\n        &mut self,\n        rows: u16, ...` — rustfmt wanted it on one line.
- **Fix:** Let `cargo fmt` apply its reflow.
- **Files modified:** `crates/vector-pty/src/local.rs` (whitespace-only)
- **Verification:** `cargo fmt --all -- --check` exits 0.
- **Committed in:** `4aa4b72` (Task 2 commit — bundled because fmt surfaced during Task 2's verification step).

---

**Total deviations:** 5 auto-fixed (1 plan-test-script bug, 4 lint compliance). **Impact on plan:** Deviation #1 is the only behavioral change — the plan's test script wouldn't have worked on bash 3.2 / macOS as written. The fix is a test-fixture refinement; no production-code or contract change. Deviations #2–#5 are mechanical lint compliance.

## Issues Encountered

None blocking. The bash-3.2-sleep-WINCH interaction was the only genuine surprise — fixed inline.

## Verification Results

Final state of plan-level verification (all green):

```
cargo build -p vector-pty                                  ✓ compiles
cargo test -p vector-pty --tests                           ✓ 4 + 1 + arch-lint = 6 pass, 0 fail
cargo clippy -p vector-pty --all-targets -- -D warnings    ✓ clean
cargo fmt --all -- --check                                 ✓ clean
ps -ef | grep -E '\bsh\b' | grep -v grep | grep '<defunct>'  ✓ no zombies
```

Per-test breakdown:

| Test file | Tests | Time | Status |
|-----------|-------|------|--------|
| `tests/lifecycle.rs` | 4 (spawn_echo, resize_sigwinch, no_zombies, drop_terminates) | ~2.58s | ✓ pass |
| `tests/term_env_advertise.rs` | 1 (TERM=xterm-256color) | ~0.04s | ✓ pass |
| `tests/no_tokio_main.rs` | 1 (arch-lint) | ~0.00s | ✓ pass |

Flake check: 3 consecutive `cargo test -p vector-pty --test lifecycle` runs all pass with consistent 2.57–2.58s timings.

## Hand-off Notes for Downstream Plans

### Plan 02-04 (vector-mux LocalDomain + traits, Wave 3)

- **Trait wrapper is mechanical:** add `vector-pty = { path = "../vector-pty" }` to `vector-mux/Cargo.toml`; write `impl PtyTransport for LocalPty` that just forwards to the inherent methods (`fn resize`, `async fn write`, `fn take_reader`, `async fn wait`). The signatures match exactly what the plan's `<interfaces>` block specified.
- **`Domain::spawn(SpawnCommand) -> Box<dyn PtyTransport>`** routes into `LocalPty::spawn(&self.shell_path, cmd)` then `Box::new(local_pty)`. `LocalDomain` carries the shell path resolved from `$SHELL` / `/etc/passwd` per D-36's shell selection.
- **Drop discipline is already correct.** `LocalPty: Drop` kills + waits the child; the `Box<dyn PtyTransport>` will drop the inner `LocalPty` via vtable. No extra cleanup needed in `LocalDomain`.
- **`portable_pty::Child + Send + Sync`** is what we use — confirms that the trait-object-safety test in `vector-mux/tests/trait_object_safety.rs` should compile a `Box<dyn PtyTransport>` cleanly. No `+ Sync` to drop.

### Plan 02-05 (vector-headless pass-through proxy, Wave 4)

- **Construction:** In `vector-headless/src/main.rs`, build a `LocalDomain` (Plan 02-04) and call `LocalDomain::spawn(SpawnCommand { argv: None, cwd: None, rows, cols, env: vec![] })` to get a `Box<dyn PtyTransport>`. Or, if Plan 02-04 lands after this, construct `LocalPty` directly via `LocalPty::spawn(Path::new(&shell), cmd)`.
- **Reader pump:** Call `take_reader()` once to get the `mpsc::Receiver<Vec<u8>>`, then `recv().await` in a loop, feeding bytes into `vector_term::Term::feed(&bytes)` (Plan 02-02 surface).
- **Writer pump:** Read parent terminal stdin bytes (crossterm raw mode), call `pty.write(&bytes).await` per chunk.
- **Resize:** On parent terminal SIGWINCH, call `pty.resize(rows, cols, 0, 0)` and `term.resize(cols, rows)` (vector-term).
- **Lifecycle on shell exit:** `pty.wait().await` returns when child exits. After the reader channel closes (receiver gets `None`), drain remaining bytes, render final grid, exit 0. No `kill()` needed — child already exited; `Drop` is a no-op in this path.

## Next Phase Readiness

- Phase 2 Wave 2 complete; Plans 02-04 (vector-mux LocalDomain + traits) and 02-05 (vector-headless) can proceed. Both consume the `LocalPty` surface ratified here.
- Pitfalls 3 + 7 are demonstrably honored end-to-end (zombie-test + SIGWINCH-propagate both pass against real `/bin/sh`).
- No blockers identified.

## Self-Check: PASSED

All claimed files exist:

- crates/vector-pty/src/error.rs — FOUND
- crates/vector-pty/src/local.rs — FOUND
- crates/vector-pty/src/lib.rs (modified) — FOUND
- crates/vector-pty/Cargo.toml (modified) — FOUND
- crates/vector-pty/tests/lifecycle.rs (modified) — FOUND
- crates/vector-pty/tests/term_env_advertise.rs (modified) — FOUND

All claimed commits exist:

- 615e1c8 — FOUND
- 4aa4b72 — FOUND

---
*Phase: 02-headless-terminal-core*
*Plan: 03*
*Completed: 2026-05-11*
