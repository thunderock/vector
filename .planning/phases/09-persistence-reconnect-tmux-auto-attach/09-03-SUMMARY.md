---
phase: 09-persistence-reconnect-tmux-auto-attach
plan: 03
subsystem: pty-actor
tags: [persist-01, persist-02, pty-actor, reconnect, event-sink, backoff]
requires:
  - 09-01 # vector_mux::Domain::reconnect_one_shot; UserEvent::PaneReconnecting/Reconnected
provides:
  - "pub trait EventSink in crates/vector-app/src/pty_actor.rs"
  - "pub struct ProxyEventSink (newtype over EventLoopProxy<UserEvent>)"
  - "pub async fn pane_io_loop (Active → Reconnecting → Swapping state machine)"
  - "Backoff schedule constant BACKOFF_SCHEDULE_SECS = [1,2,4,8,16,30]"
  - "drain_reader_to_end (drain-before-swap discipline)"
  - "PtyActorRouter::spawn_pane(pane_id, transport, domain, profile_label, cancel)"
affects:
  - "crates/vector-app/src/pty_actor.rs (reworked actor + new public types)"
  - "crates/vector-app/src/main.rs (router callers pass Domain + label + cancel)"
  - "crates/vector-app/src/frame_tick.rs (CoalesceBuffer::peek_snapshot added)"
  - "crates/vector-app/Cargo.toml (dev-dep tokio test-util feature)"
tech-stack:
  added:
    - tokio test-util (dev-dep) — virtual-time pause/advance for backoff tests
  patterns:
    - "EventSink seam — actor never calls EventLoopProxy::send_event directly"
    - "Drain-before-swap via take_reader+recv-to-None before installing new transport"
    - "Bounded exponential backoff with 30s tail cap (locked schedule)"
    - "CancellationToken-driven prompt actor teardown (< 50 ms)"
key-files:
  created:
    - crates/vector-app/tests/common/mod.rs
  modified:
    - crates/vector-app/src/pty_actor.rs
    - crates/vector-app/src/frame_tick.rs
    - crates/vector-app/src/main.rs
    - crates/vector-app/Cargo.toml
    - crates/vector-app/tests/pty_actor_reconnect.rs
    - crates/vector-app/tests/reconnect_byte_integrity.rs
decisions:
  - "EventSink trait + ProxyEventSink newtype live in pty_actor.rs (one canonical seam — Plan 09-05 will consume the same type, no parallel abstraction)"
  - "pane_io_loop made pub (not pub(crate)) so integration tests can drive it directly without a fake harness re-export"
  - "CoalesceBuffer::peek_snapshot is doc-hidden — production must not allocate-on-read; only byte-integrity tests use it"
  - "main.rs preserves legacy behavior via LocalDomain whose reconnect_one_shot returns Ok(None) (clean exit on EOF). Real DevTunnel reconnect lands in Plan 09-05"
metrics:
  duration_seconds: 395
  duration_human: 7m
  tasks_completed: 3
  files_changed: 6
  completed_date: 2026-05-22
---

# Phase 09 Plan 03: Per-pane reconnect state machine + EventSink seam

PERSIST-01 + PERSIST-02 land in `pane_io_loop`: reader EOF triggers drain-before-swap then exponential backoff (1/2/4/8/16/30s cap) calling `Domain::reconnect_one_shot`, with hot transport swap and `PaneReconnecting`/`PaneReconnected` event emission gated through a new `EventSink` trait so tests can drive the actor without a winit event loop.

## What landed

### Production code

- **`EventSink` trait** + **`ProxyEventSink`** newtype in `crates/vector-app/src/pty_actor.rs`. The actor calls `sink.send_user_event(...)` — never `EventLoopProxy::send_event` directly. `PtyActorRouter::new` wraps the supplied `EventLoopProxy<UserEvent>` once into `Arc<dyn EventSink>`; all `pane_io_loop` spawns clone that handle.
- **Reworked `pane_io_loop`** is now an outer `'outer:` loop containing the Active segment (`run_active_segment`) → Reconnect segment. Active runs the original biased `select!` over `cancel` / `resize_rx` / `write_rx` / `reader` and tracks latest dims. On `reader.recv() == None` it returns `ActiveExit::TransportDead`, the outer loop drains any in-flight reader bytes via `drain_reader_to_end`, awaits `transport.wait()`, and enters `reconnect_with_backoff`.
- **`reconnect_with_backoff`** emits `PaneReconnecting { pane_id, attempt, profile_label }` on every attempt starting at attempt=1, calls `domain.reconnect_one_shot(latest_rows, latest_cols)`, and on `Ok(Some(t))` returns `Swapped(t)`. `Ok(None)` returns `PermanentNone` (LocalDomain shell-death path). `Err(_)` logs at `warn` then sleeps the next slot — schedule index = `(attempt-1).min(5)` so attempts 7+ all sleep 30 s. Sleep is wrapped in `tokio::select!` with `cancel.cancelled()` for prompt teardown.
- **`PtyActorRouter::spawn_pane`** now takes `(pane_id, transport, domain: Arc<dyn Domain>, profile_label: String, cancel: CancellationToken)` and clones the router's stored `Arc<dyn EventSink>` into the spawned task.
- **`crates/vector-app/src/main.rs`** is updated at both `spawn_pane` call sites (bootstrap pane and split-pane handler). Both pass a `Arc<dyn Domain>` cloned from `LocalDomain` — whose `reconnect_one_shot` returns `Ok(None)` — so legacy "exit on EOF" semantics for local panes are preserved verbatim. Tunnel-side wiring (real `ReconnectableDevTunnelDomain`) is the responsibility of Plan 09-05; this plan deliberately does not touch the tunnel install path.

### Tests

- **`crates/vector-app/tests/common/mod.rs`** (NEW) provides shared fakes:
  - `FakeTransport::dead()` — reader yields `None` immediately (EOF on first `recv`).
  - `FakeTransport::piped()` — returns transport plus an `mpsc::Sender<Vec<u8>>` the test pushes bytes into.
  - `ScriptedDomain` — pops `ScriptStep::{Err,PermanentNone,Swap}` from a queue on each `reconnect_one_shot`.
  - `TestEventSink` — implements `vector_app::EventSink` over an `mpsc::UnboundedSender<UserEvent>`.
- **`crates/vector-app/tests/pty_actor_reconnect.rs`** — 4 tests:
  1. `pty_actor_enters_reconnecting_on_eof` — EOF triggers `PaneReconnecting{attempt:1, profile_label:"test-profile"}` then `PaneReconnected`.
  2. `pty_actor_exponential_backoff_schedule` — uses `#[tokio::test(start_paused = true)]` + `tokio::time::advance` to drive 4 failed reconnects across the locked schedule, then a successful swap on attempt 5. Asserts PaneReconnecting for attempts 1..=5 each.
  3. `pty_actor_cancels_backoff_on_pane_close` — domain errors forever; first PaneReconnecting fires, test calls `cancel.cancel()`, actor must terminate within 50 ms and emit `PaneExited` with NO `PaneReconnected`.
  4. `reconnect_emits_pane_reconnecting_event` — proves `profile_label` flows through verbatim.
- **`crates/vector-app/tests/reconnect_byte_integrity.rs`** — 2 tests:
  1. `reconnect_drains_old_transport_before_swap` — 1024 bytes pushed through OLD transport land in the coalesce buffer before `PaneReconnected` fires; 256 bytes pushed through NEW append cleanly to a final 1280-byte buffer.
  2. `reconnect_zero_byte_loss_under_urandom` — 8 KiB deterministic LCG payload split 50/50 across OLD and NEW transports; final coalesce buffer equals the original payload byte-for-byte. SC#2 zero-byte-loss invariant.

### Dev-dependency

- `crates/vector-app/Cargo.toml` enables `tokio` `test-util` feature in `[dev-dependencies]` so `tokio::time::pause/advance` is available for virtual-time backoff testing. Zero production impact (dev-only).

## Verification snapshot

```
$ cargo build --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.36s

$ cargo test -p vector-app --lib pty_actor
test result: ok. 3 passed; 0 failed; 0 ignored

$ cargo test -p vector-app --test pty_actor_reconnect
test result: ok. 4 passed; 0 failed; 0 ignored

$ cargo test -p vector-app --test reconnect_byte_integrity
test result: ok. 2 passed; 0 failed; 0 ignored
```

All plan acceptance grep checks pass:

- `pub trait EventSink` → 1 hit
- `pub struct ProxyEventSink` → 1 hit
- `impl EventSink for ProxyEventSink` → 1 hit
- `Arc<dyn EventSink>` → 7 hits (router field + multiple `pane_io_loop`/helper params)
- `BACKOFF_SCHEDULE_SECS` declared as `&[1, 2, 4, 8, 16, 30]`
- `reconnect_with_backoff` + `drain_reader_to_end` + `ReconnectOutcome::PermanentNone` all present
- `#[ignore` count = 0 across both new test files

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Enabled tokio `test-util` feature**

- **Found during:** Task 2 verification
- **Issue:** `tokio::time::advance` and `start_paused` attribute are gated behind tokio's `test-util` feature. The workspace tokio dep didn't enable it.
- **Fix:** Added a tokio dev-dependency in `crates/vector-app/Cargo.toml` with `test-util` + standard runtime features. Zero production impact.
- **Files modified:** `crates/vector-app/Cargo.toml`
- **Commit:** a9edd93

**2. [Rule 2 - Critical functionality] Kept a raw `EventLoopProxy<UserEvent>` for `frame_tick`**

- **Found during:** Task 1 refactor
- **Issue:** Plan instructions say "drop the raw proxy field on `PtyActorRouter`". But `frame_tick_loop` still takes `EventLoopProxy<UserEvent>` directly (Phase 4 code, untouched by this plan). Naively removing the proxy field would break frame_tick spawn.
- **Fix:** `PtyActorRouter` now holds BOTH `sink: Arc<dyn EventSink>` (used by `pane_io_loop`) AND `proxy_for_frame_tick: EventLoopProxy<UserEvent>` (used at frame_tick spawn). The actor still goes through `sink` exclusively. Migrating frame_tick to EventSink is out-of-scope for this plan (Plan 09-05 territory if it ever becomes interesting).
- **Files modified:** `crates/vector-app/src/pty_actor.rs`
- **Commit:** cb83b75

**3. [Rule 3 - Blocking] Added `CoalesceBuffer::peek_snapshot`**

- **Found during:** Task 3 implementation
- **Issue:** `CoalesceBuffer::drain()` is consume-on-read — the byte-integrity test needs to read intermediate buffer state without removing bytes. Plan suggested adding a `#[cfg(test)]` snapshot helper but `cfg(test)` is invisible to integration tests in `tests/`.
- **Fix:** Added `pub fn peek_snapshot(&self) -> Vec<u8>` marked `#[doc(hidden)]` with a doc-comment forbidding production use.
- **Files modified:** `crates/vector-app/src/frame_tick.rs`
- **Commit:** b9d2d2d

### Architectural deviations

None.

### Out-of-scope items deferred

- Real `ReconnectableDevTunnelDomain` wire-up at the tunnel pane install site. The picker actor (`devtunnels_actor.rs`) does NOT currently call `spawn_pane` directly — tunnel transports flow through `Mux::create_tab_async_with_transport` and then `DevTunnelPaneReady`. The path from there to a per-pane actor is owned by Plan 09-05, which will add the reconnect-aware wiring. This plan deliberately leaves that path untouched.

## Known Stubs

None. All paths the plan introduced are wired end-to-end (with `LocalDomain`'s `Ok(None)` preserving legacy local-shell-death semantics).

## Self-Check: PASSED

Verified:
- `crates/vector-app/src/pty_actor.rs` — FOUND
- `crates/vector-app/src/frame_tick.rs` — FOUND
- `crates/vector-app/src/main.rs` — FOUND
- `crates/vector-app/Cargo.toml` — FOUND
- `crates/vector-app/tests/common/mod.rs` — FOUND
- `crates/vector-app/tests/pty_actor_reconnect.rs` — FOUND
- `crates/vector-app/tests/reconnect_byte_integrity.rs` — FOUND

Commits:
- cb83b75 (Task 1) — FOUND
- a9edd93 (Task 2) — FOUND
- b9d2d2d (Task 3) — FOUND
