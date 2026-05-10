# Phase 1: Foundation & CI/DMG Pipeline - Context

**Gathered:** 2026-05-10
**Status:** Ready for planning

<domain>
## Phase Boundary

A black `Vector.app` opens from a CI-produced unsigned Universal `.dmg`, with the `winit`/`tokio` main-thread ownership pattern locked in from day one and structurally enforced so it cannot regress in later phases.

**In scope:** Cargo workspace skeleton (all 14 crates from ARCHITECTURE.md stubbed), `xtask` build automation, GitHub Actions matrix CI producing Universal DMG on `main` pushes and tagged releases, threading skeleton + architecture lint, native AppKit window with menus and a version overlay, DMG packaging with `create-dmg` styling, ADR practice, Conventional Commits + auto-CHANGELOG, branch protection, `cargo deny`.

**Out of scope:** Anything terminal-rendering related (Phase 2/3), wgpu/Metal surface init (Phase 3), tabs/splits (Phase 4), signing/notarization/Sparkle (v2), full release-profile perf tuning (Phase 10), file/network logging (later phase as needed).

</domain>

<decisions>
## Implementation Decisions

### Workspace scaffolding scope
- **D-01:** Day-1 skeleton stubs **all 14 crates** from ARCHITECTURE.md (`vector-app`, `vector-ui`, `vector-render`, `vector-mux`, `vector-term`, `vector-pty`, `vector-ssh`, `vector-codespaces`, `vector-tunnels`, `vector-config`, `vector-secrets`, `vector-fonts`, `vector-input`, `vector-theme`). Later phases fill in code, not structure.
- **D-02:** Shared dependencies pinned **once** at the workspace root in `[workspace.dependencies]`; every crate references `dep.workspace = true`. Single source of truth for crate versions; protects against the russh-style dual-version split described in STACK.md Section 7.
- **D-03:** Each stub `lib.rs` ships a **module skeleton + intent rustdoc** — declares the trait/struct names the crate is intended to expose (e.g. `vector-mux` exports `pub trait Domain { … }` with `unimplemented!()`) plus a short doc block explaining what the crate owns. The architecture boundary is asserted now, not later.
- **D-04:** `xtask` lives in a **separate workspace** (its own `Cargo.toml` + `Cargo.lock`), invoked via a `.cargo/config.toml` alias (`cargo xtask` → `cargo run --manifest-path xtask/Cargo.toml --`). Keeps the main workspace's incremental builds fast and avoids xtask deps polluting the main resolver graph.
- **D-05:** `rust-toolchain.toml` uses an **exact pin**: `channel = "1.88.0"`. Reproducible builds locally and on CI. `targets = ["aarch64-apple-darwin", "x86_64-apple-darwin"]`. Bumps are intentional commits.
- **D-06:** Workspace-level lints in `[workspace.lints]`:
  - `rust.unsafe_code = "deny"` for all crates except an allowlist (`vector-app` and any future AppKit-touching crate explicitly opts in with `#![allow(unsafe_code)]`).
  - `clippy.pedantic = "warn"` workspace-wide.
  - `clippy.await_holding_lock = "deny"` (forbid `Mutex` held across `.await` — Architecture Anti-Pattern 5 from PITFALLS.md surfaces in Phase 9 reconnect path; gate it now).
- **D-07:** **Defer release-profile tuning to Phase 10.** Use Cargo defaults for `[profile.release]` in Phase 1. Phase 10's perf gate decides final values (`lto`, `codegen-units`, `strip`, `panic`).

### Threading-rule enforcement (the load-bearing decision)
- **D-08:** Forbidden patterns enforced by **per-crate `tests/no_tokio_main.rs` integration tests** plus a CI grep. Each crate carries its own test that fails on `#[tokio::main]`, `Runtime::block_on(` outside an allowlist module, or `Builder::new_current_thread()` in production code. Distributing the test per-crate lets `xtask` legitimately use `block_on` without a global allowlist.
- **D-09:** The tokio runtime lives on a **dedicated I/O thread** spawned by `vector-app::main` before `EventLoop::run`. That thread builds `tokio::runtime::Builder::new_multi_thread().enable_all().build()`, then `rt.block_on(io_main(...))`. Main owns winit; the I/O thread owns the runtime. No `OnceCell<Runtime>` global, no per-subsystem current-thread runtimes.
- **D-10:** Phase 1 includes a **visible threading smoke test**: an async tokio task fires every 500ms, sends a `UserEvent::Tick(n)` via `EventLoopProxy::send_event`, and the main thread updates the window title to `Vector — tick {n}`. Proof of life that cross-thread signaling works under real AppKit + winit + tokio.
- **D-11:** `clippy::await_holding_lock = "deny"` is added at workspace lint level in Phase 1 (already in D-06). This is the structural prevention for Pitfall 5 / Anti-Pattern 5 mentioned in Phase 9's risk notes.

### What Vector.app shows in Phase 1
- **D-12:** The launched app is a **bare winit NSWindow with a `Vector v{version} (build {sha})` text overlay** rendered via native AppKit `NSTextField`. No wgpu, no Metal, no terminal grid. Defers all GPU/renderer work to Phase 3; sanity-proves the running binary matches the CI output.
- **D-13:** Build SHA is embedded at **compile time via `build.rs`** invoking `git rev-parse --short HEAD` and emitting `cargo:rustc-env=VECTOR_BUILD_SHA={short}`. Read in code via `env!("VECTOR_BUILD_SHA")`. No `vergen` crate dep needed for Phase 1.
- **D-14:** Initial window: **title `Vector`, size `1024 × 640`, centered on screen.** The threading smoke test mutates the title to `Vector — tick {n}` after startup.
- **D-15:** **Full standard menu bar** wired up in Phase 1: `Vector / File / Edit / View / Window / Help`. Items are mostly stubbed/disabled; `Vector → Quit` (Cmd-Q), `Window → Minimize`/`Zoom`/`Close` (Cmd-M / Cmd-W) are functional. Locks in the menu structure so later phases just fill in menu actions.
- **D-16:** App icon is a **placeholder `.icns` designed to evoke speed + vector/tensor motif** (creative direction; concrete art TBD during planning). The icon source lives in `crates/vector-app/resources/icon.svg` (or PNG layers), and `xtask` generates the `.icns` via `iconutil` from a tiled iconset directory. Replaceable in a single commit later.

### CI artifact strategy
- **D-17:** **DMG built on every push to `main` only.** PRs run lint/test/architecture-lint but skip the Universal build to conserve runner minutes. Tagged releases (`v*`) trigger the full publish path.
- **D-18:** **Tagged releases auto-publish** to GitHub Releases via `on: push: tags: ['v*']`. No manual approval gate (solo dev / small-trusted-audience project). Release body is populated from the auto-generated CHANGELOG section for that version.
- **D-19:** **Tip DMG goes to BOTH** a workflow artifact (90-day retention on the Actions run) AND a pinned non-versioned `tip` GitHub Release that's overwritten on every `main` push. Stable URL teammates can bookmark; debuggable artifact if Release upload ever fails.
- **D-20:** **`cargo deny` is enforced in CI from Phase 1** (`cargo deny check advisories licenses bans sources`). Roadmap explicitly lists `cargo-deny` in Phase 1 stack additions. Allowlist evolves over time.
- **D-21:** CI structure is a **matrix-then-merge** topology:
  - Job `build-arm64` on `macos-14`: `cargo build --release --target aarch64-apple-darwin`, uploads binary.
  - Job `build-x86_64` on `macos-13`: `cargo build --release --target x86_64-apple-darwin`, uploads binary.
  - Job `package`: depends on both, downloads both binaries, runs `cargo xtask dmg --universal`.
  - (We must validate end-to-end that `macos-13` x86_64 runners still exist; if GitHub retires them before Phase 1 ships, fall back to `cargo-zigbuild` on `macos-14`.)
- **D-22:** **CI invokes `cargo xtask dmg`** for the packaging step. Identical code path locally and in CI; satisfies BUILD-03 ("running `cargo xtask dmg` locally produces an identical DMG").
- **D-23:** Caching via **`Swatinem/rust-cache@v2`** keyed on `Cargo.lock + target triple + rust-toolchain.toml`. Hits across PRs and main pushes; invalidated cleanly on toolchain or dep bumps.
- **D-24:** **Deployment target `MACOSX_DEPLOYMENT_TARGET=13.0`** set in both CI workflow env and xtask wrapper. Info.plist `LSMinimumSystemVersion = 13.0`. Matches PROJECT.md constraint; same floor for both arm64 and x86_64 binaries.
- **D-25:** DMG packaging uses **`cargo-bundle` + `create-dmg` (shell script)**. `cargo-bundle` produces `Vector.app`; `create-dmg` wraps it in a styled DMG with a background image (with `xattr -dr com.apple.quarantine /Applications/Vector.app` instructions rendered into the background), drag-to-Applications target, and custom icon positions.
- **D-26:** `xattr` Gatekeeper-bypass instructions ship in **THREE places**: GitHub README, the DMG background image, and the GitHub Release body. Maximum discoverability for teammates who land on any one surface first.

### Versioning, CHANGELOG, commits, logging
- **D-27:** **CalVer** versioning: `YYYY.MM.DD`. Tagged release on `2026-05-10` is `v2026.05.10`. One release per day; same-day re-releases would overwrite (acceptable for this stage). Workspace `[workspace.package].version` updated by `cargo xtask release`.
- **D-28:** DMG filename: **`Vector-{version}-universal.dmg`**. Tip builds use `Vector-{version}-tip-{shortsha}-universal.dmg` so a downloaded tip never collides with a tagged release.
- **D-29:** **CHANGELOG.md auto-generated from Conventional Commits** via `git-cliff`. Format follows "Keep a Changelog" style sections (Added / Changed / Fixed / etc.) mapped from commit types (`feat:` → Added, `fix:` → Fixed, etc.). `cargo xtask release` runs `git-cliff` to update CHANGELOG.md before tagging.
- **D-30:** **Conventional Commits enforced in CI** via `commitlint` or `convco`. PR-level check fails if any commit doesn't match `<type>(<scope>): <subject>`. Required because auto-CHANGELOG only works if commits are conformant.
- **D-31:** **Local pre-commit hooks via `cargo-husky`**: install on first `cargo build` and run `cargo fmt --check && cargo clippy --all-targets -- -D warnings`. Catches violations before push.
- **D-32:** **Logging via minimal `tracing-subscriber` with `EnvFilter`**: initialized in `vector-app::main`, default level INFO, override via `RUST_LOG`. Stdout only in Phase 1. File logging deferred to whenever Codespaces remote diagnostics demand it (Phase 6+).
- **D-33:** **ADR practice begins in Phase 1.** `docs/adr/0001-*.md` … one per major architectural decision (MADR template). Phase 1 produces at least: `0001-rust-workspace-layout.md`, `0002-winit-tokio-threading.md`, `0003-architecture-lint-mechanism.md`, `0004-dmg-pipeline.md`, `0005-versioning-calver.md`.

### Gating and protection
- **D-34:** **PR required status checks**: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (which includes the per-crate architecture-lint tests), `cargo deny check`, `commitlint`. Universal DMG build runs but is **not** a required check (too slow for PR feedback loop).
- **D-35:** **`main` is branch-protected**: linear history required, required status checks (D-34) must pass, PR review required with `0` reviewers configured (so solo work isn't blocked, but the gate exists to flip on as teammates join). Force-push disabled.

### Claude's Discretion
- Exact placeholder icon artwork (within the speed + vector/tensor motif user described).
- ADR file numbering and titles (will follow MADR template; titles will be descriptive).
- The precise list of files in the `tests/no_tokio_main.rs` allowlist comment.
- `create-dmg` background image visual design (must surface the `xattr` command).
- Conventional Commits scope vocabulary (the list of `(scope)` values — likely `build`, `ci`, `app`, `xtask`, `docs`, etc.).
- The `git-cliff` template for CHANGELOG.md output.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-level (always required)
- `.planning/PROJECT.md` — Project mission, requirements, constraints, scope decisions.
- `.planning/REQUIREMENTS.md` — Full requirement IDs (BUILD-01..05, WIN-05 belong to this phase).
- `.planning/ROADMAP.md` §"Phase 1: Foundation & CI/DMG Pipeline" — Phase goal, success criteria, risks.
- `.planning/STATE.md` — Current workflow position.

### Stack + architecture research (Phase 1 critical)
- `.planning/research/STACK.md` — Locked dep versions: Rust 1.88+, `cargo-bundle 0.10`, `tokio 1.52.3`, `winit 0.30.13`, `objc2 0.6.4`, GitHub Actions matrix (`macos-14` arm64 + `macos-13` x86_64), `cargo-deny`, `lipo` workflow, deployment target.
- `.planning/research/ARCHITECTURE.md` §"Recommended Project Structure" — The 14-crate layout that Phase 1 must scaffold. §"Triple-loop threading (UI / render / I/O)" — The threading invariant being enforced.
- `.planning/research/PITFALLS.md` §"Pitfall 1" (don't roll your own VT parser — relevant to crate-stub doc), §"Pitfall 5" (winit/tokio ownership — THE Phase 1 pitfall, governs D-08..D-11), §"Pitfall 18" (over-scoping at v1 — guards the scope decisions here).
- `.planning/research/FEATURES.md` §"Build & Distribution" — DMG/universal/release conventions cross-checked against ghostty/Alacritty/WezTerm.
- `.planning/research/SUMMARY.md` — Top-line synthesis; useful for the planner to read once before decomposing tasks.

### Reference implementations (read-out-of-tree, do not vendor)
- ghostty release pipeline — model for "tip + tagged" DMG releases on GitHub Releases.
- Alacritty `Cargo.toml` workspace — model for `[workspace.dependencies]` + `[workspace.lints]` layout.
- WezTerm workspace — model for `xtask` separate-workspace pattern and per-crate test layout.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **Repo is greenfield** — no Rust code yet. No reusable assets in-tree. All scaffolding is new.
- `LICENSE` (MIT or similar; already in place) — kept as-is; referenced in `[workspace.package].license`.
- `README.md` (single line currently) — to be expanded in Phase 1 to include install + `xattr` instructions.
- `CLAUDE.md` (project + global instructions) — already documents constraints; downstream agents inherit these.
- `.planning/` directory — managed by GSD; no code-side coupling needed.

### Established Patterns
- **GSD workflow** is the orchestrator: phases live in `.planning/phases/`, decisions in CONTEXT.md, plans in PLAN.md. Phase 1 should not invent parallel doc-management systems.
- **Linting/formatting discovery order** (from global `CLAUDE.md`): Makefile → justfile → `.github/workflows/*.yml` → `pre-commit-config.yaml` / `pyproject.toml` / `package.json`. Phase 1 establishes the source of truth here — `cargo` toolchain. No Make/just/pre-commit-framework (we use `cargo-husky` instead).

### Integration Points
- **None to integrate with** — Phase 1 *is* the integration point all later phases plug into. The decisions here (workspace layout, threading invariant, lint policy, CI structure, ADR practice) are inherited by Phases 2–10.
- Phase 2 inherits: `vector-term` crate stub (will gain `alacritty_terminal` integration), `vector-pty` stub (will gain `portable-pty`), the per-crate architecture-lint test pattern, `tokio` runtime ownership rule.
- Phase 3 inherits: `vector-render` stub (will gain `wgpu`), `winit` event loop already wired and verified under load, `EventLoopProxy::send_event` as the only main-thread wakeup primitive.
- Phase 6 inherits: `vector-secrets` stub (gains `keyring 4.0`), `vector-codespaces` stub (gains `oauth2` + `octocrab`).

</code_context>

<specifics>
## Specific Ideas

- **Icon motif:** "Cool placeholder showing speed and vector/tensor." User direction. Planner free to design concrete art within that motif; final artwork swappable in a single commit.
- **ghostty's "tip + tagged" release pattern** is the explicit reference for the DMG distribution model (D-17..D-19).
- **WezTerm's per-crate testing discipline** is the reference for the `tests/no_tokio_main.rs` layout (D-08).
- **`xattr -dr com.apple.quarantine /Applications/Vector.app`** is the exact incantation that must appear in the README, the DMG background, and the GitHub Release body (D-26).

</specifics>

<deferred>
## Deferred Ideas

- **wgpu/Metal surface init** — explicitly deferred to Phase 3 by user choice (D-12). Phase 1 stays bare-AppKit.
- **Release-profile tuning (lto/codegen-units/strip/panic)** — deferred to Phase 10 hardening (D-07).
- **File-rotated logging under `~/Library/Logs/Vector/`** — deferred until remote-diagnostics demand it (D-32).
- **Apple Developer signing + notarization + Sparkle auto-update** — already out-of-scope in PROJECT.md; reaffirmed here as deferred to v2 (DIST-V2-01, DIST-V2-02).
- **Same-day release sequence (`YYYY.MM.DD-N`)** — explicitly chosen not to implement (D-27); one release per day is acceptable.
- **Full vergen crate** — declined in favor of `build.rs` + `git rev-parse` (D-13). Reconsider if rustc semver / build timestamp / dirty-tree detection becomes necessary.
- **`cargo-zigbuild` cross-compilation** — fallback only, if GitHub retires `macos-13` x86_64 runners before Phase 1 ships (D-21 note).

</deferred>

---

*Phase: 1-Foundation & CI/DMG Pipeline*
*Context gathered: 2026-05-10*
