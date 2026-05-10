# Phase 1: Foundation & CI/DMG Pipeline - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-10
**Phase:** 1 — Foundation & CI/DMG Pipeline
**Areas discussed:** Workspace scaffolding scope, Threading-rule enforcement, What Vector.app shows in Phase 1, CI artifact strategy, Versioning/CHANGELOG/Commits/Logging, Gating & branch protection

---

## Workspace scaffolding scope

### Day-1 crate thickness

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal (vector-app + xtask) | Just the binary + xtask; add crates as later phases need them. | |
| Thin slice | vector-app + xtask + vector-term + vector-render stubs. | |
| Full ARCHITECTURE.md scaffold | All 14 crates stubbed with empty `lib.rs`. | ✓ |

**User's choice:** Full ARCHITECTURE.md scaffold (14 crates).

### Shared dep management

| Option | Description | Selected |
|--------|-------------|----------|
| Central `[workspace.dependencies]` | Pin every shared crate once; per-crate `dep.workspace = true`. | ✓ |
| `[workspace.dependencies]` + `[workspace.package]` | Same plus shared package metadata. | |
| Per-crate independent deps | Each crate declares its own versions. | |

**User's choice:** Central `[workspace.dependencies]`.

### Stub `lib.rs` contents

| Option | Description | Selected |
|--------|-------------|----------|
| Truly empty `lib.rs` | Just a comment / empty file. | |
| Module skeleton + intent doc | Trait/struct names + rustdoc explaining the crate's purpose. | ✓ |
| Minimal `pub fn ping()` | One trivial fn each crate exports, called in a smoke test. | |

**User's choice:** Module skeleton + intent doc.

### xtask layout

| Option | Description | Selected |
|--------|-------------|----------|
| Same workspace, `members = ["crates/*", "xtask"]` | One unified workspace. | |
| Separate `xtask` workspace via `.cargo/config.toml` alias | Independent Cargo.toml/Cargo.lock; `cargo xtask` alias. | ✓ |

**User's choice:** Separate `xtask` workspace via `.cargo/config.toml` alias.

### Rust toolchain pin

| Option | Description | Selected |
|--------|-------------|----------|
| Exact pin: `channel = "1.88.0"` | Every contributor and CI uses the exact same compiler. | ✓ |
| Minor floor: `channel = "1.88"` | Accepts any 1.88.x. | |
| Stable channel: `channel = "stable"` | Always latest stable. | |

**User's choice:** Exact pin `1.88.0`.

### Workspace lint policy

| Option | Description | Selected |
|--------|-------------|----------|
| `unsafe_code = "deny"` + clippy pedantic warn | High bar from day one. | ✓ |
| Default rustc + `clippy::all` warn | Standard. | |
| Deny warnings in CI only | Local dev unblocked; CI strict. | |

**User's choice:** `unsafe_code = "deny"` + clippy pedantic warn.

### Release-profile tuning

| Option | Description | Selected |
|--------|-------------|----------|
| Defer tuning to Phase 10 | Use Cargo defaults for `[profile.release]`. | ✓ |
| Set sane defaults now | `lto = "thin"`, `codegen-units = 16`, `strip = "debuginfo"`. | |
| Full perf profile now | `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`. | |

**User's choice:** Defer to Phase 10.

---

## Threading-rule enforcement

### Forbidden-pattern enforcement

| Option | Description | Selected |
|--------|-------------|----------|
| CI grep + custom test | Workspace-level test in `tests/architecture.rs`. | ✓ |
| Dylint custom lint | AST-level enforcement via custom dylint rule. | |
| Runtime panic-guard | Wrapper crate; panic on misuse at runtime. | |
| Wrapper crate hides tokio | `vector-rt` re-exports curated subset. | |

**User's choice:** CI grep + custom test. **Final placement (D-08):** moved to per-crate `tests/no_tokio_main.rs` after the Tests question below.

### Tokio runtime ownership

| Option | Description | Selected |
|--------|-------------|----------|
| Dedicated I/O thread owns the multi-thread runtime | Spawned at startup; `rt.block_on(io_main())`. | ✓ |
| Lazy global runtime via `OnceCell` | Any caller can `RT.spawn(...)`. | |
| Current-thread runtime per worker | Each subsystem its own runtime. | |

**User's choice:** Dedicated I/O thread.

### Threading smoke test

| Option | Description | Selected |
|--------|-------------|----------|
| Async tick → EventLoopProxy → window title update | Visible cross-thread proof. | ✓ |
| Headless integration test only | No visible UI behavior. | |
| Both | Visible + headless. | |

**User's choice:** Async tick → window title update.

### Forbid `Mutex` across `.await`

| Option | Description | Selected |
|--------|-------------|----------|
| Yes — `clippy::await_holding_lock = "deny"` | Standard clippy lint, one line. | ✓ |
| Defer to Phase 9 | It's a Phase 9 anti-pattern. | |

**User's choice:** Yes — deny now.

---

## What Vector.app shows in Phase 1

### App surface

| Option | Description | Selected |
|--------|-------------|----------|
| Bare winit black NSWindow | Cheapest; defers all GPU to Phase 3. | |
| wgpu/Metal clearing to black | Proves wgpu init under threading model. | |
| Black window + version overlay (NSTextField) | Bare winit + native AppKit text overlay. | ✓ |

**User's choice:** Black window + `Vector v{version} (build {sha})` overlay.

### Build SHA source

| Option | Description | Selected |
|--------|-------------|----------|
| `build.rs` reads `git rev-parse --short HEAD` | Compile-time env injection. | ✓ |
| `vergen` crate | Generates VERGEN_GIT_SHA, build timestamp, rustc version. | |
| `CARGO_PKG_VERSION` + env var from CI | Mixed env approach. | |

**User's choice:** `build.rs` + `git rev-parse`.

### Window title and size

| Option | Description | Selected |
|--------|-------------|----------|
| `Vector`, 1024×640, centered | Standard terminal default. | ✓ |
| `Vector v0.1.0`, 800×600 | Explicit version in title, smaller. | |
| You decide | Pick during planning. | |

**User's choice:** `Vector`, 1024×640, centered.

### Menu bar in Phase 1

| Option | Description | Selected |
|--------|-------------|----------|
| Bare-minimum (Quit only) | Just `Vector → Quit Vector` (Cmd-Q). | |
| Full standard menus | File / Edit / View / Window / Help all stubbed. | ✓ |
| No menu bar | AppKit auto-generated default. | |

**User's choice:** Full standard menus (stubbed).

### App icon

| Option | Description | Selected |
|--------|-------------|----------|
| Placeholder generated icon | Simple SVG → .icns via iconutil. | |
| No icon (AppKit default) | Skip the icon. | |
| You decide / I'll supply one | Defer concrete artwork. | |
| Free-text | — | ✓ |

**User's choice (free text):** "Start with a cool placeholder with something which shows speed and vector/tensors." → Translated in CONTEXT.md to a placeholder `.icns` evoking speed + vector/tensor motif; concrete artwork TBD during planning.

---

## CI artifact strategy

### Which pushes produce a DMG

| Option | Description | Selected |
|--------|-------------|----------|
| Every push to `main` only | PRs skip DMG build. | ✓ |
| Every push, every branch, every PR | Maximum visibility, costly. | |
| `main` + every PR | Middle ground. | |
| Only tagged releases | Cheapest; no tip pattern. | |

**User's choice:** Every push to `main` only.

### Tagged-release publish

| Option | Description | Selected |
|--------|-------------|----------|
| Auto-publish on tag push | `on: push: tags: ['v*']`, no manual gate. | ✓ |
| Manual approval via Actions environment | Human gate before upload. | |
| Build only — user runs `gh release create` | Maximum manual control. | |

**User's choice:** Auto-publish on tag push.

### Tip DMG storage

| Option | Description | Selected |
|--------|-------------|----------|
| Pinned `tip` GitHub Release | Stable URL, overwritten each push. | |
| Workflow artifact only | No stable URL; 90-day retention. | |
| Both | Belt-and-suspenders. | ✓ |

**User's choice:** Both — workflow artifact AND tip release.

### `cargo deny`

| Option | Description | Selected |
|--------|-------------|----------|
| Enforce now — fail CI on advisory/license | Roadmap explicitly lists Phase 1. | ✓ |
| Run advisory-only (warn, don't fail) | Signal without gating. | |
| Defer to Phase 10 | Hardening owns it. | |

**User's choice:** Enforce now.

### Matrix-build + lipo structure

| Option | Description | Selected |
|--------|-------------|----------|
| Matrix build then merge job | Job A (arm64) + Job B (x86_64) + Job C (lipo+bundle). | ✓ |
| Single `macos-14` cross-compiling both | One job, cross-compile to x86_64. | |
| Single `macos-14` with `cargo-zigbuild` | Zig-based cross-compile. | |

**User's choice:** Matrix build then merge job.

### `cargo xtask dmg` and CI sharing

| Option | Description | Selected |
|--------|-------------|----------|
| CI invokes `cargo xtask dmg` | Same code path locally and in CI. | ✓ |
| Workflow YAML drives `cargo build` + `lipo` directly | Independent paths. | |

**User's choice:** CI invokes `cargo xtask dmg`.

### Cache strategy

| Option | Description | Selected |
|--------|-------------|----------|
| `Swatinem/rust-cache@v2` keyed by Cargo.lock + target triple | Standard rust-cache. | ✓ |
| No caching in Phase 1 | Empty cache every run. | |
| Manual `actions/cache` for registry only | Cache registry, rebuild source. | |

**User's choice:** `Swatinem/rust-cache@v2`.

### macOS deployment target

| Option | Description | Selected |
|--------|-------------|----------|
| `MACOSX_DEPLOYMENT_TARGET=13.0` | Matches PROJECT.md constraint. | ✓ |
| `MACOSX_DEPLOYMENT_TARGET=14.0` | Tighter floor; drops Ventura. | |
| Whatever the runner defaults to | Risks inconsistent floor. | |

**User's choice:** 13.0.

### DMG packaging tooling

| Option | Description | Selected |
|--------|-------------|----------|
| `cargo-bundle` + `hdiutil` only | Lean, plain DMG. | |
| `cargo-bundle` + `create-dmg` (shell script) | Styled DMG (background, drag target). | ✓ |
| `cargo-bundle` only — ZIP the .app | Breaks BUILD-03/04. | |

**User's choice:** `cargo-bundle` + `create-dmg`.

### `xattr` instructions placement

| Option | Description | Selected |
|--------|-------------|----------|
| README only | Minimal. | |
| README + INSTALL.txt inside DMG | Friendlier. | |
| README + DMG background with instructions | Most discoverable. | ✓ |

**User's choice:** README + DMG background image (+ Release body, per CONTEXT.md D-26).

---

## Versioning / CHANGELOG / Commits / Logging

### Versioning convention

| Option | Description | Selected |
|--------|-------------|----------|
| Start at `0.1.0`, manual bumps via xtask | SemVer pre-1.0. | |
| Start at `0.0.1`, auto-bump on main push | Mechanical. | |
| CalVer `YYYY.MM.DD` | Date-based. | ✓ |

**User's choice:** CalVer `YYYY.MM.DD`.

### DMG filename

| Option | Description | Selected |
|--------|-------------|----------|
| `Vector-{version}-universal.dmg` | Clear, sortable. | ✓ |
| `Vector.dmg` | Constant filename. | |
| `Vector-{version}-macOS-universal.dmg` | Explicit platform marker. | |

**User's choice:** `Vector-{version}-universal.dmg`.

### CHANGELOG format

| Option | Description | Selected |
|--------|-------------|----------|
| Keep-a-Changelog, manual | Hand-maintained Unreleased section. | |
| Auto-generated from Conventional Commits | `git-cliff` / `cargo-release`. | ✓ |
| No CHANGELOG in Phase 1 | Defer. | |

**User's choice:** Auto-generated from Conventional Commits.

### Same-day CalVer bump

| Option | Description | Selected |
|--------|-------------|----------|
| `YYYY.MM.DD` only | One per day or it overwrites. | ✓ |
| `YYYY.MM.DD.HHMM` for tip; `YYYY.MM.DD` for tagged | Mixed precision. | |
| `YYYY.MM.DD-N` sequence | Numbered same-day releases. | |

**User's choice:** `YYYY.MM.DD` only.

### Conventional Commits enforcement

| Option | Description | Selected |
|--------|-------------|----------|
| Enforce in CI via commitlint/convco | PR commits must match. | ✓ |
| Don't enforce — best effort | Voluntary. | |
| Enforce only on merge commit / PR title | Lighter ceremony. | |

**User's choice:** Enforce in CI.

### Pre-commit hooks

| Option | Description | Selected |
|--------|-------------|----------|
| `cargo-husky` running fmt + clippy | Rust-native, trivial. | ✓ |
| `pre-commit` framework | Python tool; cross-language. | |
| No pre-commit hooks | Rely on CI only. | |

**User's choice:** `cargo-husky`.

### Logging in Phase 1

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal `tracing-subscriber` + `EnvFilter` | Stdout, `RUST_LOG` controlled. | ✓ |
| Stdout + rotating file log | `tracing-appender`. | |
| Defer all logging | `println!` only. | |

**User's choice:** Minimal `tracing-subscriber` + `EnvFilter`.

### ADR practice

| Option | Description | Selected |
|--------|-------------|----------|
| Lightweight `docs/adr/0001-*.md` per decision | MADR/Nygard template. | ✓ |
| CONTEXT.md is the ADR | No separate ADR system. | |
| Defer ADR to Phase 5+ | Start later. | |

**User's choice:** Lightweight `docs/adr/` from Phase 1.

---

## Gating and branch protection

### Required CI checks for PR merge

| Option | Description | Selected |
|--------|-------------|----------|
| fmt + clippy + test + architecture-lint (+ deny + commitlint) | Core minimum. | ✓ |
| Above + Universal DMG build as required | Strongest, slower PR cycle. | |
| Only fmt + clippy + test | Lightest gate. | |

**User's choice:** Core minimum (CONTEXT.md D-34 expands to include `cargo deny` + `commitlint`).

### `main` branch protection

| Option | Description | Selected |
|--------|-------------|----------|
| Protect: status checks + PR review (0 reviewers) | Gate exists, configurable. | ✓ |
| Protect: status checks only | No review requirement. | |
| No branch protection | Direct pushes allowed. | |

**User's choice:** Status checks + PR review (0 reviewers).

### Architecture-lint test layout

| Option | Description | Selected |
|--------|-------------|----------|
| Workspace-root `tests/architecture.rs` | Single integration test crate. | |
| Per-crate `tests/no_tokio_main.rs` | Distributed; per-crate allowlist. | ✓ |
| Standalone `xtask` subcommand | `cargo xtask check-architecture`. | |

**User's choice:** Per-crate `tests/no_tokio_main.rs`.

---

## Claude's Discretion

- Exact placeholder `.icns` artwork (within speed + vector/tensor motif).
- ADR file numbering and titles.
- `tests/no_tokio_main.rs` per-crate allowlist comment.
- `create-dmg` background image visual design.
- Conventional Commits scope vocabulary (`build`, `ci`, `app`, `xtask`, `docs`, …).
- `git-cliff` template for CHANGELOG output.

## Deferred Ideas

- wgpu/Metal surface init — Phase 3.
- Release-profile tuning — Phase 10.
- File-rotated logging — later phase (likely 6+).
- Signing / notarization / Sparkle — v2 (out-of-scope per PROJECT.md).
- Same-day release sequence (`YYYY.MM.DD-N`) — declined.
- `vergen` crate — declined.
- `cargo-zigbuild` cross-compile — fallback only if `macos-13` retires.
