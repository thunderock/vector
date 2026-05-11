# 0002. winit/tokio main-thread ownership

- Status: accepted
- Date: 2026-05-10
- Deciders: solo (user)
- Tags: phase-1, threading, win-05, pitfall-5

## Context and Problem Statement

macOS forces the AppKit event loop on the main thread. Async I/O (PTY, SSH,
HTTPS, OAuth, port-forward) is tokio-native. Mixing them naively (spawning
tokio on the main thread, or wrapping winit inside `#[tokio::main]`) causes
deadlocks and sporadic AppKit crashes that surface only under load (Phase 3
renderer, Phase 9 reconnect).

## Decision Drivers

- macOS hard requirement: AppKit on main thread
- Tokio runtime is multi-thread by default; we want that for I/O fan-out
- Cross-thread signal must be safe + cheap
- Future-self protection: regression must fail the build, not just lint

## Considered Options

- `#[tokio::main]` on `vector-app::main` (rejected — pulls tokio onto main,
  fights winit)
- `OnceCell<Runtime>` global (rejected — every async caller has to know about
  it; unclear ownership)
- Per-subsystem current-thread runtimes (rejected — no shared executor; type
  interop nightmare)
- winit on main + dedicated I/O thread spawning multi-thread tokio runtime,
  EventLoopProxy::send_event as the sole cross-thread channel

## Decision Outcome

winit on main + dedicated I/O thread, per D-08..D-11. `vector-app::main`
spawns a `std::thread` named `tokio-io`; that thread builds
`tokio::runtime::Builder::new_multi_thread().enable_all().build()` and calls
`rt.block_on(io_main(proxy))`. The main thread runs `event_loop.run_app(...)`.
Cross-thread signal: `EventLoopProxy::send_event(UserEvent)`.

## Pros and Cons of the Options

- **`#[tokio::main]`:** ergonomic; incompatible with AppKit main-thread rule.
- **Global runtime via OnceCell:** flexible; ownership obscured; encourages
  ad-hoc `block_on` calls throughout the codebase.
- **Per-subsystem current-thread:** no cross-subsystem `Send` futures; doubles
  the runtime count for no benefit.
- **Main thread = winit, dedicated I/O thread (chosen):** one `block_on`, one
  proxy. Mechanical to verify (ADR 0003).

## Consequences

- One `block_on(...)` in the entire codebase — at the bottom of the I/O
  thread. Allowlisted by the architecture-lint test (ADR 0003).
- A 500ms `Tick(n)` smoke test (D-10) proves the channel works under real
  AppKit + winit + tokio at startup.
- `clippy::await_holding_lock = "deny"` (D-11) prevents the related anti-pattern
  (Mutex held across `.await`) at the workspace lint level.
- Phase 3 renderer plugs in without re-architecting the threading model.
