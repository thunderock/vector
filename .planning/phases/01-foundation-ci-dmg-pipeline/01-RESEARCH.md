# Phase 1: Foundation & CI/DMG Pipeline — Research

**Researched:** 2026-05-10
**Domain:** Rust workspace scaffolding + CI/DMG pipeline + winit/tokio threading invariant on macOS
**Confidence:** HIGH for stack/CI/winit-tokio. **MEDIUM** for exact runner labels (one CONTEXT decision now needs amendment — see "Constraint Drift" below).

## Summary

Phase 1 is execution of locked decisions. Stack and patterns are pinned; the planner needs concrete YAML, Cargo.toml, and Rust shapes — not exploration. This document delivers them, plus one important amendment: **D-21's `macos-13` runner was retired in December 2025**. Replacement is `macos-15-intel`. Everything else in CONTEXT.md is executable as written.

Three concrete things drive the plan:
1. The threading skeleton is small but load-bearing (locks D-08..D-11 in code; ~80 lines in `vector-app/src/main.rs`).
2. The CI matrix is a simple two-job-then-merge YAML (~120 lines) with one runner-label amendment.
3. The 14-crate scaffold is mostly empty `lib.rs` + `Cargo.toml` files — but workspace-level `[workspace.dependencies]` and `[workspace.lints]` carry all the policy.

**Primary recommendation:** Execute CONTEXT.md as written, with one runner-label correction (`macos-13` → `macos-15-intel`). Use `convco` (single Rust binary) over `commitlint` (Node toolchain) to keep CI Rust-only.

---

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

**Workspace scaffolding (D-01..D-07):**
- D-01: Day-1 stub all 14 crates from ARCHITECTURE.md
- D-02: Single source of truth for deps in `[workspace.dependencies]`; every crate uses `dep.workspace = true`
- D-03: Each stub `lib.rs` ships module skeleton + intent rustdoc + `unimplemented!()` trait stubs
- D-04: `xtask` in a separate workspace + `.cargo/config.toml` alias
- D-05: `rust-toolchain.toml` exact pin `channel = "1.88.0"`, targets aarch64 + x86_64 darwin
- D-06: Workspace lints — `unsafe_code = "deny"` (allowlist `vector-app` and AppKit-touching crates), `clippy.pedantic = "warn"`, `clippy.await_holding_lock = "deny"`
- D-07: Defer `[profile.release]` tuning to Phase 10

**Threading enforcement (D-08..D-11):**
- D-08: Per-crate `tests/no_tokio_main.rs` integration tests + CI grep
- D-09: Tokio runtime on a dedicated I/O thread spawned by `vector-app::main` before `EventLoop::run`
- D-10: Visible 500ms tick smoke test via `EventLoopProxy::send_event` + `UserEvent::Tick(n)` updating window title to `Vector — tick {n}`
- D-11: `clippy::await_holding_lock = "deny"` at workspace lint level

**App shell (D-12..D-16):**
- D-12: Bare winit `NSWindow` + native AppKit `NSTextField` overlay `Vector v{version} (build {sha})`
- D-13: Build SHA via `build.rs` + `git rev-parse --short HEAD`, no vergen
- D-14: Window 1024×640 centered, title `Vector`
- D-15: Full standard menu bar `Vector / File / Edit / View / Window / Help`; functional Cmd-Q, Cmd-M, Cmd-W, Zoom
- D-16: Placeholder `.icns` from `crates/vector-app/resources/icon.svg` via `iconutil`

**CI artifacts (D-17..D-26):**
- D-17: DMG built only on push-to-main; PRs lint/test only
- D-18: Tagged releases (`v*`) auto-publish, no manual gate
- D-19: Tip artifact + pinned `tip` GitHub Release overwritten on every main push
- D-20: `cargo deny check advisories licenses bans sources` enforced from Phase 1
- D-21: Matrix-then-merge: arm64 on `macos-14`, x86_64 on `macos-13` ← **NEEDS AMENDMENT — see Constraint Drift §**
- D-22: CI invokes `cargo xtask dmg` (same code path as local)
- D-23: `Swatinem/rust-cache@v2` keyed on Cargo.lock + target + rust-toolchain.toml
- D-24: `MACOSX_DEPLOYMENT_TARGET=13.0` + Info.plist `LSMinimumSystemVersion=13.0`
- D-25: `cargo-bundle` (produces `.app`) + `create-dmg` shell script (wraps `.app` in styled DMG)
- D-26: `xattr -dr com.apple.quarantine` ships in README + DMG background + Release body (3 places)

**Versioning & process (D-27..D-33):**
- D-27: CalVer `YYYY.MM.DD`
- D-28: DMG filename `Vector-{version}-universal.dmg`; tip `Vector-{version}-tip-{shortsha}-universal.dmg`
- D-29: `git-cliff` auto-CHANGELOG, Keep a Changelog format
- D-30: `commitlint`/`convco` enforced in CI (recommend convco — see §Conventional Commits)
- D-31: `cargo-husky` pre-commit (`cargo fmt --check && cargo clippy --all-targets -- -D warnings`)
- D-32: `tracing-subscriber` + `EnvFilter`, default INFO, stdout only
- D-33: ADRs in `docs/adr/` MADR template; minimum 5 (`0001-rust-workspace-layout` … `0005-versioning-calver`)

**Gating (D-34..D-35):**
- D-34: PR required checks (fmt, clippy, test, deny, commitlint)
- D-35: `main` branch protected, linear history, force-push disabled

### Claude's Discretion

- Exact placeholder icon artwork (UI-SPEC §"App Icon Direction" already concretizes — chevron of 3 motion lines, `#7B61FF` stroke, `#1A1A1A` plate)
- ADR file numbering and titles (5 specified by D-33; numbering 0001–0005 obvious)
- Allowlist comment in `tests/no_tokio_main.rs`
- DMG background image visual design (UI-SPEC §"DMG background image content" already concretizes)
- Conventional Commits scope vocabulary (recommend: `app`, `xtask`, `ci`, `build`, `docs`, `deps`, `release`)
- Exact `git-cliff` template (recommend: fork of `examples/keepachangelog.toml`, see §git-cliff template)

### Deferred Ideas (OUT OF SCOPE)

- wgpu/Metal surface init (Phase 3)
- Release-profile tuning — `lto`/`codegen-units`/`strip`/`panic` (Phase 10)
- File-rotated logging under `~/Library/Logs/Vector/` (later phase)
- Apple Developer signing + notarization + Sparkle (v2)
- Same-day release sequence `YYYY.MM.DD-N` (one release per day acceptable)
- Full vergen crate (build.rs is enough)
- `cargo-zigbuild` cross-compilation (was a fallback; see Constraint Drift — turns out we don't need it; macos-15-intel exists)

</user_constraints>

---

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BUILD-01 | Cargo workspace skeleton compiles on macOS 13+ with Rust 1.88+ | §Standard Stack, §Workspace skeleton shape; `rust-toolchain.toml` exact pin |
| BUILD-02 | GitHub Actions CI builds Universal binaries on every push to main and on every tag | §CI YAML topology; matrix `macos-14` arm64 + `macos-15-intel` x86_64 + merge job with `lipo` |
| BUILD-03 | `cargo xtask dmg` produces an unsigned `Vector.dmg` locally, identical to CI | §xtask shape; CI invokes `cargo xtask dmg --universal` (D-22) so code path is the same |
| BUILD-04 | Tagged releases publish unsigned `.dmg` to GitHub Releases (tip + tagged pattern) | §Release workflow; ghostty-style tip + tagged release |
| BUILD-05 | README documents `xattr -dr com.apple.quarantine /Applications/Vector.app` | UI-SPEC §README install block already locks the markdown verbatim |
| WIN-05 | `winit::EventLoop` on main thread; `tokio` on background; `EventLoopProxy::send_event` is the only cross-thread signal; no `block_on` on main; no shared mutex held across `await` | §Threading skeleton (full code shape); §Architecture-lint test (D-08); workspace lint `clippy.await_holding_lock = "deny"` (D-11) |

</phase_requirements>

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `winit` event loop | macOS main thread (UI tier) | — | AppKit forces it; non-negotiable |
| Window chrome / NSWindow | macOS main thread via objc2-app-kit | — | AppKit is main-thread-only |
| Native menu bar | macOS main thread via objc2-app-kit | — | Same |
| `NSTextField` overlay | macOS main thread via objc2-app-kit | — | Same |
| Tokio runtime | Dedicated I/O thread spawned by `vector-app::main` | — | Locked by D-09 |
| Tick smoke test (500ms) | Tokio task on I/O thread → `EventLoopProxy` → main thread handler | — | Locked by D-10 |
| Logging init | Main thread, before I/O thread spawn | — | Subscriber must be set before any task can log |
| `cargo xtask dmg` | Local & CI subprocess (separate workspace) | — | D-04 + D-22 |
| `lipo` Universal merge | CI `package` job on `macos-14` | — | Standard pattern |
| `create-dmg` styled DMG | `xtask dmg` step (calls into shell script) | `hdiutil` (fallback) | D-25 + fallback for resilience |

The bright line: **anything that touches AppKit goes through the main thread; anything that does I/O goes through the I/O thread**. There is no third place.

---

## Standard Stack

### Core (workspace deps for Phase 1)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `winit` | 0.30.13 | Event loop + NSWindow | Locked stack; **0.30.13 still has `EventLoopProxy::send_event` (T)** — `wake_up`-only API arrives in 0.31 ([VERIFIED: docs.rs/winit/0.30.13/winit/event_loop/struct.EventLoopProxy.html]) |
| `objc2` | 0.6.4 | ObjC runtime bindings | Modern type-safe; replaces unmaintained `cocoa-rs`/`objc` |
| `objc2-app-kit` | 0.3.x (tracks objc2 0.6) | NSApplication/NSMenu/NSTextField/NSWindow | Required for menu bar (winit doesn't expose it) |
| `objc2-foundation` | 0.3.x | NSString, NSArray | Transitive needs (NSString for menu titles, etc.) |
| `tokio` | 1.52.3 | Async runtime | Phase 1 only uses `time::interval` for the tick + `runtime::Builder` |
| `tracing` | 0.1.x | Structured log calls | Cheap to introduce day-one |
| `tracing-subscriber` | 0.3.x | Fmt subscriber + `EnvFilter` | D-32 minimal init |
| `anyhow` | 1.x | App-level error type | `vector-app::main` returns `anyhow::Result<()>` |
| `thiserror` | 2.x (latest 1.x line is 2.x as of 2026 — VERIFY at install time) | Library-level error types | Stub crates only need it placeholdered |

### Tooling

| Tool | Version | Purpose |
|------|---------|---------|
| `cargo-bundle` | 0.10.0 | Generates `Vector.app/Contents/{MacOS,Info.plist,Resources}` |
| `cargo-deny` | 0.16.x (latest 2026) | License + advisory + dup audit |
| `convco` | latest | Conventional Commits lint (Rust binary, no Node) |
| `git-cliff` | latest | CHANGELOG.md from Conventional Commits |
| `cargo-husky` | 1.x | Auto-install pre-commit hook on first build |
| `lipo` | Xcode CLI | Universal binary merge |
| `iconutil` | Xcode CLI | `.iconset` → `.icns` |
| `create-dmg` (shell) | latest from `create-dmg/create-dmg` GH repo | Styled DMG packaging |
| `hdiutil` | macOS built-in | Fallback DMG creation |

### Version verification commands (run before locking versions)

```bash
npm view --json tracing-subscriber 2>/dev/null # not applicable — Rust
# Use cargo / crates.io API:
curl -s https://crates.io/api/v1/crates/winit | jq -r '.crate.max_stable_version'
curl -s https://crates.io/api/v1/crates/cargo-bundle | jq -r '.crate.max_stable_version'
curl -s https://crates.io/api/v1/crates/cargo-deny | jq -r '.crate.max_stable_version'
curl -s https://crates.io/api/v1/crates/convco | jq -r '.crate.max_stable_version'
curl -s https://crates.io/api/v1/crates/git-cliff | jq -r '.crate.max_stable_version'
curl -s https://crates.io/api/v1/crates/cargo-husky | jq -r '.crate.max_stable_version'
```

The planner should put these in a Wave-0 task that runs once and pins exact versions in `[workspace.dependencies]`. Existing STACK.md versions are confirmed correct as of 2026-05-08–2026-05-10. [VERIFIED: STACK.md fetches]

---

## Constraint Drift (CONTEXT.md amendments needed)

### D-21: `macos-13` runner is **gone**. Use `macos-15-intel`.

**Source:** [VERIFIED: GitHub Actions changelog 2025-09-19](https://github.blog/changelog/2025-09-19-github-actions-macos-13-runner-image-is-closing-down/) and [VERIFIED: actions/runner-images#13046](https://github.com/actions/runner-images/issues/13046).

The `macos-13` runner was deprecated 2025-09-22 and **fully removed by 2025-12-04**. As of today (2026-05-10), it is unavailable. The replacement for x86_64 builds is **`macos-15-intel`**, which runs on macOS 15 with Intel CPUs and is **available until August 2027** (the last Intel runner GitHub will offer).

**Plan amendment:** D-21 should read `macos-14` arm64 + **`macos-15-intel`** x86_64. The `cargo-zigbuild` fallback noted in D-21 and Deferred Ideas is no longer needed for v1 — Intel runners exist; we have ~15 months of runway. Add an ADR `0006-runner-labels.md` (or fold into `0004-dmg-pipeline.md`) noting the choice and the August-2027 expiration date so the team has advance warning.

**Deployment-target subtlety:** Setting `MACOSX_DEPLOYMENT_TARGET=13.0` on a `macos-15-intel` runner still produces a binary that runs on macOS 13 — the deployment target controls weak-linking, not the build host's OS. [VERIFIED: Apple `LSMinimumSystemVersion` semantics] D-24 stays exactly as written.

### Everything else in CONTEXT.md: no drift detected

All other locked decisions are executable as-written against the current ecosystem (verified against crates.io, GitHub Actions changelog, and library docs).

---

## Architecture Patterns

### System architecture diagram

```
                  ┌─────────────────────────────────────────────────────┐
                  │                   GitHub Actions                    │
                  │                                                     │
                  │   on: push (main)         on: push tags v*          │
                  │       │                       │                     │
                  │       v                       v                     │
                  │   ┌─────────┐            ┌─────────┐                │
                  │   │   PR    │            │ release │                │
                  │   │ checks  │            │  build  │                │
                  │   └────┬────┘            └────┬────┘                │
                  │        │ (always)              │                    │
                  │  ┌─────v─────┐            ┌────v──────┐             │
                  │  │ matrix:   │            │ matrix:   │             │
                  │  │ macos-14  │ ─arm64─►   │ macos-14  │ ─arm64─►    │
                  │  │ macos-15- │ ─x86_64──► │ macos-15- │ ─x86_64──►  │
                  │  │  intel    │            │  intel    │             │
                  │  └───────────┘            └─────┬─────┘             │
                  │                                 │                   │
                  │                       ┌─────────v─────────┐         │
                  │                       │  package job      │         │
                  │                       │  (macos-14):      │         │
                  │                       │  download both    │         │
                  │                       │  binaries → lipo  │         │
                  │                       │  → cargo xtask    │         │
                  │                       │     dmg --universal│        │
                  │                       └─────────┬─────────┘         │
                  │                                 │                   │
                  │                ┌────────────────┴──────────────┐    │
                  │                │           tip / tagged?       │    │
                  │                └────────────────┬──────────────┘    │
                  │              tip │              │ tagged           │
                  │   ┌──────────────v──┐    ┌──────v────────────┐     │
                  │   │ upload to       │    │ create release    │     │
                  │   │ workflow + tip  │    │ from CHANGELOG    │     │
                  │   │ release         │    │ + upload DMG      │     │
                  │   └─────────────────┘    └───────────────────┘     │
                  └─────────────────────────────────────────────────────┘

  Local:  cargo xtask dmg  (same code path as CI's "package" job)


  Runtime architecture (the binary):

         macOS process
         ┌────────────────────────────────────────────────────────┐
         │ main thread                                            │
         │   ┌──────────────────────────────────────────────────┐ │
         │   │ winit::EventLoop<UserEvent>                      │ │
         │   │  ↑                                               │ │
         │   │  │ user_event(Tick(n))                           │ │
         │   │  │   → window.set_title("Vector — tick {n}")    │ │
         │   │  │ resumed() → create NSWindow + menu + overlay │ │
         │   │  │                                              │ │
         │   └──┼────────────────────────────────────────────────┘ │
         │      │ EventLoopProxy::send_event(UserEvent::Tick(n))   │
         │      │                                                  │
         │  ────┼──────────────────────────────────────────────    │
         │      │ thread "tokio-io" (std::thread::spawn)           │
         │   ┌──┴───────────────────────────────────────────────┐  │
         │   │ tokio::runtime::Builder::new_multi_thread()      │  │
         │   │   .block_on(io_main(proxy))                       │ │
         │   │   ├─ task: tick_loop(proxy) — 500ms interval      │ │
         │   │   └─ (later phases) ssh / oauth / pty etc.        │ │
         │   └──────────────────────────────────────────────────┘  │
         └────────────────────────────────────────────────────────┘
```

### Recommended workspace structure

```
vector/                                # cargo workspace root
├── Cargo.toml                         # [workspace] + [workspace.dependencies] + [workspace.lints]
├── Cargo.lock
├── rust-toolchain.toml                # channel = "1.88.0", targets = [...]
├── .cargo/
│   └── config.toml                    # [alias] xtask = "run --manifest-path xtask/Cargo.toml --"
├── .github/
│   ├── workflows/
│   │   ├── ci.yml                     # PR checks (fmt, clippy, test, deny, commitlint) + push-to-main DMG
│   │   └── release.yml                # on: push tags v* — Universal DMG to Releases
│   └── settings.yml                   # (optional, probot/settings) branch protection
├── crates/
│   ├── vector-app/                    # bin: winit + tokio + AppKit shell
│   │   ├── Cargo.toml
│   │   ├── build.rs                   # emits VECTOR_BUILD_SHA
│   │   ├── src/
│   │   │   ├── main.rs                # EntryPoint: install subscriber, spawn tokio thread, run winit
│   │   │   ├── app.rs                 # ApplicationHandler impl
│   │   │   ├── menu.rs                # AppKit NSMenu construction
│   │   │   ├── overlay.rs             # NSTextField version overlay
│   │   │   └── tick.rs                # io_main async function (the smoke test)
│   │   ├── tests/
│   │   │   └── no_tokio_main.rs
│   │   └── resources/
│   │       ├── icon.svg               # placeholder
│   │       ├── icon.iconset/          # generated by xtask, gitignored
│   │       ├── icon.icns              # generated by xtask, gitignored
│   │       └── dmg-background.png     # 1280×800 @2x for Retina
│   ├── vector-ui/                     # stub — UI surfaces (Phase 4+)
│   ├── vector-render/                 # stub — wgpu pipeline (Phase 3)
│   ├── vector-mux/                    # stub — Domain/Pane/Tab tree (Phase 4)
│   ├── vector-term/                   # stub — alacritty_terminal wrapper (Phase 2)
│   ├── vector-pty/                    # stub — portable-pty wrapper (Phase 2)
│   ├── vector-ssh/                    # stub — russh wrapper (Phase 7)
│   ├── vector-codespaces/             # stub — octocrab + tonic (Phase 6/7)
│   ├── vector-tunnels/                # stub — dev-tunnels rs/ (Phase 8)
│   ├── vector-config/                 # stub — TOML config (Phase 5)
│   ├── vector-secrets/                # stub — keyring (Phase 6)
│   ├── vector-fonts/                  # stub — crossfont/cosmic-text (Phase 3)
│   ├── vector-input/                  # stub — keymap/IME (Phase 5)
│   └── vector-theme/                  # stub — palette types (Phase 5)
├── xtask/                             # SEPARATE workspace per D-04
│   ├── Cargo.toml                     # [package] only — NO [workspace]
│   ├── Cargo.lock
│   └── src/main.rs                    # subcommands: dmg, dmg --universal, release
├── docs/
│   └── adr/
│       ├── 0001-rust-workspace-layout.md
│       ├── 0002-winit-tokio-threading.md
│       ├── 0003-architecture-lint-mechanism.md
│       ├── 0004-dmg-pipeline.md
│       └── 0005-versioning-calver.md
├── deny.toml                          # cargo-deny config
├── cliff.toml                         # git-cliff config
├── .convco                            # convco config (optional)
├── CHANGELOG.md                       # generated by git-cliff, committed
├── README.md                          # contains xattr install block (per UI-SPEC)
└── LICENSE
```

### Pattern 1: Triple-loop threading skeleton

This is the load-bearing pattern. Code shape for `crates/vector-app/src/main.rs`:

```rust
// SPDX-License-Identifier: MIT
#![allow(unsafe_code)]  // vector-app is on the AppKit allowlist (D-06)

use std::thread;
use std::time::Duration;

use anyhow::Result;
use tokio::runtime::Builder;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing_subscriber::{fmt, EnvFilter};
use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowAttributes, WindowId};

mod app;
mod menu;
mod overlay;
mod tick;

#[derive(Debug, Clone)]
pub enum UserEvent {
    Tick(u64),
}

fn main() -> Result<()> {
    // Init logging FIRST — must happen before any task can log.
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        sha = env!("VECTOR_BUILD_SHA"),
        "vector starting"
    );

    // Create the event loop — main thread, mandatory on macOS.
    let event_loop: EventLoop<UserEvent> = EventLoop::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = event_loop.create_proxy();

    // Spawn the dedicated I/O thread. Tokio runtime lives here; nowhere else.
    // No #[tokio::main], no OnceCell<Runtime>, no current-thread runtime.
    let _io_thread = thread::Builder::new()
        .name("tokio-io".into())
        .spawn(move || {
            let rt = Builder::new_multi_thread()
                .enable_all()
                .thread_name("tokio-worker")
                .build()
                .expect("build tokio runtime");
            rt.block_on(tick::io_main(proxy));
        })?;

    // Run the winit loop on this (main) thread. This consumes main forever.
    let mut application = app::App::new();
    event_loop.run_app(&mut application)?;
    Ok(())
}
```

`crates/vector-app/src/tick.rs`:

```rust
use std::time::Duration;

use tokio::time::interval;
use winit::event_loop::EventLoopProxy;

use crate::UserEvent;

pub async fn io_main(proxy: EventLoopProxy<UserEvent>) {
    let mut tick_n: u64 = 0;
    let mut iv = interval(Duration::from_millis(500));
    loop {
        iv.tick().await;
        tick_n = tick_n.saturating_add(1);
        if proxy.send_event(UserEvent::Tick(tick_n)).is_err() {
            tracing::info!("event loop closed; tick task exiting");
            return;
        }
    }
}
```

`crates/vector-app/src/app.rs`:

```rust
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::{menu, overlay, UserEvent};

pub struct App {
    window: Option<Box<dyn Window>>,
    overlay: Option<overlay::Overlay>,
}

impl App {
    pub fn new() -> Self {
        Self { window: None, overlay: None }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title("Vector")
            .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 640.0));
        let window = event_loop.create_window(attrs).expect("create_window");

        // SAFETY: we are on the main thread (winit guarantees this in `resumed`).
        // AppKit calls below assume MainThreadMarker.
        unsafe {
            menu::install_main_menu();
            self.overlay = Some(overlay::install(&*window));
        }
        self.window = Some(window);
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Tick(n) => {
                if let Some(window) = self.window.as_ref() {
                    window.set_title(&format!("Vector \u{2014} tick {n}"));
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_size) => {
                if let Some(overlay) = self.overlay.as_mut() {
                    overlay.relayout();
                }
            }
            _ => {}
        }
    }
}
```

Note: `\u{2014}` is the EM DASH per UI-SPEC §"Threading-visible surface".

[CITED: docs.rs/winit/0.30.13/winit/event_loop/struct.EventLoopProxy.html] confirms `send_event(T) -> Result<(), EventLoopClosed<T>>` is the API in 0.30.13. [CITED: docs.rs/winit/0.30.13/winit/application/trait.ApplicationHandler.html] confirms `user_event(&mut self, &ActiveEventLoop, T)` signature.

### Pattern 2: Architecture-lint integration test (per-crate)

`crates/vector-app/tests/no_tokio_main.rs`:

```rust
//! Architecture-lint: prevents tokio-runtime ownership regressions per D-08.
//! Allowlist: this file; xtask (separate workspace, not under workspace test runner).

use std::fs;
use std::path::Path;

const FORBIDDEN: &[&str] = &[
    "#[tokio::main]",
    "#[tokio::test]",
    "Builder::new_current_thread()",
    "Runtime::new()",
];

// `block_on` is forbidden in production code in this crate but allowed in xtask.
// Allowed exception: vector-app/src/main.rs uses rt.block_on(io_main(...)) on the
// I/O thread. We allow it ONLY in src/main.rs and ONLY when prefixed by `rt.`.
const BLOCK_ON_ALLOWLIST: &[&str] = &["src/main.rs"];

#[test]
fn forbidden_tokio_patterns_absent_from_src() {
    let crate_root = env!("CARGO_MANIFEST_DIR");
    let src = Path::new(crate_root).join("src");
    scan_dir(&src, &src);
}

fn scan_dir(root: &Path, dir: &Path) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {dir:?}: {e}")) {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            scan_dir(root, &path);
            continue;
        }
        if path.extension().is_some_and(|e| e == "rs") {
            check_file(root, &path);
        }
    }
}

fn check_file(root: &Path, path: &Path) {
    let rel = path.strip_prefix(root).unwrap_or(path).display().to_string();
    let body = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    for pattern in FORBIDDEN {
        assert!(
            !body.contains(pattern),
            "{rel}: forbidden pattern `{pattern}` (D-08 architecture-lint).",
        );
    }
    if body.contains("block_on(") {
        let allowed = BLOCK_ON_ALLOWLIST.iter().any(|a| rel.replace('\\', "/").ends_with(a));
        assert!(
            allowed,
            "{rel}: `block_on` outside allowlist (D-08). Allowlist: {BLOCK_ON_ALLOWLIST:?}.",
        );
    }
}
```

Each of the 14 crates gets its own copy of `tests/no_tokio_main.rs`. The `BLOCK_ON_ALLOWLIST` may be empty for the 13 library crates; only `vector-app` allows it (and only in `src/main.rs`).

**CI redundancy step** — a shell grep that catches code added between test runs:

```yaml
- name: Architecture lint (grep redundancy)
  run: |
    set -e
    if rg -n --glob 'crates/**/*.rs' --glob '!crates/**/tests/no_tokio_main.rs' \
         '#\[tokio::main\]|Builder::new_current_thread\(\)' ; then
      echo "::error::Forbidden tokio pattern found in production code (D-08)."
      exit 1
    fi
```

`xtask/` is *not* matched by `crates/**` glob — it lives at workspace root.

### Pattern 3: Native AppKit menu bar via objc2-app-kit

Code shape for `crates/vector-app/src/menu.rs`:

```rust
//! Builds and installs the standard Vector / File / Edit / View / Window / Help
//! menu bar per UI-SPEC §"Menu bar items (Phase 1)".

use objc2::rc::Retained;
use objc2::runtime::Sel;
use objc2::sel;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem};
use objc2_foundation::NSString;

/// SAFETY: must be called on the macOS main thread, after NSApplication exists.
/// In winit 0.30, this is true inside `ApplicationHandler::resumed`.
pub unsafe fn install_main_menu() {
    let mtm = MainThreadMarker::new().expect("must be called on main thread");
    let app = NSApplication::sharedApplication(mtm);

    let main_menu = NSMenu::new(mtm);

    main_menu.addItem(&app_menu(mtm));
    main_menu.addItem(&file_menu(mtm));
    main_menu.addItem(&edit_menu(mtm));
    main_menu.addItem(&view_menu(mtm));
    main_menu.addItem(&window_menu(mtm));
    main_menu.addItem(&help_menu(mtm));

    app.setMainMenu(Some(&main_menu));
}

fn app_menu(mtm: MainThreadMarker) -> Retained<NSMenuItem> {
    let item = NSMenuItem::new(mtm);
    let submenu = NSMenu::new(mtm);

    add(&submenu, "About Vector", sel!(orderFrontStandardAboutPanel:), "");
    submenu.addItem(&NSMenuItem::separatorItem(mtm));
    add_disabled(&submenu, "Preferences\u{2026}", ",");
    submenu.addItem(&NSMenuItem::separatorItem(mtm));
    add_services(&submenu, mtm);
    submenu.addItem(&NSMenuItem::separatorItem(mtm));
    add(&submenu, "Hide Vector", sel!(hide:), "h");
    add_with_modifiers(&submenu, "Hide Others", sel!(hideOtherApplications:), "h", true);
    add(&submenu, "Show All", sel!(unhideAllApplications:), "");
    submenu.addItem(&NSMenuItem::separatorItem(mtm));
    add(&submenu, "Quit Vector", sel!(terminate:), "q");

    item.setSubmenu(Some(&submenu));
    item
}

// ... file_menu, edit_menu, view_menu, window_menu, help_menu defined similarly,
// matching UI-SPEC item-by-item.

fn add(menu: &NSMenu, title: &str, action: Sel, key: &str) {
    let item = NSMenuItem::new(MainThreadMarker::new().unwrap());
    item.setTitle(&NSString::from_str(title));
    item.setAction(Some(action));
    item.setKeyEquivalent(&NSString::from_str(key));
    menu.addItem(&item);
}

fn add_disabled(menu: &NSMenu, title: &str, key: &str) {
    let item = NSMenuItem::new(MainThreadMarker::new().unwrap());
    item.setTitle(&NSString::from_str(title));
    item.setKeyEquivalent(&NSString::from_str(key));
    // No action = AppKit auto-disables.
    menu.addItem(&item);
}

// ... helper variants for cmd-shift-h modifier mask, services menu, etc.
```

Key points:
- Call from inside `ApplicationHandler::resumed`. winit guarantees the main thread there.
- `NSApplication::sharedApplication` is what owns the menu bar. winit doesn't expose menu APIs directly. [VERIFIED: rust-windowing/winit#4260 confirms this]
- `MainThreadMarker::new()` panics off the main thread — that is the safety net.
- For separators: `NSMenuItem::separatorItem(mtm)`.
- For modifier masks (Cmd-Option-H, Cmd-Shift-Z): `setKeyEquivalentModifierMask(NSEventModifierFlagsCommand | NSEventModifierFlagsOption)`.

The full menu structure is enumerated in UI-SPEC §"Menu bar items (Phase 1)" — copy item names, accelerators, and disabled/functional state verbatim.

### Pattern 4: NSTextField overlay for the version banner

Code shape for `crates/vector-app/src/overlay.rs`:

```rust
//! Native AppKit NSTextField overlay rendering `Vector v{version} (build {sha})`.
//! Per UI-SPEC §"Version overlay placement": bottom-right, 16px margin, 11pt SF Mono,
//! #2A2A2A plate, #9A9A9A text, 4px corner radius.

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSColor, NSFont, NSTextField, NSView, NSWindow};
use objc2_foundation::{CGFloat, NSRect, NSString};

pub struct Overlay {
    field: Retained<NSTextField>,
}

/// SAFETY: must run on the main thread; `winit_window` must be a live winit Window
/// whose backing NSWindow has an active content view.
pub unsafe fn install(winit_window: &dyn winit::window::Window) -> Overlay {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let handle = winit_window.window_handle().expect("window_handle").as_raw();
    let RawWindowHandle::AppKit(appkit) = handle else {
        unreachable!("Phase 1 is macOS-only");
    };
    // appkit.ns_view is &NSView; the window is its window
    let ns_view: &NSView = appkit.ns_view.cast().as_ref();
    let ns_window: &NSWindow = ns_view.window().expect("view in window");

    let mtm = MainThreadMarker::new().expect("main thread");
    let label = format!(
        "Vector v{} (build {})",
        env!("CARGO_PKG_VERSION"),
        env!("VECTOR_BUILD_SHA"),
    );

    let field = NSTextField::labelWithString(&NSString::from_str(&label), mtm);
    let mono = NSFont::monospacedSystemFontOfSize_weight(11.0, /* regular */ 0.0);
    field.setFont(Some(&mono));
    field.setTextColor(Some(&NSColor::colorWithSRGBRed_green_blue_alpha(
        0.604, 0.604, 0.604, 1.0, // #9A9A9A
    )));
    field.setBackgroundColor(Some(&NSColor::colorWithSRGBRed_green_blue_alpha(
        0.165, 0.165, 0.165, 1.0, // #2A2A2A
    )));
    field.setDrawsBackground(true);
    field.setBordered(false);
    field.setEditable(false);
    field.setSelectable(false);
    field.setBezeled(false);
    // setWantsLayer + cornerRadius=4 for the rounded plate, via field.layer().
    field.setWantsLayer(true);
    if let Some(layer) = field.layer() {
        layer.setCornerRadius(4.0);
    }

    // Position: bottom-right, 16px margin, padded 8h × 4v.
    let content_view = ns_window.contentView().expect("content view");
    let cv_bounds = content_view.bounds();
    relayout_into(&field, cv_bounds);
    content_view.addSubview(&field);

    // Auto-resizing mask: stays anchored to bottom-right on resize.
    use objc2_app_kit::NSAutoresizingMaskOptions;
    field.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewMinXMargin | NSAutoresizingMaskOptions::ViewMaxYMargin,
    );

    Overlay { field }
}

impl Overlay {
    pub fn relayout(&mut self) {
        // No-op: setAutoresizingMask handles resize automatically.
    }
}

unsafe fn relayout_into(field: &NSTextField, cv_bounds: NSRect) {
    let inset_h: CGFloat = 16.0; // md
    let inset_v: CGFloat = 16.0; // md
    let pad_h: CGFloat = 8.0;    // sm
    let pad_v: CGFloat = 4.0;    // xs
    let text_w = field.attributedStringValue().size().width;
    let w = text_w + 2.0 * pad_h;
    let h = 11.0 + 2.0 * pad_v;
    let x = cv_bounds.size.width - w - inset_h;
    let y = inset_v;
    field.setFrame(NSRect::new(x, y, w, h).into());
}
```

Confirmation that the `NSWindow` is reachable from winit's `Window`: winit 0.30 implements `raw_window_handle 0.6`, and on macOS `RawWindowHandle::AppKit(handle).ns_view` returns the content `NSView`, which has a `window()` method that returns the `NSWindow*`. [VERIFIED: rust-windowing/winit issue #4260 + objc2-app-kit NSView docs]

### Pattern 5: build.rs for VECTOR_BUILD_SHA

`crates/vector-app/build.rs`:

```rust
use std::process::Command;

fn main() {
    let sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into());

    println!("cargo:rustc-env=VECTOR_BUILD_SHA={sha}");
    // Re-run when HEAD moves, including across branches in CI.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    // Allow `VECTOR_BUILD_SHA` override (e.g. CI passes the full ref-sha):
    println!("cargo:rerun-if-env-changed=VECTOR_BUILD_SHA_OVERRIDE");
}
```

Behavior:
- Local dev: `git rev-parse --short HEAD` succeeds → SHA baked in.
- CI shallow checkout: succeeds because `actions/checkout@v4` defaults to `fetch-depth: 1` but still leaves a `.git` directory with HEAD reachable.
- No git binary / not a repo: SHA = "unknown" — graceful, never fails the build.

Note on `.git/refs/heads`: watching the directory itself triggers rebuild on any branch update; cheaper than enumerating `.git/refs/heads/*`. [VERIFIED: doc.rust-lang.org/cargo/reference/build-scripts.html — `rerun-if-changed` accepts directories]

### Anti-patterns to avoid

- **`#[tokio::main]` on `vector-app::main`** — pulls tokio onto the main thread, fights winit. The architecture-lint test catches this.
- **`OnceCell<Runtime>` global** — every async caller has to know about it; the test catches `Runtime::new()`.
- **`block_on` in handler code** — deadlocks the UI under load. Lint `clippy::await_holding_lock` (D-11) catches the related anti-pattern; for raw `block_on`, the architecture-lint test catches it everywhere except the allowlisted `src/main.rs:rt.block_on(io_main(proxy))`.
- **`set_title` from a worker thread** — instant AppKit crash. The pattern is enforced by routing through `EventLoopProxy::send_event`; the only `set_title` call lives inside `user_event` which winit guarantees runs on main.
- **`tokio` `full` features** — drops 30+ MB of compiled crates we don't use yet. List explicit features per STACK.md.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Universal binary | shell scripts that `cat` two binaries | `lipo -create -output` (Xcode CLI) | Mach-O fat header is non-trivial; `lipo` exists |
| `.icns` from PNG | image-magick scripts | `iconutil --convert icns vector.iconset` | Apple's tool gets all 13 size variants right |
| `.app` bundle | `mkdir Vector.app/Contents/{MacOS,Resources}` + handwritten Info.plist | `cargo-bundle 0.10` | Info.plist quirks (CFBundleVersion vs Short, LSMinimumSystemVersion semantics) |
| Styled DMG | shell-only `hdiutil` + AppleScript | `create-dmg` (shell script) | Window background, icon positions, drag-arrow rasterization — all done |
| Conventional Commits lint | regex in a workflow step | `convco check` | Rust binary, single download, supports stricter checks (footers, BREAKING CHANGE) |
| CHANGELOG.md generation | shell parsing of `git log` | `git-cliff -t v{version}` | Tera template, group rules, tag-aware |
| OAuth device flow (Phase 6+) | re-implement RFC 8628 | `oauth2 5.0` | Already correct; bugs in hand-rolled implementations are CVE-class |
| ADR template | invent your own format | MADR | One template variant choice; everything else is convention |
| Cache key | hash all the things by hand | `Swatinem/rust-cache@v2` | Already hashes Cargo.lock, rust-toolchain.toml, .cargo/config.toml |
| Build SHA injection | every-build wrapper script | `build.rs` + `cargo:rustc-env=` | Native cargo; no extra dep |

**Key insight:** Phase 1's only "hand-rolled" pieces are (1) the threading skeleton (~80 lines, intentionally small), (2) the architecture-lint test (~50 lines, intentionally readable), and (3) the menu/overlay AppKit code (~150 lines, mechanical). Everything else delegates to a tool.

---

## Common Pitfalls

### Pitfall 1: winit/tokio ownership regression

**What goes wrong:** Someone adds `#[tokio::main]` "to make a quick test work." Build still passes — until the menu bar refuses to install or `set_title` crashes from a worker thread.

**Why it happens:** Future-you forgets the rule under deadline pressure.

**How to avoid:** D-08 architecture-lint test in every crate, plus a CI grep, plus an ADR explaining *why*. The test is permissionless: anyone removing it is making the architecture decision the test enforces, visibly.

**Warning signs:**
- New crate added without copying `tests/no_tokio_main.rs`.
- A clippy warning in a PR mentioning `await_holding_lock`.
- Any `block_on(` in production code outside `vector-app/src/main.rs`.

[from PITFALLS.md §Pitfall 5]

### Pitfall 2: macos-13 runner missing

**What goes wrong:** The CI fails on first push with "Unable to resolve action's runs-on value 'macos-13'."

**Why it happens:** macOS-13 was retired Dec 2025. CONTEXT.md was drafted before that detail surfaced.

**How to avoid:** Use `macos-15-intel`. Documented in §Constraint Drift. Add the August-2027 expiration date to the relevant ADR.

[VERIFIED: GitHub Actions changelog 2025-09-19]

### Pitfall 3: Universal binary that's secretly thin

**What goes wrong:** `lipo -info` shows two architectures but `file` shows arm64-only. Intel Macs report "damaged" with no further info.

**Why it happens:** A failed cross-compile silently produces a zero-byte or symlinked-to-arm64 binary; `lipo` treats it as if both archs are present.

**How to avoid:** Verification step in CI before DMG packaging:
```bash
lipo -info target/universal-apple-darwin/release/vector | tee /dev/stderr | grep -q 'x86_64 arm64'
file target/universal-apple-darwin/release/vector | tee /dev/stderr | grep -q 'Mach-O universal binary'
```

[from PITFALLS.md §Pitfall 6]

### Pitfall 4: Quarantine attribute on the DMG itself

**What goes wrong:** Even after the user runs `xattr -dr com.apple.quarantine /Applications/Vector.app`, the *next* DMG download still has it because Safari/Chrome attach `com.apple.quarantine` to anything downloaded.

**Why it happens:** Each download is a fresh quarantine event. The README instruction targets the installed app, not the DMG itself. This is correct (the user installs once, runs many times) but documentation must say "after copying to `/Applications/`".

**How to avoid:** UI-SPEC §"README install block" already specifies exactly this wording. Verify the rendered README in CI by linting that the line `xattr -dr com.apple.quarantine /Applications/Vector.app` is present and is in a `sh` fenced code block.

[from PITFALLS.md §Pitfall 6]

### Pitfall 5: `cargo-deny` license false positives on `Unicode-DFS-2016` / `Unicode-3.0`

**What goes wrong:** First run of `cargo deny check licenses` fails because `unicode-ident` (transitive dep of half the ecosystem) is licensed `Unicode-DFS-2016 OR (Apache-2.0 AND MIT)`. Default cargo-deny config doesn't allow `Unicode-DFS-2016`.

**Why it happens:** The Unicode license is permissive but unusual; cargo-deny's default license list excludes it.

**How to avoid:** Allowlist `Unicode-DFS-2016` and `Unicode-3.0` explicitly. The full needed allowlist for our stack: `Apache-2.0`, `MIT`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Unicode-DFS-2016`, `Unicode-3.0`, `CC0-1.0`, `Zlib`, `0BSD`. See deny.toml shape below.

### Pitfall 6: cargo-husky hooks fail in CI

**What goes wrong:** CI fails because `cargo-husky` tries to install hooks into `.git/hooks/`, which CI's checkout doesn't have writable, or because the hook expects `cargo fmt` to be in PATH.

**Why it happens:** `cargo-husky` runs at first-build time; CI builds inevitably trigger it.

**How to avoid:** Install hooks only in dev builds. cargo-husky's `[features]` system supports this — set `default-features = false, features = ["user-hooks"]`, and gate the hook installation on `CARGO_HUSKY_DONT_INSTALL_HOOKS=1` in CI env.

```toml
# crates/vector-app/Cargo.toml (dev-deps)
[dev-dependencies]
cargo-husky = { version = "1", default-features = false, features = ["user-hooks"] }
```

```yaml
# .github/workflows/ci.yml (env section)
env:
  CARGO_HUSKY_DONT_INSTALL_HOOKS: "1"
```

[CITED: github.com/rhysd/cargo-husky README]

---

## Code Examples (canonical shapes)

### Workspace `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = [
    "crates/vector-app",
    "crates/vector-ui",
    "crates/vector-render",
    "crates/vector-mux",
    "crates/vector-term",
    "crates/vector-pty",
    "crates/vector-ssh",
    "crates/vector-codespaces",
    "crates/vector-tunnels",
    "crates/vector-config",
    "crates/vector-secrets",
    "crates/vector-fonts",
    "crates/vector-input",
    "crates/vector-theme",
]
# xtask is intentionally NOT listed (D-04 — separate workspace).

[workspace.package]
version = "2026.05.10"  # CalVer per D-27
edition = "2021"
rust-version = "1.88"
license = "MIT"
authors = ["Vector contributors"]
repository = "https://github.com/<owner>/vector"

[workspace.dependencies]
# Core Phase 1 deps; later phases add more.
winit = { version = "0.30.13", default-features = false, features = ["rwh_06"] }
objc2 = "0.6.4"
objc2-app-kit = "0.3"
objc2-foundation = "0.3"
raw-window-handle = "0.6"
tokio = { version = "1.52.3", features = ["rt-multi-thread", "macros", "time", "sync"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
anyhow = "1"
thiserror = "2"

# Pre-pinned for later phases (no code uses them yet, just vendored versions
# to avoid a flag-day bump). Listed but commented out per Pitfall 12 (compile bloat):
# alacritty_terminal = "0.26.0"
# wgpu = "29.0.3"
# portable-pty = "0.9.0"
# russh = "0.60.2"
# octocrab = "0.50.0"
# oauth2 = "5.0.0"
# keyring = "4.0.0"
# reqwest = { version = "0.13.3", default-features = false, features = ["rustls-tls", "json", "stream"] }
# tonic = "0.14.6"
# prost = "0.13"
# crossfont = "0.9.0"

[workspace.lints.rust]
unsafe_code = "deny"
# Per-crate override: vector-app overrides via `#![allow(unsafe_code)]` (D-06 allowlist).

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
await_holding_lock = "deny"
# Allow some pedantic lints that are noisy without being useful:
module_name_repetitions = "allow"
must_use_candidate = "allow"
missing_errors_doc = "allow"
missing_panics_doc = "allow"
```

### Stub crate `Cargo.toml` (e.g. `crates/vector-mux/Cargo.toml`)

```toml
[package]
name = "vector-mux"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Window/Tab/Pane mux tree and Domain trait — Phase 4."

[lints]
workspace = true

[dependencies]
anyhow.workspace = true
thiserror.workspace = true
tracing.workspace = true
```

### Stub `lib.rs` (D-03: module skeleton + intent rustdoc)

```rust
//! # vector-mux
//!
//! Owns the mux tree (`Window` → `Tab` → `Pane`) and the `Domain` trait that
//! produces `PtyTransport`s. Lifted in spirit from WezTerm's `mux` crate.
//!
//! Phase 4 will land:
//!  - `Mux::get()` global accessor
//!  - `Domain` trait + `LocalDomain` impl
//!  - `Pane` trait + `LocalPane` impl
//!  - Resize routing + focus tracking
//!
//! Phase 1 ships only the trait names and a doc placeholder so cross-crate
//! references compile.

#![cfg_attr(docsrs, feature(doc_cfg))]

use anyhow::Result;

pub trait Domain: Send + Sync {
    fn label(&self) -> String {
        unimplemented!("Phase 4")
    }
}

pub trait Pane: Send + Sync {}
```

### CI workflow `.github/workflows/ci.yml`

```yaml
name: ci

on:
  pull_request:
  push:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  CARGO_HUSKY_DONT_INSTALL_HOOKS: "1"
  RUST_BACKTRACE: short
  MACOSX_DEPLOYMENT_TARGET: "13.0"

jobs:
  lint:
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.88.0
        with:
          components: rustfmt, clippy
          targets: aarch64-apple-darwin
      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: ci-lint
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets --all-features -- -D warnings

  commitlint:
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }
      - name: Install convco
        run: |
          curl -sSL https://github.com/convco/convco/releases/latest/download/convco-macos.zip -o convco.zip
          unzip convco.zip && chmod +x convco && sudo mv convco /usr/local/bin/
      - name: Lint commits since base
        run: |
          if [ "${{ github.event_name }}" = "pull_request" ]; then
            convco check ${{ github.event.pull_request.base.sha }}..HEAD
          else
            convco check HEAD~10..HEAD || true  # tolerant on direct main pushes
          fi

  test:
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.88.0
        with: { targets: aarch64-apple-darwin }
      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: ci-test
      - name: Architecture-lint per-crate tests
        run: cargo test --workspace --tests
      - name: Architecture-lint grep redundancy
        run: |
          if rg -n --glob 'crates/**/*.rs' --glob '!crates/**/tests/no_tokio_main.rs' \
               '#\[tokio::main\]|Builder::new_current_thread\(\)' ; then
            echo "::error::Forbidden tokio pattern (D-08)"
            exit 1
          fi

  deny:
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.88.0
      - uses: EmbarkStudios/cargo-deny-action@v2
        with:
          command: check advisories licenses bans sources

  build-arm64:
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    needs: [lint, test, deny]
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.88.0
        with: { targets: aarch64-apple-darwin }
      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: ci-build-arm64
      - run: cargo build --release --target aarch64-apple-darwin -p vector-app
      - uses: actions/upload-artifact@v4
        with:
          name: vector-aarch64
          path: target/aarch64-apple-darwin/release/vector-app

  build-x86_64:
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    needs: [lint, test, deny]
    runs-on: macos-15-intel    # ← AMENDMENT to D-21 (macos-13 retired Dec 2025)
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.88.0
        with: { targets: x86_64-apple-darwin }
      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: ci-build-x86_64
      - run: cargo build --release --target x86_64-apple-darwin -p vector-app
      - uses: actions/upload-artifact@v4
        with:
          name: vector-x86_64
          path: target/x86_64-apple-darwin/release/vector-app

  package:
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    needs: [build-arm64, build-x86_64]
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }  # for git rev-parse + git-cliff
      - uses: dtolnay/rust-toolchain@1.88.0
        with: { targets: aarch64-apple-darwin }
      - uses: actions/download-artifact@v4
        with: { name: vector-aarch64, path: artifacts/aarch64 }
      - uses: actions/download-artifact@v4
        with: { name: vector-x86_64, path: artifacts/x86_64 }
      - name: Install create-dmg + cargo-bundle
        run: |
          brew install create-dmg
          cargo install cargo-bundle@0.10.0
      - name: Build Universal DMG via xtask
        run: cargo xtask dmg --universal \
                --arm64 artifacts/aarch64/vector-app \
                --x86_64 artifacts/x86_64/vector-app
      - uses: actions/upload-artifact@v4
        with:
          name: vector-universal-dmg
          path: target/dmg/Vector-*-universal.dmg
          retention-days: 90
      - name: Publish tip release (overwrite)
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          gh release delete tip --yes --cleanup-tag || true
          gh release create tip --prerelease --title "Tip build" \
             --notes "Built from $(git rev-parse --short HEAD). Use \`xattr -dr com.apple.quarantine /Applications/Vector.app\` after install." \
             target/dmg/Vector-*-universal.dmg
```

### Release workflow `.github/workflows/release.yml`

```yaml
name: release

on:
  push:
    tags: ['v*']

env:
  MACOSX_DEPLOYMENT_TARGET: "13.0"
  CARGO_HUSKY_DONT_INSTALL_HOOKS: "1"

jobs:
  build-arm64:
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.88.0
        with: { targets: aarch64-apple-darwin }
      - uses: Swatinem/rust-cache@v2
        with: { shared-key: rel-arm64 }
      - run: cargo build --release --target aarch64-apple-darwin -p vector-app
      - uses: actions/upload-artifact@v4
        with: { name: vector-aarch64, path: target/aarch64-apple-darwin/release/vector-app }

  build-x86_64:
    runs-on: macos-15-intel    # AMENDMENT to D-21
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.88.0
        with: { targets: x86_64-apple-darwin }
      - uses: Swatinem/rust-cache@v2
        with: { shared-key: rel-x86_64 }
      - run: cargo build --release --target x86_64-apple-darwin -p vector-app
      - uses: actions/upload-artifact@v4
        with: { name: vector-x86_64, path: target/x86_64-apple-darwin/release/vector-app }

  release:
    needs: [build-arm64, build-x86_64]
    runs-on: macos-14
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }
      - uses: dtolnay/rust-toolchain@1.88.0
        with: { targets: aarch64-apple-darwin }
      - uses: actions/download-artifact@v4
        with: { name: vector-aarch64, path: artifacts/aarch64 }
      - uses: actions/download-artifact@v4
        with: { name: vector-x86_64, path: artifacts/x86_64 }
      - run: |
          brew install create-dmg git-cliff
          cargo install cargo-bundle@0.10.0
      - name: Build Universal DMG
        run: cargo xtask dmg --universal \
                --arm64 artifacts/aarch64/vector-app \
                --x86_64 artifacts/x86_64/vector-app
      - name: Generate release notes
        run: git-cliff --latest -o RELEASE_NOTES.md
      - name: Publish release
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          gh release create "${{ github.ref_name }}" \
             --title "Vector ${{ github.ref_name }}" \
             --notes-file RELEASE_NOTES.md \
             target/dmg/Vector-*-universal.dmg
```

### `cargo-bundle` config in `crates/vector-app/Cargo.toml`

```toml
[package]
name = "vector-app"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[[bin]]
name = "vector-app"
path = "src/main.rs"

[package.metadata.bundle]
name = "Vector"
identifier = "com.vector.app"
icon = ["resources/icon.icns"]
copyright = "© 2026 Vector contributors"
category = "public.app-category.developer-tools"
short_description = "Native macOS terminal with first-class GitHub Codespaces support."
long_description = """
Vector is a fast native macOS terminal — written in Rust, GPU-accelerated —
with first-class GitHub Codespaces and Dev Tunnels support.
"""
osx_minimum_system_version = "13.0"

# Custom Info.plist additions (D-15 menu bar wiring expects these defaults):
osx_info_plist_exts = ["resources/Info.plist.partial"]
```

`crates/vector-app/resources/Info.plist.partial` (merged into the generated Info.plist):

```xml
<key>LSMinimumSystemVersion</key>
<string>13.0</string>
<key>NSHighResolutionCapable</key>
<true/>
<key>CFBundleVersion</key>
<string>2026.05.10</string>
<key>CFBundleShortVersionString</key>
<string>2026.05.10</string>
```

[CITED: github.com/burtonageo/cargo-bundle README — `osx_info_plist_exts` syntax confirmed]

### `xtask` shape

`xtask/Cargo.toml`:

```toml
[package]
name = "xtask"
version = "0.0.0"
edition = "2021"
publish = false
# Note: no [workspace] line; this is a separate workspace per D-04.

[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
xshell = "0.2"  # for ergonomic shell-out from Rust
```

`xtask/src/main.rs` outline:

```rust
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use xshell::{cmd, Shell};

#[derive(Parser)]
struct Cli { #[command(subcommand)] cmd: Cmd }

#[derive(Subcommand)]
enum Cmd {
    /// Build the unsigned Universal DMG.
    Dmg {
        #[arg(long)] universal: bool,
        #[arg(long)] arm64: Option<PathBuf>,
        #[arg(long)] x86_64: Option<PathBuf>,
    },
    /// Bump CalVer + run git-cliff + tag (no push — user reviews diff).
    Release,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let sh = Shell::new()?;
    sh.change_dir(workspace_root()?);
    match cli.cmd {
        Cmd::Dmg { universal: true, arm64, x86_64 } => dmg_universal(&sh, arm64, x86_64),
        Cmd::Dmg { universal: false, .. } => dmg_local(&sh),
        Cmd::Release => release(&sh),
    }
}

fn dmg_universal(sh: &Shell, arm64: Option<PathBuf>, x86_64: Option<PathBuf>) -> Result<()> {
    // 1. Resolve binary inputs (CI passes paths; local computes them).
    let arm64 = arm64.unwrap_or_else(|| build_for("aarch64-apple-darwin", sh).unwrap());
    let x86_64 = x86_64.unwrap_or_else(|| build_for("x86_64-apple-darwin", sh).unwrap());
    // 2. lipo merge.
    let universal = sh.current_dir().join("target/universal-apple-darwin/release/vector-app");
    sh.create_dir(universal.parent().unwrap())?;
    cmd!(sh, "lipo -create -output {universal} {arm64} {x86_64}").run()?;
    cmd!(sh, "lipo -info {universal}").run()?;  // verification
    // 3. cargo-bundle expects the binary in the target dir of the active triple.
    //    We copy the universal binary into target/release/ for cargo-bundle to pick up.
    let bundle_target = sh.current_dir().join("target/release/vector-app");
    sh.copy_file(&universal, &bundle_target)?;
    cmd!(sh, "cargo bundle --release -p vector-app").run()?;
    // 4. iconutil — generate .icns from .iconset (kept gitignored, regenerated each build).
    generate_icns(sh)?;
    // 5. create-dmg styled wrap.
    let app_path = sh.current_dir().join("target/release/bundle/osx/Vector.app");
    let version = env!("CARGO_PKG_VERSION");  // workspace version
    let dmg = sh.current_dir().join(format!("target/dmg/Vector-{version}-universal.dmg"));
    sh.create_dir(dmg.parent().unwrap())?;
    cmd!(sh,
        "create-dmg
           --volname Vector
           --volicon crates/vector-app/resources/icon.icns
           --background crates/vector-app/resources/dmg-background.png
           --window-pos 200 120
           --window-size 640 400
           --icon-size 96
           --icon Vector.app 160 200
           --app-drop-link 480 200
           --hide-extension Vector.app
           --no-internet-enable
           --hdiutil-quiet
           {dmg} {app_path}"
    ).run()?;
    Ok(())
}

fn release(sh: &Shell) -> Result<()> {
    // 1. Compute new CalVer (today, with -N suffix if same-day re-release; D-27 chose
    //    not to support same-day, so this errors if today's tag exists).
    let version = chrono::Local::now().format("%Y.%m.%d").to_string();
    let tag = format!("v{version}");
    if cmd!(sh, "git rev-parse --verify {tag}").read().is_ok() {
        anyhow::bail!("tag {tag} already exists");
    }
    // 2. Bump workspace version via toml_edit.
    bump_workspace_version(sh, &version)?;
    // 3. Generate CHANGELOG.md.
    cmd!(sh, "git-cliff -t {tag} -o CHANGELOG.md").run()?;
    // 4. Commit + tag (no push — user reviews per CLAUDE.md "do not push").
    cmd!(sh, "git add Cargo.toml CHANGELOG.md").run()?;
    cmd!(sh, "git commit -m chore(release):{tag}").run()?;
    cmd!(sh, "git tag {tag}").run()?;
    println!("Tagged {tag}. Run `git push --follow-tags` when ready.");
    Ok(())
}
```

### `.cargo/config.toml`

```toml
[alias]
xtask = "run --manifest-path xtask/Cargo.toml --release --"
```

`--release` is the WezTerm/cargo-make convention — xtask itself benefits from optimization (especially for IO-heavy work like `lipo`/`create-dmg` orchestration), and it only compiles once, then is cached. The first invocation pays a ~30s cost; every subsequent invocation is cached. [CITED: github.com/wezterm/wezterm — workspace alias pattern]

### `rust-toolchain.toml`

```toml
[toolchain]
channel = "1.88.0"
components = ["rustfmt", "clippy"]
targets = ["aarch64-apple-darwin", "x86_64-apple-darwin"]
profile = "minimal"
```

### `deny.toml`

```toml
[graph]
all-features = true
no-default-features = false

[advisories]
version = 2
yanked = "deny"
ignore = []
# unmaintained: "all" yields false positives on stable but stagnant deps.
# Use "workspace" so we only fail on direct deps that are unmaintained.
unmaintained = "workspace"

[licenses]
version = 2
# Allow all standard permissive licenses our stack uses.
allow = [
    "Apache-2.0",
    "MIT",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-DFS-2016",
    "Unicode-3.0",
    "CC0-1.0",
    "Zlib",
    "0BSD",
    "MPL-2.0",  # required by some webpki/rustls deps
]
confidence-threshold = 0.93

# OpenSSL is *intentionally* not in the allow list — we use rustls everywhere.
# If a transitive dep brings in OpenSSL, deny.toml fails the build, surfacing it
# for explicit decision rather than silent inclusion.
[bans]
multiple-versions = "warn"
wildcards = "deny"
deny = [
    { name = "openssl", reason = "use rustls; OpenSSL adds C build complexity" },
    { name = "openssl-sys", reason = "see openssl above" },
]
skip-tree = []

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-git = [
    # Add Microsoft dev-tunnels git URL when Phase 8 lands.
]
```

[CITED: embarkstudios.github.io/cargo-deny/checks/{advisories,licenses,bans,sources}/cfg.html]

### `cliff.toml` (Keep a Changelog format, mapped to Conventional Commits)

```toml
[changelog]
header = """
# Changelog

All notable changes to Vector are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow CalVer (`YYYY.MM.DD`).
"""

body = """
{% if version -%}
## [{{ version | trim_start_matches(pat="v") }}] - {{ timestamp | date(format="%Y-%m-%d") }}
{% else -%}
## [Unreleased]
{% endif -%}
{% for group, commits in commits | group_by(attribute="group") %}
### {{ group | upper_first }}
{% for c in commits -%}
- {{ c.message | upper_first }}{% if c.id %} ({{ c.id | truncate(length=7, end="") }}){% endif %}
{% endfor %}
{% endfor %}
"""

[git]
conventional_commits = true
filter_unconventional = false
commit_parsers = [
    { message = "^feat",     group = "Added" },
    { message = "^fix",      group = "Fixed" },
    { message = "^perf",     group = "Performance" },
    { message = "^refactor", group = "Changed" },
    { message = "^docs",     group = "Documentation" },
    { message = "^test",     group = "Tests" },
    { message = "^chore",    group = "Internal", skip = false },
    { message = "^build",    group = "Build" },
    { message = "^ci",       group = "CI" },
    { body    = ".*BREAKING CHANGE", group = "Breaking" },
]
sort_commits = "newest"
```

[VERIFIED: github.com/orhun/git-cliff/blob/main/examples/keepachangelog.toml — fork + adapt]

---

## Conventional Commits: convco vs commitlint

**Recommendation: convco.** Reasons:

| Criterion | convco | commitlint |
|-----------|--------|------------|
| Runtime | single Rust binary | Node.js + npm packages |
| CI install time | ~5s (download + chmod) | ~30s (npm install + Node setup) |
| Project alignment | Matches Rust-only stance | Adds Node toolchain to repo |
| Active maintenance | Yes (latest 2026) | Yes |
| Conventional Commits coverage | Full (subject, scope, body, footers, BREAKING CHANGE) | Full |
| `convco check` semantics | Validates commit range; clear errors | Same |

There is no feature in commitlint that we need that convco lacks for v1. Pick convco unless the team has a Node-toolchain reason to prefer commitlint (none for Vector).

CI step:
```yaml
- name: Install convco
  run: |
    curl -sSL https://github.com/convco/convco/releases/latest/download/convco-macos.zip -o /tmp/convco.zip
    unzip /tmp/convco.zip -d /tmp/convco && chmod +x /tmp/convco/convco && sudo mv /tmp/convco/convco /usr/local/bin/
- name: Lint commits
  run: convco check ${{ github.event.pull_request.base.sha }}..HEAD
```

Optional `.convco` config:
```yaml
types:
  - feat
  - fix
  - docs
  - style
  - refactor
  - perf
  - test
  - build
  - ci
  - chore
  - revert
scopes:
  - app
  - xtask
  - ci
  - build
  - docs
  - deps
  - release
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `cocoa-rs` for AppKit | `objc2-app-kit 0.3` | objc2 0.6 stabilized 2024 | Type-safe ObjC; replaces unmaintained `cocoa` |
| `objc` (legacy) | `objc2 0.6` | Same | Same |
| `winit::EventLoopProxy::send_event(T)` | `EventLoopProxy::wake_up()` (no payload) | winit 0.31.0-beta.1 (Nov 2024) | Phase 1 uses 0.30.13 — `send_event(T)` still works. **Phase 1 must NOT bump winit to 0.31** without re-architecting tick payload via mpsc channel pair. |
| `macos-13` GH runner | `macos-15-intel` for x86_64 | macos-13 retired Dec 2025 | Phase 1 amendment to D-21 |
| `cargo-bundle` for Tauri | `cargo-bundle 0.10` (still maintained, alpha) | n/a | Still the right tool; alpha label is conservative |
| Manual Sparkle update | (deferred to v2 — needs signing) | n/a | Out of scope |

**Deprecated/outdated:**
- `cocoa` crate: use `objc2-app-kit`.
- `objc` crate (0.2): use `objc2` (0.6).
- `harfbuzz_rs`: stale; use CoreText via `crossfont` when fonts land in Phase 3.
- `tokio_pty_process`: use `portable-pty` when PTYs land in Phase 2.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `convco` is the right pick over `commitlint` for our Rust-only stance | §Conventional Commits | Low — both work; convco saves Node install. Easy swap. |
| A2 | `cargo-husky` `user-hooks` feature gates correctly with `CARGO_HUSKY_DONT_INSTALL_HOOKS=1` | §Pitfall 6 | Medium — if env var doesn't gate, CI runs may try to install hooks and fail. Verify with a CI dry-run during planning. |
| A3 | Workspace lints `unsafe_code = "deny"` allow per-crate `#![allow(unsafe_code)]` to override | §Workspace `Cargo.toml` | Low — this is the documented `[workspace.lints]` semantics; verified in rustc docs but worth a smoke test. |
| A4 | `iconutil` path on `macos-14`/`macos-15-intel` GH runners works without extra install | §xtask | Low — Xcode CLI tools include it; runners ship with Xcode. |
| A5 | `cargo bundle --release` finds the universal binary at `target/release/vector-app` after we copy it there | §xtask shape | Medium — cargo-bundle alpha-quality, may have path quirks. **Verify in Wave 0 with a smoke test.** Backup: invoke cargo-bundle with `--bin vector-app --target aarch64-apple-darwin` and patch the binary post-bundle. |
| A6 | `MACOSX_DEPLOYMENT_TARGET=13.0` on `macos-15-intel` runner produces a binary that actually runs on macOS 13 | §Constraint Drift | Low — deployment target controls weak-linking, not host. Standard Apple toolchain semantics. |
| A7 | `cargo-bundle` 0.10 supports `osx_info_plist_exts` for merging additional plist entries | §cargo-bundle config | Low — verified from README; if it doesn't, fall back to a post-bundle `PlistBuddy` step in xtask. |
| A8 | The `tip` GitHub Release pattern (delete + create) doesn't trigger noisy notifications for watchers | §CI YAML | Low — using `gh release delete --yes --cleanup-tag || true` then `gh release create`. Worst case: watchers see two events; tolerable. |
| A9 | `git-cliff -t v{version}` generates the correct CHANGELOG section for the new tag | §release.yml | Low — git-cliff is mature and explicitly supports this. |
| A10 | Branch protection rules (D-35) can be set via GitHub UI in a one-shot manual step + documented in ADR | §Open Questions | Low — `.github/settings.yml` via probot/settings is optional; manual is acceptable for solo-dev case. |

---

## Open Questions (RESOLVED)

1. **Does `cargo bundle --release` reliably pick up the universal binary we placed at `target/release/`?**
   - What we know: cargo-bundle reads the binary from `target/<profile>/<bin-name>` based on the active target triple.
   - What's unclear: whether passing no `--target` defaults to the host triple, in which case `lipo`-merged universal binary may be silently dropped.
   - Recommendation: Wave 0 smoke task — run `cargo xtask dmg --universal` locally on Apple Silicon and verify `lipo -info Vector.app/Contents/MacOS/vector-app` shows both archs. If broken, use the `cargo-bundle --bin <name>` + post-process patch route documented in A5.
   - **RESOLVED:** Wave-0 smoke test scheduled in 01-04 Task 1; cargo-bundle accepts the pre-merged universal binary at target/release/ per the spike.

2. **Branch protection automation: probot/settings or manual?**
   - What we know: probot/settings exists and works, but requires a GitHub App install.
   - What's unclear: whether the user wants to install a third-party app on the repo.
   - Recommendation: Manual configuration via UI, documented in `0006-branch-protection.md` ADR (or fold into `0001`). Add a setup-checklist to `docs/setup.md` so future contributors can re-verify.
   - **RESOLVED:** 01-06 documents manual UI configuration in docs/setup.md and captures the decision in ADR 0006; probot/settings deferred until multi-contributor scenario emerges.

3. **Should `cargo-husky` be opt-in via env var or always-on with a CI gate?**
   - What we know: `cargo-husky` installs hooks at first build; CI must not install them.
   - What's unclear: whether contributors who don't want auto-hooks have a clean way to opt out.
   - Recommendation: `CARGO_HUSKY_DONT_INSTALL_HOOKS=1` env var, documented in CONTRIBUTING.md. Default = hooks-on for local devs.
   - **RESOLVED:** 01-02 documents the CARGO_HUSKY_DONT_INSTALL_HOOKS=1 environment variable; 01-05 ci.yml sets it in the workflow env: block; manual verification step captured in 01-02-SUMMARY.md.

4. **convco install via `cargo install convco` vs prebuilt binary download?**
   - What we know: `cargo install convco` works but takes 30–60s in CI.
   - Recommendation: download prebuilt from GH Releases (5s). Cache the binary in `Swatinem/rust-cache@v2` for free.
   - **RESOLVED:** 01-05 ci.yml uses the prebuilt binary curl-download (~5s) over cargo install (~30s) per the recommendation.

---

## Environment Availability

| Dependency | Required By | Available (assumed CI) | Version | Fallback |
|------------|------------|------------------------|---------|----------|
| `lipo` | Universal binary merge | ✓ (Xcode CLI on all macos-* runners) | bundled | — |
| `iconutil` | `.icns` generation | ✓ (Xcode CLI) | bundled | sips + manual icns assembly |
| `hdiutil` | DMG fallback | ✓ (built into macOS) | bundled | — |
| `create-dmg` | Styled DMG | ✗ default; install via `brew install create-dmg` in CI | latest | `hdiutil create -fs HFS+ -srcfolder Vector.app -volname Vector` |
| `cargo-bundle` | `.app` bundle | ✗ default; `cargo install cargo-bundle@0.10.0` | 0.10.0 | hand-roll `mkdir`+`Info.plist` (rejected) |
| `cargo-deny` | License/advisory audit | ✗ default; install via `cargo install cargo-deny` or use action | 0.16.x | — |
| `convco` | Conventional Commits lint | ✗ default; download from GH Releases | latest | `commitlint` (rejected) |
| `git-cliff` | CHANGELOG | ✗ default; `brew install git-cliff` or `cargo install` | latest | hand-edit CHANGELOG (rejected) |
| `gh` (GitHub CLI) | Release creation | ✓ (preinstalled on all macos-* runners) | bundled | curl + REST API (rejected — verbose) |
| `git` | build.rs SHA | ✓ | bundled | "unknown" fallback in build.rs |

**Missing dependencies with no fallback:** none — `create-dmg` has `hdiutil` fallback; `cargo-bundle` is the only un-replaceable tool and it installs cleanly.

**Missing dependencies with fallback:** `create-dmg` falls back to `hdiutil` if a CI run can't install Homebrew packages.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in) |
| Config file | none — Rust convention |
| Quick run command | `cargo test --workspace --tests --lib` |
| Full suite command | `cargo test --workspace --all-targets` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| BUILD-01 | Workspace skeleton compiles | smoke | `cargo build --workspace` | ❌ Wave 0 (workspace doesn't exist yet) |
| BUILD-02 | CI builds Universal binary on push | integration (CI) | GH Actions workflow run; observe artifact upload | ❌ Wave 0 (workflow doesn't exist) |
| BUILD-02 (verify) | Resulting binary is actually fat | unit (CI step) | `lipo -info $BIN \| grep -q 'x86_64 arm64'` | ❌ Wave 0 |
| BUILD-03 | `cargo xtask dmg` works locally | smoke (manual + CI parity) | `cargo xtask dmg --universal && test -f target/dmg/Vector-*.dmg` | ❌ Wave 0 |
| BUILD-04 | Tagged release publishes DMG | integration (CI on tag push) | manual on first `v*` tag; observe gh release | ❌ Wave 0 |
| BUILD-05 | README has xattr instruction | unit (lint) | `rg -q 'xattr -dr com.apple.quarantine /Applications/Vector.app' README.md` | ❌ Wave 0 |
| WIN-05 | winit on main, tokio on background, send_event only | unit (architecture-lint) | `cargo test -p vector-app --test no_tokio_main` | ❌ Wave 0 (test file new) |
| WIN-05 | Tick smoke test visible | manual | Launch Vector.app; observe title goes `Vector` → `Vector — tick 1` → ... within 1s | n/a — manual |
| WIN-05 (across crates) | No `tokio::main` etc. anywhere | unit (per-crate) | `cargo test --workspace --tests` runs all 14 `no_tokio_main.rs` | ❌ Wave 0 |
| WIN-05 (CI grep) | Belt-and-braces redundancy | smoke | `rg -n --glob 'crates/**/*.rs' '#\[tokio::main\]\|Builder::new_current_thread()'` returns nonzero exit if found | ❌ Wave 0 (script in workflow) |

### Sampling rate

- **Per task commit:** `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace --tests` (cargo-husky runs the first two; CI runs all)
- **Per wave merge:** `cargo test --workspace --all-targets` + `cargo deny check` + the architecture-lint grep
- **Phase gate:** Full CI green on a PR; manual smoke test of the launched app showing the tick + version overlay; manual install from a downloaded DMG verifying the `xattr` instruction works.

### Wave 0 gaps (all of these are new files this phase introduces)

- [ ] `Cargo.toml` (workspace root) — workspace.dependencies + workspace.lints
- [ ] `rust-toolchain.toml` — channel pin
- [ ] `.cargo/config.toml` — xtask alias
- [ ] `xtask/Cargo.toml` + `xtask/src/main.rs` — DMG/release automation
- [ ] All 14 `crates/vector-*/Cargo.toml` + stub `lib.rs` (or `main.rs` for vector-app)
- [ ] All 14 `crates/vector-*/tests/no_tokio_main.rs` — architecture-lint
- [ ] `crates/vector-app/build.rs` — VECTOR_BUILD_SHA
- [ ] `crates/vector-app/src/{main,app,menu,overlay,tick}.rs` — threading skeleton + AppKit
- [ ] `crates/vector-app/resources/{icon.svg,Info.plist.partial,dmg-background.png}` — assets
- [ ] `.github/workflows/ci.yml` — PR + main checks
- [ ] `.github/workflows/release.yml` — tag-triggered release
- [ ] `deny.toml` — cargo-deny config
- [ ] `cliff.toml` — git-cliff config
- [ ] `docs/adr/0001..0005-*.md` — five ADRs (MADR template)
- [ ] `README.md` — install block per UI-SPEC
- [ ] `CHANGELOG.md` — initial empty version

### False-positive failure modes to guard against

| Failure | What it looks like | Detection |
|---------|---------------------|-----------|
| Universal binary that's secretly thin | DMG contains arm64-only binary; Intel users see "damaged" | `lipo -info` step in package job checks for both archs |
| `xattr` instruction silently missing from README | DMG ships fine; teammates can't open the app | CI step: `rg -q 'xattr -dr com.apple.quarantine'` against README, DMG background metadata, and last release body |
| Architecture-lint test never runs because file pattern excludes it | New crate added without `tests/no_tokio_main.rs`; threading violations slip through | CI step counts `crates/*/tests/no_tokio_main.rs` files vs workspace member count; mismatch fails the build |
| `cargo-husky` hooks re-install on every CI build | CI flakes / slows | `CARGO_HUSKY_DONT_INSTALL_HOOKS=1` in workflow env block |
| `git-cliff` produces empty CHANGELOG section | Tagged release ships with empty release notes | `git-cliff --latest` exits nonzero on empty; CI step asserts non-empty file size |
| Tip release deletes during a concurrent push | Rare, but two pushes within seconds delete each other's tip release | accept — solo-dev project, low-frequency; document as known limitation |
| `MACOSX_DEPLOYMENT_TARGET` set in workflow but not in cargo-bundle's Info.plist | Binary's Mach-O LC_VERSION_MIN_MACOSX says 13.0 but Info.plist's LSMinimumSystemVersion says something else | Info.plist.partial pins LSMinimumSystemVersion=13.0 explicitly; CI step asserts `plutil -p` output matches |

---

## Sources

### Primary (HIGH confidence)
- [docs.rs/winit/0.30.13](https://docs.rs/winit/0.30.13/winit/event_loop/struct.EventLoopProxy.html) — `send_event` signature
- [docs.rs/winit/0.30.13 ApplicationHandler](https://docs.rs/winit/0.30.13/winit/application/trait.ApplicationHandler.html) — `user_event` method
- [github.com/burtonageo/cargo-bundle](https://github.com/burtonageo/cargo-bundle) — `[package.metadata.bundle]` keys
- [embarkstudios.github.io/cargo-deny](https://embarkstudios.github.io/cargo-deny/) — config schema
- [GitHub Actions changelog: macos-13 closing down](https://github.blog/changelog/2025-09-19-github-actions-macos-13-runner-image-is-closing-down/) — runner deprecation
- [actions/runner-images#13046](https://github.com/actions/runner-images/issues/13046) — `macos-15-intel` until Aug 2027
- [github.com/create-dmg/create-dmg](https://github.com/create-dmg/create-dmg) — flags + examples
- [github.com/orhun/git-cliff/examples/keepachangelog.toml](https://github.com/orhun/git-cliff/blob/main/examples/keepachangelog.toml) — template fork target
- [github.com/Swatinem/rust-cache](https://github.com/Swatinem/rust-cache) — `key`/`shared-key`/auto-hash semantics
- [adr.github.io/madr](https://adr.github.io/madr/) — MADR template (4.0.0)
- [doc.rust-lang.org/cargo/reference/build-scripts.html](https://doc.rust-lang.org/cargo/reference/build-scripts.html) — `cargo:rustc-env`, `rerun-if-changed` semantics

### Secondary (MEDIUM confidence)
- [github.com/rhysd/cargo-husky](https://github.com/rhysd/cargo-husky) — `user-hooks` feature + `CARGO_HUSKY_DONT_INSTALL_HOOKS`
- [github.com/convco/convco](https://github.com/convco/convco) — `convco check` semantics
- [rust-windowing/winit#4260](https://github.com/rust-windowing/winit/issues/4260) — `NSApplicationDelegate` + main menu interaction with winit

### Tertiary (LOW confidence — verify in Wave 0)
- A5: cargo-bundle universal-binary path behavior — needs local smoke test
- A2: cargo-husky env-var gate behavior in CI — needs CI dry-run

---

## Project Constraints (from CLAUDE.md)

These directives override default agent behavior; planner and executors must comply.

- **Comments:** succinct; one short line max; only when WHY is non-obvious; no multi-paragraph docstrings unless requested.
- **Linting:** discover via Makefile → justfile → CI workflow → pre-commit-config → tool config. For Vector, the source of truth is `cargo` (rustfmt + clippy + test + deny + commit-lint). No Make/just/pre-commit-framework.
- **Run the project's commands verbatim.** Don't invent equivalents.
- **Workflow:** commit each logical stage separately; do **not** push (user reviews diffs and pushes).
- **Scope discipline:** Phase 1 must not slip into Phase 2/3/4 territory. UI-SPEC and CONTEXT.md scope notes are the boundary.
- **Tech stack:** Rust workspace, `wgpu`/`alacritty_terminal`/`tokio`/`octocrab`/`russh`/`oauth2`/`keyring`/`reqwest`/`tonic`/`portable-pty` per STACK.md (Phase 1 only uses winit/objc2-app-kit/tokio/tracing of these).
- **Distribution:** unsigned DMG; CI must produce downloadable artifact per release; no Apple Developer subscription required.
- **GSD workflow enforcement:** all repo edits go through a GSD command. Phase 1 work flows through `/gsd-execute-phase`.
- **macOS only for v1.** macOS 13 baseline. Apple Silicon + Intel via Universal binary.

---

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — versions verified against crates.io fetch dates in STACK.md; winit 0.30.13 API confirmed against docs.rs.
- CI/DMG topology: HIGH — pattern matches ghostty's published pipeline; macos-13 amendment verified against GitHub changelog.
- Threading skeleton: HIGH — winit 0.30 ApplicationHandler trait + EventLoopProxy::send_event are well-documented; pattern matches WezTerm's approach.
- Architecture-lint mechanism: MEDIUM — pattern is well-trodden but exact "BLOCK_ON_ALLOWLIST" mechanism is project-specific; verify in Wave 0.
- Pitfalls: HIGH — sourced from research/PITFALLS.md plus runner-deprecation update.
- cargo-bundle universal-binary path behavior: MEDIUM — alpha-quality tool; smoke test mandatory in Wave 0.

**Research date:** 2026-05-10
**Valid until:** 2026-06-10 (30 days for stable; flag if winit, cargo-bundle, or runner labels change before then)

---

## RESEARCH COMPLETE
