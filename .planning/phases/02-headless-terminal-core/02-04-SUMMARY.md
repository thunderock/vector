---
phase: 02-headless-terminal-core
plan: 04
subsystem: vector-mux
tags: [async-trait, domain-trait, pty-transport, local-domain, object-safety, d-38, core-04, core-05]

# Dependency graph
requires:
  - phase: 02-headless-terminal-core
    plan: 01
    provides: workspace deps (async-trait 0.1) + #[ignore] trait_object_safety.rs (2 stubs)
  - phase: 02-headless-terminal-core
    plan: 03
    provides: LocalPty (spawn/resize/write/take_reader/wait) + SpawnCommand + PtyError
provides:
  - "Public API: `vector_mux::PtyTransport` (async trait, Send + 'static) + `vector_mux::TransportKind`"
  - "Public API: `vector_mux::Domain` (async trait, Send + Sync) + `vector_mux::SpawnCommand`"
  - "`LocalDomain::new()` / `LocalDomain::with_shell(PathBuf)` — full impl"
  - "`LocalTransport(LocalPty)` newtype with `impl PtyTransport` — wires LocalPty into the trait surface without a vector-pty -> vector-mux dep edge"
  - "`CodespaceDomain` / `DevTunnelDomain` stubs that compile against the locked trait shape (`unimplemented!(\"Phase 7\")` / `unimplemented!(\"Phase 8\")`)"
  - "8 passing object-safety + behavior tests in trait_object_safety.rs (includes end-to-end CORE-04/05 reachability proof through `Box<dyn PtyTransport>`)"
affects: [02-05 vector-headless (constructs LocalDomain then spawn -> Box<dyn PtyTransport>), 04 mux (consumes Domain trait objects for routing), 07 Codespaces (fills CodespaceDomain::spawn body), 08 DevTunnels (fills DevTunnelDomain::spawn body), 09 reconnect (fills CodespaceDomain/DevTunnelDomain::reconnect bodies)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Newtype wrapper (`LocalTransport(LocalPty)`) carries `impl PtyTransport` so the concrete transport crate (vector-pty) stays free of trait-from-consumer dep cycle"
    - "`#[async_trait]` on trait + `Box<dyn ... Send + 'static>` for trait objects — async-trait 0.1 boxes the futures so `Box<dyn Trait>` is straightforwardly object-safe"
    - "Shell resolution chain: `$SHELL` -> `/etc/passwd` lookup by current uid (via `id -un` + parse) -> `/bin/zsh` -> `/bin/bash` -> bail"
    - "End-to-end trait-surface reachability test: `LocalDomain::spawn` -> `Box<dyn PtyTransport>` -> `.take_reader()` -> bytes + `.wait()` -> exit 0 — proves the trait surface (not just direct LocalPty access) carries CORE-04/05"

key-files:
  created:
    - crates/vector-mux/src/transport.rs
    - crates/vector-mux/src/domain.rs
    - crates/vector-mux/src/local_domain.rs
    - crates/vector-mux/src/codespace_domain.rs
    - crates/vector-mux/src/devtunnel_domain.rs
  modified:
    - crates/vector-mux/Cargo.toml (added async-trait, tokio, vector-pty deps + tests dev-deps)
    - crates/vector-mux/src/lib.rs (replaced stub `Domain`/`Pane` traits with module tree + re-exports)
    - crates/vector-mux/tests/trait_object_safety.rs (un-ignored 2 stubs, replaced with 8 real tests)
    - crates/vector-pty/src/local.rs (LocalPty::write: &self -> &mut self — Rule 3 blocking-issue fix for trait-object Send-future)
    - Cargo.lock

key-decisions:
  - "Newtype `LocalTransport(LocalPty)` in vector-mux carries `impl PtyTransport for LocalTransport` — NOT `impl PtyTransport for LocalPty` in vector-pty. This is the only way to satisfy D-38 (trait surface in vector-mux) AND avoid a vector-pty -> vector-mux dep cycle. vector-pty stays consumer-agnostic."
  - "`LocalPty::write` signature changed from `&self` to `&mut self`. Required because `Box<dyn PtyTransport>::write(&mut self)`'s returned future must be `Send`, which means `&mut LocalPty` must be Send (LocalPty itself is `!Sync` because `Box<dyn MasterPty + Send>` is not Sync — portable_pty 0.9's MasterPty trait is `Send` but not `Sync`). Taking `&mut self` requires only `LocalPty: Send`, which it is. Auto-fixed (Rule 3 blocking issue)."
  - "`CodespaceDomain::reconnect` and `DevTunnelDomain::reconnect` both `unimplemented!(\"Phase 9: Persistence + reconnect\")` — body lands in the phase that owns reconnect logic, not the phase that owns the transport (Phases 7/8)."
  - "Shell resolution: `$SHELL` env -> `/etc/passwd` parse keyed by `id -un` -> `/bin/zsh` -> `/bin/bash` -> error. Plain `std::fs::read_to_string(\"/etc/passwd\")` is sufficient on macOS dev hosts; `dscl` would be more canonical but requires shelling out and isn't needed."
  - "8 tests in trait_object_safety.rs cover: 2 compile-time object-safety checks (`Box<dyn PtyTransport>`, `Box<dyn Domain>`), 3 label/alive checks (LocalDomain, CodespaceDomain, DevTunnelDomain), 2 should-panic checks confirming phase markers fire (Phase 7, Phase 8), 1 end-to-end CORE-04/05 reachability proof through the trait surface."

patterns-established:
  - "Trait surface in consumer crate; concrete impls in producer crate; newtype wrapper in consumer crate carries the `impl Trait for Producer`. Lets the producer stay dep-free of the trait crate."
  - "Object-safety asserted by `let f: fn(Box<dyn Trait>) = ...; fn_addr_eq(...)` rather than `let _ = Box::new(...)` to avoid clippy `no_effect_underscore_binding` warnings under workspace pedantic."
  - "Phase-N markers in `unimplemented!()` messages give grep-able evidence in stub source files that downstream phases haven't lost their hook points."

requirements-completed: [CORE-04, CORE-05]

# Metrics
duration: 4min
completed: 2026-05-11
---

# Phase 2 Plan 04: vector-mux Domain + PtyTransport Traits Summary

**Lock the load-bearing seam (D-38) between terminal model and transport: `PtyTransport` + `Domain` traits ship in their FINAL shape (`async_trait`, `Send + 'static` / `Send + Sync`), `LocalDomain` is fully implemented via newtype `LocalTransport(LocalPty)`, and `CodespaceDomain`/`DevTunnelDomain` ship as compile-time stubs with phase markers. 8 tests pass including an end-to-end `Box<dyn PtyTransport>` round-trip (echo "hi" -> bytes via take_reader, exit 0 via wait) that proves CORE-04 + CORE-05 reach through the trait surface and not just direct LocalPty access. No vector-pty -> vector-mux dep edge introduced; Plan 02-03's 5 integration tests still pass.**

## Performance

- **Duration:** ~4 min (219s wall clock)
- **Started:** 2026-05-11T16:30:06Z
- **Completed:** 2026-05-11T16:33:45Z
- **Tasks:** 2 (each committed atomically)
- **Test count:** 8 passing (was 2 ignored stubs from Plan 02-01) — 0 ignored, 0 failed
- **Test wall-clock:** trait_object_safety suite finishes in ~0.04s including the end-to-end shell-spawn check

## Accomplishments

- `PtyTransport` trait (`Send + 'static`, async-trait): `resize` / `write` / `take_reader` / `kind` / `wait` — the FINAL D-38 shape that Phases 4/7/8/9 plug into without reshaping.
- `Domain` trait (`Send + Sync`, async-trait): `spawn` / `label` / `is_alive` / `reconnect` — unified `SpawnCommand` carries argv/cwd/rows/cols/env across all transports.
- `LocalDomain::new()` resolves the user shell via the documented chain (`$SHELL` → `/etc/passwd` keyed by `id -un` → `/bin/zsh` → `/bin/bash`); `LocalDomain::with_shell(PathBuf)` injects a path explicitly for tests.
- `LocalDomain::spawn(SpawnCommand)` returns `Box<dyn PtyTransport>` wrapping `LocalPty` via the `LocalTransport` newtype — kernel-level CORE-04 (clean exit, no zombies, SIGWINCH on resize) and CORE-05 (`TERM=xterm-256color`) carry through unchanged.
- `CodespaceDomain` and `DevTunnelDomain` ship with phase markers (`unimplemented!("Phase 7…")`, `unimplemented!("Phase 8…")`, `unimplemented!("Phase 9: Persistence + reconnect")`). Both compile against the final trait shape; Phase 7/8/9 plans only fill bodies.
- 8 tests pass:
  1. `pty_transport_is_object_safe` — `Box<dyn PtyTransport>` compiles
  2. `domain_is_object_safe` — `Box<dyn Domain>` compiles
  3. `local_domain_constructs_when_shell_resolves` — shell resolution chain works on a host with `/bin/zsh` or `/bin/bash` or `$SHELL`
  4. `codespace_domain_compiles_with_unimplemented_body` — label "codespace", not alive
  5. `devtunnel_domain_compiles_with_unimplemented_body` — label "dev_tunnel", not alive
  6. `codespace_spawn_panics_with_phase_marker` (`#[should_panic(expected = "Phase 7")]`)
  7. `devtunnel_spawn_panics_with_phase_marker` (`#[should_panic(expected = "Phase 8")]`)
  8. `local_domain_spawn_yields_reader_and_clean_exit` — **the critical D-38 reachability proof**: `LocalDomain::spawn(SpawnCommand { argv: Some(vec!["sh", "-c", "echo hi"]) ... })` -> `Box<dyn PtyTransport>` -> `take_reader()` collects bytes containing "hi" -> `wait()` returns `Ok(Some(0))`. CORE-04 + CORE-05 carry through the trait surface, not just direct LocalPty.
- No `unsafe` (workspace `unsafe_code = "deny"` holds).
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo fmt --all -- --check` clean.
- Plan 02-03's 5 vector-pty integration tests still pass (no regression).
- Plan 02-02's 26 vector-term tests still pass (no regression).

## Task Commits

1. **Task 1: Define `PtyTransport` + `Domain` traits + Cargo wiring** — `b88a02d` (feat)
2. **Task 2: `LocalDomain` full impl + Codespace/DevTunnel stubs + object-safety + reachability tests** — `c0ad634` (feat)

## Files Created/Modified

### Created (5)

- `crates/vector-mux/src/transport.rs` — `pub enum TransportKind { Local, Codespace, DevTunnel }` + `#[async_trait] pub trait PtyTransport: Send + 'static` with 5 methods.
- `crates/vector-mux/src/domain.rs` — `pub struct SpawnCommand { argv, cwd, rows, cols, env }` (derives `Debug, Clone, Default`) + `#[async_trait] pub trait Domain: Send + Sync` with 4 methods.
- `crates/vector-mux/src/local_domain.rs` — `pub struct LocalDomain { shell: PathBuf }` + `impl Domain` + `pub struct LocalTransport(LocalPty)` + `impl PtyTransport for LocalTransport` + private `fn resolve_shell() -> Result<PathBuf>`.
- `crates/vector-mux/src/codespace_domain.rs` — `pub struct CodespaceDomain { _private: () }` + `impl Domain` with Phase 7 + Phase 9 markers.
- `crates/vector-mux/src/devtunnel_domain.rs` — `pub struct DevTunnelDomain { _private: () }` + `impl Domain` with Phase 8 + Phase 9 markers.

### Modified (4 + Cargo.lock)

- `crates/vector-mux/Cargo.toml` — added `async-trait = { workspace = true }`, `tokio = { workspace = true }`, `vector-pty = { path = "../vector-pty" }` to `[dependencies]`; added `tokio` with rt-multi-thread/macros/time/sync features to `[dev-dependencies]`. Updated description.
- `crates/vector-mux/src/lib.rs` — retired the Phase-1 stub `Domain` (no methods) and `Pane` (marker trait) and `_force_anyhow_use` helper; module tree (`codespace_domain, devtunnel_domain, domain, local_domain, transport`) + `pub use` re-exports.
- `crates/vector-mux/tests/trait_object_safety.rs` — replaced 2 `#[ignore]` stubs with 8 real tests; `LocalTransport` is publicly exposed for completeness but the test file uses only `LocalDomain` + `Box<dyn PtyTransport>`.
- `crates/vector-pty/src/local.rs` — single-line change: `LocalPty::write(&self, …)` → `LocalPty::write(&mut self, …)`. Required by the trait-object Send-future constraint (see Decisions).
- `Cargo.lock` — vector-mux dep edge to vector-pty resolves cleanly; no new transitive crates.

## Public API (final, Phases 4/7/8/9 compile against this)

```rust
// crates/vector-mux/src/transport.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind { Local, Codespace, DevTunnel }

#[async_trait::async_trait]
pub trait PtyTransport: Send + 'static {
    fn resize(&mut self, rows: u16, cols: u16, px_w: u16, px_h: u16) -> Result<()>;
    async fn write(&mut self, bytes: &[u8]) -> Result<()>;
    fn take_reader(&mut self) -> Option<mpsc::Receiver<Vec<u8>>>;
    fn kind(&self) -> TransportKind;
    async fn wait(&mut self) -> Result<Option<i32>>;
}

// crates/vector-mux/src/domain.rs
#[derive(Debug, Clone, Default)]
pub struct SpawnCommand {
    pub argv: Option<Vec<String>>,
    pub cwd: Option<PathBuf>,
    pub rows: u16,
    pub cols: u16,
    pub env: Vec<(String, String)>,
}

#[async_trait::async_trait]
pub trait Domain: Send + Sync {
    async fn spawn(&self, cmd: SpawnCommand) -> Result<Box<dyn PtyTransport>>;
    fn label(&self) -> String;
    fn is_alive(&self) -> bool;
    async fn reconnect(&self) -> Result<()>;
}

// crates/vector-mux/src/local_domain.rs
pub struct LocalDomain { /* shell: PathBuf */ }
impl LocalDomain {
    pub fn new() -> Result<Self>;                  // resolves $SHELL chain
    pub fn with_shell(shell: PathBuf) -> Self;     // explicit for tests
}
pub struct LocalTransport(LocalPty);               // newtype wrapper for impl
impl Domain for LocalDomain { ... }
impl PtyTransport for LocalTransport { ... }
```

## Decisions Made

- **Newtype wrapper `LocalTransport(LocalPty)` in vector-mux, not `impl PtyTransport for LocalPty` in vector-pty.** This is the only way to (a) keep the trait surface in vector-mux per D-38 AND (b) avoid a vector-pty → vector-mux dep cycle. The plan explicitly walked through both arms and committed to this; we executed it as written.
- **`LocalPty::write(&self) -> (&mut self)` in vector-pty.** Forced by the trait-object Send-future constraint: `Box<dyn PtyTransport>::write(&mut self)`'s returned future must be `Send`, which means the borrow of `LocalPty` inside the async block must be Send. `&LocalPty: Send` requires `LocalPty: Sync`, but `Box<dyn portable_pty::MasterPty + Send>` is `!Sync` (portable-pty 0.9's `MasterPty` trait is `Send` but not `Sync`). `&mut LocalPty: Send` only requires `LocalPty: Send`, which is the case. This is a vector-pty signature change discovered during integration; no other Plan 02-03 callers use `LocalPty::write` (the 5 lifecycle/term-env tests never call it), so the change is internal-only and zero-risk to Plan 02-03's contracts.
- **`reconnect()` on CodespaceDomain / DevTunnelDomain panics with `"Phase 9: Persistence + reconnect"` not the transport-owning phase number.** Reconnect logic lives in Phase 9 (Persistence + Reconnect + tmux Auto-Attach); the transport phases only ship the spawn body. Markers reflect the phase that owns the body.
- **`LocalDomain` carries the shell path as `PathBuf`, not the unresolved env var string.** `LocalDomain::new()` resolves at construction; `LocalDomain::with_shell(PathBuf)` is the explicit-injection escape hatch used by the end-to-end reachability test (which sets `/bin/sh` to keep the test deterministic across hosts with different `$SHELL`).
- **Object-safety asserted via `fn accepts_boxed(b: Box<dyn Trait>) { drop(b); }` + `let f: fn(_) = accepts_boxed; assert!(fn_addr_eq(f, accepts_boxed as _))`.** The plan's original `let _b: Box<dyn Trait> = ...` form trips clippy `no_effect_underscore_binding` under workspace pedantic. The fn-pointer form sidesteps the lint while still serving as a compile-time check (the fn signature itself fails to compile if `Box<dyn Trait>` isn't a valid type).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `LocalPty::write(&self)` doesn't yield a Send future when wrapped in `impl PtyTransport`**

- **Found during:** Task 2 (initial `cargo build -p vector-mux` after creating `LocalTransport`).
- **Issue:** `impl PtyTransport for LocalTransport` delegates `async fn write(&mut self)` to `self.0.write(bytes).await` where `self.0` is `LocalPty`. The plan's signature `LocalPty::write(&self, …)` requires the async block's captured `&LocalPty` to be `Send`, which requires `LocalPty: Sync`. But `LocalPty` contains `master: Box<dyn portable_pty::MasterPty + Send>` — `MasterPty` in portable-pty 0.9 is `Send` but NOT `Sync`. Compile error: `the trait bound 'Box<dyn MasterPty + Send>: Sync' is not satisfied`.
- **Fix:** Changed `LocalPty::write(&self, bytes: &[u8])` → `LocalPty::write(&mut self, bytes: &[u8])` in `crates/vector-pty/src/local.rs`. `&mut LocalPty: Send` requires only `LocalPty: Send`, which it is. Added a 3-line code comment explaining why. No vector-pty test calls `.write` (the 5 lifecycle/term-env tests only construct + read + resize + wait — `.write` is exercised by Plan 02-04 reachability test and Plan 02-05's input pump only).
- **Files modified:** `crates/vector-pty/src/local.rs`
- **Verification:** `cargo test -p vector-pty --tests` still passes 5/5; `cargo build -p vector-mux` now succeeds.
- **Committed in:** `b88a02d` (Task 1 commit — bundled because the trait shape + the vector-pty signature change are inseparable).

**2. [Rule 1 - Bug] Clippy `no_effect_underscore_binding` on `let _b:` / `let _f:` object-safety markers**

- **Found during:** Task 2 (after `cargo clippy -p vector-mux --all-targets -- -D warnings`).
- **Issue:** Workspace `clippy::pedantic` treats `let _b: Box<dyn Trait> = ...;` and `let _f: fn(_) = ...;` as no-effect bindings.
- **Fix:** Rewrote the two object-safety tests to use a named function (`fn accepts_boxed(b: Box<dyn Trait>) { drop(b); }`) and assert non-trivially via `std::ptr::fn_addr_eq(f, accepts_boxed as fn(_))`. The fn signature itself remains the compile-time object-safety check.
- **Files modified:** `crates/vector-mux/tests/trait_object_safety.rs`
- **Committed in:** `c0ad634` (Task 2 commit).

**3. [Rule 1 - Bug] Clippy `while_let_loop` on the end-to-end reachability test**

- **Found during:** Task 2 clippy run.
- **Issue:** `loop { match … { Ok(Some(x)) => …, Ok(None) | Err(_) => break } }` can be rewritten as `while let Ok(Some(x)) = … { … }`.
- **Fix:** Applied the clippy-suggested rewrite. Semantically identical.
- **Files modified:** `crates/vector-mux/tests/trait_object_safety.rs`
- **Committed in:** `c0ad634` (Task 2 commit).

**4. [Rule 1 - Bug] rustfmt wraps `anyhow::bail!(long-message)` and `assert!(fn_addr_eq(...))` across multiple lines**

- **Found during:** Task 2 (after `cargo fmt --all`).
- **Issue:** Single-line `anyhow::bail!("...long...")` and `assert!(std::ptr::fn_addr_eq(f, accepts_boxed as ...))` exceed rustfmt's max width.
- **Fix:** Let `cargo fmt --all` apply its wrapping.
- **Files modified:** `crates/vector-mux/src/local_domain.rs`, `crates/vector-mux/tests/trait_object_safety.rs`
- **Verification:** `cargo fmt --all -- --check` exits 0.
- **Committed in:** `c0ad634` (Task 2 commit).

---

**Total deviations:** 4 auto-fixed (1 Rule 3 blocking signature change in vector-pty, 3 Rule 1 lint/format compliance).

**Impact on plan:** Deviation #1 is the only substantive change — `LocalPty::write(&self)` → `&mut self` is a vector-pty surface change discovered while wiring the trait impl. The plan's `<interfaces>` block noted this exact mismatch (`LocalPty::write(&self, bytes: &[u8])` is what Plan 02-03 shipped) and proposed wrapper-struct routing; the wrapper-struct alone wasn't sufficient because the trait-object Send-future bound still required the underlying call site to allow `&mut self` borrow. The fix is internal to vector-pty's API and doesn't touch its tests. Deviations #2–#4 are mechanical lint compliance.

## Issues Encountered

None blocking. The Sync/Send dance around `Box<dyn MasterPty + Send>` was the only genuine integration surprise — the plan's `<interfaces>` block had flagged the `&self` vs `&mut self` mismatch but proposed wrapping rather than re-signing; turns out the re-sign is mandatory because trait-object async fn futures must be Send.

## Verification Results

Final state of plan-level verification (all green):

```
cargo build -p vector-mux                                          ✓ compiles
cargo test -p vector-mux --tests                                   ✓ 8 + arch-lint = 9 pass, 0 fail
cargo build --workspace                                            ✓ 15 crates compile
cargo test -p vector-pty --tests                                   ✓ 5 + arch-lint = 6 pass (regression — Plan 02-03 untouched)
cargo test -p vector-term --tests                                  ✓ 26 pass (regression — Plan 02-02 untouched)
cargo clippy --workspace --all-targets -- -D warnings              ✓ clean
cargo fmt --all -- --check                                         ✓ clean
```

Grep invariants:

```
grep -c 'Box<dyn PtyTransport>' crates/vector-mux/tests/trait_object_safety.rs        5  ✓ ≥ 1
grep -c 'Box<dyn Domain>'        crates/vector-mux/tests/trait_object_safety.rs        4  ✓ ≥ 1
grep -c 'unimplemented!("Phase 7'  crates/vector-mux/src/codespace_domain.rs           1  ✓ = 1
grep -c 'unimplemented!("Phase 8'  crates/vector-mux/src/devtunnel_domain.rs           1  ✓ = 1
grep -c 'unimplemented!("Phase 9'  crates/vector-mux/src/{codespace,devtunnel}_domain.rs 2 ✓ = 2
grep -c 'impl PtyTransport' crates/vector-pty/src/local.rs                              0  ✓ = 0 (no trait impl in vector-pty)
grep -c '^vector-mux' crates/vector-pty/Cargo.toml                                      0  ✓ no dep cycle
grep -c 'unsafe' crates/vector-mux/src/*.rs                                             0  ✓ no unsafe
```

## Hand-off Notes for Downstream Plans

### Plan 02-05 (vector-headless pass-through proxy, Wave 4)

- **Constructor:**

  ```rust
  let domain = vector_mux::LocalDomain::new()?;          // resolves $SHELL automatically
  let cmd = vector_mux::SpawnCommand {
      argv: None,                                         // None = login shell
      cwd: None,
      rows, cols,
      env: vec![],
  };
  let mut transport: Box<dyn vector_mux::PtyTransport> = domain.spawn(cmd).await?;
  ```

- **Reader pump:** `let mut rx = transport.take_reader().expect("take_reader first call");` — call exactly once; pump bytes into `vector_term::Term::feed(&[u8])`.
- **Writer pump:** `transport.write(&bytes).await?` per parent-stdin chunk. Note: `write` takes `&mut self` on the trait object — design your single-actor pattern so only one task holds the `&mut Box<dyn PtyTransport>` at a time (Plan 02-05's actor pattern is the canonical solution; you cannot share `&mut` across the reader + writer + sigwinch tasks).
- **Resize:** `transport.resize(rows, cols, 0, 0)?` plus `term.resize(cols, rows)`. PTY-first ordering still applies.
- **Lifecycle:** `transport.wait().await` returns `Ok(Some(exit_code))` when child exits. Then drop the transport; the underlying LocalPty's Drop already kills+waits.
- **Where Domain helps:** for the headless pass-through, you may NOT need `Domain` at all — you can call `LocalPty::spawn` directly (Plan 02-03 surface). But going through Domain is cheap and aligns vector-headless with the Phase 4 mux model that Phases 7/8 use, so the actor pattern is reusable. Choose whichever shape Plan 02-05's planner picked.

### Phase 4 (Mux — Tabs & Splits)

- `Pane` carries `Box<dyn PtyTransport>` (or a small enum if perf demands it; the trait object adds one vtable indirection per byte chunk, which is below noise floor for terminal I/O).
- `Tab` carries `Vec<Pane>` with a split layout.
- `Window` carries `Box<dyn Domain>` so a single window can host local + Codespace tabs concurrently.
- The Mux's spawn-new-tab path is `domain.spawn(SpawnCommand { rows, cols, ... }).await`.

### Phase 7 (SSH Transport + Codespaces Connect)

- **Only fill bodies.** Don't reshape the trait. The plan-level acceptance criterion for D-38 is "Phases 7/8/9 compile against the same trait surface".
- Replace `crates/vector-mux/src/codespace_domain.rs` `_private: ()` field with the real state (codespace name, GitHub token handle from vector-secrets, ssh keypair path).
- `CodespaceDomain::spawn` body: octocrab call to fetch tunnel auth → russh-over-DevTunnel transport → `Box::new(CodespaceTransport(channel))` returned as `Box<dyn PtyTransport>`.
- `CodespaceTransport`'s `take_reader()` returns an mpsc-bridged russh channel reader; `write` bridges to the channel writer; `resize` issues an SSH window-change request; `wait` watches for channel close.
- `kind() -> TransportKind::Codespace` (instead of `Local`).

### Phase 8 (Dev Tunnels Integration, spike-gated)

- Same shape as Phase 7. `DevTunnelDomain::spawn` body fills in based on spike outcome (subprocess vs vendored SDK vs deferred-to-v2). If deferred to v2, leave the stub in place and document in Phase 8 SUMMARY.
- `kind() -> TransportKind::DevTunnel`.

### Phase 9 (Persistence + Reconnect + tmux Auto-Attach)

- Fill `CodespaceDomain::reconnect()` and `DevTunnelDomain::reconnect()` bodies. The trait shape is locked; reconnect issues a new transport attach using cached `SpawnCommand` state (Phase 9 will need to plumb that — track in the Phase 9 plan).
- `LocalDomain::reconnect()` stays `Ok(())` — local reconnect is just "spawn again".

## Next Phase Readiness

- Plan 02-04 closes Phase 2 Wave 3. Plan 02-05 (vector-headless pass-through) is the only remaining Phase 2 plan and depends on `LocalDomain::spawn` returning `Box<dyn PtyTransport>` — ratified here.
- D-38 fulfilled. Phases 4/7/8/9 each compile against the same trait surface; no later phase can silently reshape contracts.
- No blockers identified.

## Self-Check: PASSED

All claimed files exist:

- crates/vector-mux/src/transport.rs — FOUND
- crates/vector-mux/src/domain.rs — FOUND
- crates/vector-mux/src/local_domain.rs — FOUND
- crates/vector-mux/src/codespace_domain.rs — FOUND
- crates/vector-mux/src/devtunnel_domain.rs — FOUND
- crates/vector-mux/Cargo.toml (modified) — FOUND
- crates/vector-mux/src/lib.rs (modified) — FOUND
- crates/vector-mux/tests/trait_object_safety.rs (modified) — FOUND
- crates/vector-pty/src/local.rs (modified) — FOUND

All claimed commits exist:

- b88a02d — FOUND (Task 1)
- c0ad634 — FOUND (Task 2)

---
*Phase: 02-headless-terminal-core*
*Plan: 04*
*Completed: 2026-05-11*
