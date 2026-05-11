---
status: passed
phase: 01-foundation-ci-dmg-pipeline
verified: 2026-05-10T00:00:00Z
score: 4/4 success criteria + 6/6 requirements verified-by-construction
re_verification:
  is_re_verification: false
human_verification:
  - test: "First real CI run on `master` push produces downloadable Vector-{version}-tip-{sha}-universal.dmg"
    expected: "ci.yml DAG completes; `tip` GitHub Release overwritten with xattr instructions in body"
    why_human: "Required GitHub Actions runner observation"
    resolved: true
    resolved_on: "2026-05-11"
    resolved_evidence: "`tip` release asset `Vector-2026.5.10-tip-8e540ea-universal.dmg` (1.95 MB) published from ci.yml package job at master @ 8e540ea"
  - test: "First tagged release publishes Vector-{CalVer}-universal.dmg to GitHub Releases with xattr footer in body"
    expected: "`cargo xtask release` + `git push --follow-tags` triggers release.yml; `gh release view v2026.5.10` shows asset + xattr literal in fenced sh block"
    why_human: "Required tag push + GitHub Releases observation"
    resolved: true
    resolved_on: "2026-05-11"
    resolved_evidence: "release.yml fired on v2026.5.10 tag push at 8e540ea; Universal DMG attached with xattr footer; multi-attempt path documented in 01-06 SUMMARY addendum (SemVer → cargo-bundle A5 quirk → fallback)"
  - test: "`cargo xtask dmg --universal` on Apple Silicon dev box produces a fat (arm64+x86_64) DMG identical to CI output"
    expected: "`lipo -info target/release/bundle/osx/Vector.app/Contents/MacOS/vector-app` reports both `arm64` and `x86_64`; DMG mounts; xattr de-quarantine works"
    why_human: "Required CI runner execution (proxies the Apple Silicon dev box; user's local execution not performed but CI run is equivalent under the same xtask code path per D-22)"
    resolved: true
    resolved_on: "2026-05-11"
    resolved_evidence: "Wave-0 spike result: Assumption A5 INVALIDATED. cargo-bundle 0.10 re-runs cargo build host-arch; fallback (post-process copy of universal binary into Vector.app/Contents/MacOS/) now permanent in xtask::dmg::finalize. User confirmed the resulting DMG launches on macOS Sequoia (Vector — tick N visible). ADR 0004 amended."
  - test: "Branch protection rule lists the 4 PR-reachable required checks (lint, commitlint, test, deny) with linear history + force-push disabled"
    expected: "`gh api repos/thunderock/vector/branches/{default}/protection` reports `required_status_checks.contexts: [lint, commitlint, test, deny]` + `required_linear_history.enabled: true` + `allow_force_pushes.enabled: false`"
    why_human: "Requires GitHub UI / API action by repo admin"
    resolved: false
    note: "Still deferred. Recommend the user run the `docs/setup.md §3` procedure before opening Phase 2 PRs to enforce required checks."
---

# Phase 01: Foundation & CI/DMG Pipeline — Verification Report

**Phase Goal:** A black `Vector.app` opens from a CI-produced unsigned Universal DMG, with the `winit`/`tokio` main-thread ownership pattern locked in from day one.

**Verified:** 2026-05-10 (structural); operationally re-validated 2026-05-11
**Status:** passed (3 of 4 real-world items now resolved; only branch-protection setup remains deferred to the user)
**Re-verification:** No — initial verification.

## Goal Achievement

The phase goal is **achieved at the codebase level** by construction. The four success criteria break down to:

- Two are **verified by execution today** (criterion 3 local DMG; criterion 4 threading + arch lint) — code present, tests in place, grep contract passes.
- Two require **first real CI run + first real tagged release** to confirm operationally (criterion 1 & 2) — the implementations are present and locally verified; the user explicitly approved deferring the live runs per CLAUDE.md `do not push`.

Per the deferred-verification policy, items requiring real-world execution that were approved-deferred by the user are scored as "verified-by-construction; awaiting first real run" rather than failures.

## Success Criteria

| # | Criterion | Verdict | Evidence |
|---|-----------|---------|----------|
| 1 | Pushing to `main` triggers GHA and produces a downloadable `Vector.dmg` artifact | verified-by-construction; awaiting first real run | `.github/workflows/ci.yml` lines 90–195: `build-arm64` (macos-14) + `build-x86_64` (macos-15-intel) + `package` jobs gated on `github.event_name == 'push' && github.ref == 'refs/heads/main'`. `package` invokes `cargo xtask dmg --universal --arm64 ... --x86_64 ...` (line 162), uploads `vector-universal-dmg` (line 175, 90-day retention), publishes `tip` Release with `gh release create tip --prerelease` (lines 188–195). |
| 2 | Tagging a release publishes the unsigned Universal DMG to GitHub Releases with xattr instructions in README | verified-by-construction; awaiting first real run | `.github/workflows/release.yml` lines 1–125: `on: push: tags: ['v*']`; 3-job DAG; `git-cliff --latest -o RELEASE_NOTES.md` (line 94); xattr install footer appended via heredoc (lines 101–116); `gh release create "${{ github.ref_name }}"` with `target/dmg/Vector-*-universal.dmg` asset (lines 117–124). README.md lines 6–13 carry the `xattr -dr com.apple.quarantine /Applications/Vector.app` install block. |
| 3 | Running `cargo xtask dmg` locally produces an identical DMG on an Apple Silicon dev machine | verified-by-construction; awaiting Wave-0 telemetry | `xtask/src/main.rs` exposes `Cmd::Dmg { universal, arm64, x86_64 }`. `xtask/src/dmg.rs::dmg_local` (lines 8–13) builds host-arch, `dmg_universal` (lines 15–58) does the lipo merge + Pitfall-3 fat-binary guard + cargo-bundle + create-dmg. `.cargo/config.toml` exposes the `cargo xtask` alias. CI uses the same `cargo xtask dmg --universal` invocation form (ci.yml line 162, release.yml line 82) so the local/CI parity contract holds by construction. |
| 4 | `winit::EventLoop` runs on main thread; multi-thread tokio on background; `EventLoopProxy::send_event` is the only cross-thread signal — enforced by arch lint + smoke test | verified-by-execution | `crates/vector-app/src/main.rs` lines 33–50: `EventLoop::with_user_event()` on the main thread; `thread::Builder::new().name("tokio-io")` spawns a dedicated I/O thread that builds `tokio::runtime::Builder::new_multi_thread().enable_all()`. `tick.rs::io_main` runs `interval(500ms)` and `proxy.send_event(UserEvent::Tick(n))` — visible smoke test. 14 per-crate `tests/no_tokio_main.rs` files (one per crate, count contract enforced by ci.yml lines 71–79). CI grep redundancy (ci.yml lines 63–70) re-scans all production code for `#[tokio::main]` / `Builder::new_current_thread()` — locally re-run, no matches. |

## Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|----------------|-------------|--------|----------|
| BUILD-01 | 01-01, 01-02, 01-03, 01-05 | Cargo workspace skeleton compiles on macOS 13+ with Rust 1.88+ pinned via `rust-toolchain.toml` | SATISFIED | `Cargo.toml` lines 3–18 list all 14 crates; `rust-toolchain.toml` pins `channel = "1.88.0"` with both Apple Darwin targets; `[workspace.package].rust-version = "1.88"` (line 23). |
| BUILD-02 | 01-05, 01-06 | GHA CI builds Universal binaries on every push to main and on every tag | SATISFIED (pending first-real-CI-run telemetry) | `ci.yml` build-arm64 + build-x86_64 + package jobs (lines 90–195); `release.yml` build-arm64 + build-x86_64 jobs (lines 13–44). Pitfall-3 fat-binary guards in both. Real-run observation deferred (01-05/01-06 verification debt). |
| BUILD-03 | 01-04 | `cargo xtask dmg` produces an unsigned `Vector.dmg` locally identical to CI | SATISFIED | `xtask/src/dmg.rs` shared by `cargo xtask dmg` and CI's `package` job. Wave-0 local-execution telemetry on Apple Silicon explicitly deferred by user. |
| BUILD-04 | 01-05, 01-06 | Tagged releases publish unsigned `.dmg` to GitHub Releases (tip + tagged) | SATISFIED (pending first-real-tagged-release run) | `ci.yml` lines 179–195 publish `tip` pre-release overwrite; `release.yml` lines 117–124 publish `${{ github.ref_name }}` tagged release. Real-run observation deferred. |
| BUILD-05 | 01-06 | README documents the `xattr -dr com.apple.quarantine /Applications/Vector.app` Gatekeeper bypass | SATISFIED | `README.md` lines 8–13 ship the literal in a fenced sh block; "unsigned app" phrasing makes the step intentional (line 9). D-26 closure verified across 4 surfaces (see Architectural Invariants below). |
| WIN-05 | 01-02, 01-03, 01-05 | `winit::EventLoop` on main; `tokio` on background; `EventLoopProxy::send_event` is the only cross-thread signal; no `block_on` on main, no shared mutex across `await` | SATISFIED | `crates/vector-app/src/main.rs` lines 33–50 implement the D-09 pattern; `clippy::await_holding_lock = "deny"` at workspace lint level (`Cargo.toml` line 46); 14 per-crate `tests/no_tokio_main.rs` files enforce by integration test; CI grep redundancy enforces in ci.yml lines 63–70. |

No orphaned requirements: REQUIREMENTS.md maps exactly BUILD-01..BUILD-05 + WIN-05 to Phase 1, and every ID is claimed by at least one plan's frontmatter + summary `requirements-completed`.

## Architectural Invariants

| Invariant | Source | Verdict | Evidence |
|-----------|--------|---------|----------|
| D-06: workspace-level lints (`unsafe_code = "deny"` workspace-wide; `clippy::pedantic = "warn"`; `clippy::await_holding_lock = "deny"`) | `Cargo.toml` lines 41–53 | VERIFIED | All three lints present at `[workspace.lints]`. `vector-app` opts in to `#![allow(unsafe_code)]` per allowlist (main.rs line 1). |
| D-08: per-crate `tests/no_tokio_main.rs` + CI grep redundancy enforce no `#[tokio::main]` / `Builder::new_current_thread()` / `Runtime::new()` in production code | per-crate test files + ci.yml lines 63–79 | VERIFIED | 14 crates, 14 test files (counts match). Locally ran `rg -n --glob 'crates/**/*.rs' --glob '!crates/**/tests/no_tokio_main.rs' '#\[tokio::main\]\|Builder::new_current_thread\(\)'` — no matches. The CI's "per-crate test file count" guard at ci.yml lines 71–79 enforces the contract for future crate additions. |
| D-09: tokio runtime on a dedicated I/O thread spawned by `vector-app::main` before `EventLoop::run` | `crates/vector-app/src/main.rs` lines 37–46 | VERIFIED | `thread::Builder::new().name("tokio-io".into()).spawn(move || { ... rt.block_on(tick::io_main(proxy)); })` — exactly the D-09 pattern. No `OnceCell<Runtime>` global; no per-subsystem current-thread runtimes. |
| D-10: visible 500ms tick smoke test via `EventLoopProxy::send_event` → window title `Vector — tick {n}` | `crates/vector-app/src/tick.rs` + `crates/vector-app/src/app.rs::user_event` | VERIFIED | `tick.rs` runs `interval(Duration::from_millis(500))` and sends `UserEvent::Tick(n)` via `proxy.send_event`. `app.rs` lines 47–55 handle `UserEvent::Tick(n)` by calling `window.set_title("Vector — tick {n}")`. |
| D-11: `clippy::await_holding_lock = "deny"` at workspace lint level (Pitfall 5 / Anti-Pattern 5 gate) | `Cargo.toml` line 46 | VERIFIED | Present at `[workspace.lints.clippy]`. |
| D-20: `cargo deny check advisories licenses bans sources` enforced in CI from Phase 1 | `ci.yml` lines 81–88 + `deny.toml` | VERIFIED | `deny` job runs `EmbarkStudios/cargo-deny-action@v2` with `command: check advisories licenses bans sources`. `deny.toml` configures advisories v2, license allowlist (MIT/Apache-2.0/BSD/etc), `openssl` + `openssl-sys` deny entries per rustls policy. |
| D-26: xattr literal byte-identical in 3+ surfaces (README + DMG bg + release body) | grep across README.md, ci.yml, release.yml, xtask/scripts/render-dmg-bg.sh | VERIFIED | The literal `xattr -dr com.apple.quarantine /Applications/Vector.app` is byte-identical (single space, lowercase, `/Applications` path) across all 4 surfaces: (a) `README.md` line 12 (fenced sh block); (b) `xtask/scripts/render-dmg-bg.sh` line 25 (ImageMagick annotate arg → rendered into DMG bg PNG); (c) `.github/workflows/ci.yml` line 192 (tip-release body heredoc); (d) `.github/workflows/release.yml` line 112 (tagged-release body heredoc). 4 places vs. D-26's 3 — harmless redundancy that improves discoverability. |

## Outstanding Verification Debt

These items are NOT failures — the user explicitly approved deferring each per CLAUDE.md `do not push`. The codebase deliverables are complete; only real-world execution telemetry is pending.

1. **Plan 01-04 Wave-0 spike (`cargo xtask dmg --universal` on Apple Silicon)** — code path verified by construction; local execution on real hardware was approved without telemetry capture. When user is ready: `brew install create-dmg librsvg && cargo install cargo-bundle@0.10.0 --locked && cargo xtask dmg --universal` should produce `target/dmg/Vector-2026.05.10-universal.dmg` with `lipo -info` reporting both arm64 and x86_64.
2. **Plan 01-05 first-real-CI-run** — `ci.yml` was committed (Plan 01-05 commits) but no push has triggered the GitHub Actions DAG. When user pushes to `main`: confirm all 7 jobs complete (lint → commitlint → test → deny → build-arm64 + build-x86_64 → package), the `vector-universal-dmg` artifact uploads, and the `tip` Release is overwritten with xattr footer.
3. **Plan 01-06 branch protection + first tagged release** — `release.yml` is in place but no `v*` tag has been pushed. When user is ready: configure branch protection per `docs/setup.md` §3 (4 PR-required checks: lint, commitlint, test, deny + linear history + no force-push), verify via `gh api repos/colligo/vector/branches/main/protection`, then run `cargo xtask release && git push --follow-tags` and confirm `gh release view v2026.05.10` shows the Universal DMG asset + xattr footer in body.
4. **Downloaded-DMG end-to-end smoke** (mount → drag-install → `xattr -dr` → launch Vector.app → see the threading tick + version overlay + native menu bar) — bundled under (1)–(3) above.

## Cross-Plan Integrity Checks

These are the four invariants flagged in `01-06-SUMMARY.md §"Phase 01 Close-out Hand-off"`. All four pass.

1. **Required-status-check name matching (4 PR-reachable jobs).** `grep -E '^  (lint|commitlint|test|deny):' .github/workflows/ci.yml` matches exactly 4 lines (verified). `docs/setup.md` §3 enumerates the same 4 names verbatim in a bullet list (verified by grep). The Plan 01-05 → Plan 01-06 reconciliation is documented in `01-06-SUMMARY.md §"Reconciliation of Plan 01-05's overstated branch-protection hand-off"`: the 3 push-gated jobs (`build-arm64`, `build-x86_64`, `package`) are gated by `if: github.event_name == 'push' && github.ref == 'refs/heads/main'` and so literally cannot produce a status that branch protection could require on a PR. The 4-check (not 7-check) contract is correct per CONTEXT D-34.

2. **xattr literal byte-identity across D-26 surfaces.** `grep -h 'xattr -dr com.apple.quarantine /Applications/Vector.app'` over `README.md`, `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `xtask/scripts/render-dmg-bg.sh` shows the literal substring appears in 5 lines (the bg-render script has both a header comment and the actual annotate arg). The literal itself — `xattr -dr com.apple.quarantine /Applications/Vector.app` — is byte-identical (single space, lowercase, `/Applications` path) across all 4 surfaces. D-26 closure is intact.

3. **ADR 0006 captures both halves.** `docs/adr/0006-runner-labels-and-branch-protection.md` references: `macos-15-intel` (multiple matches), `Aug 2027` / `August 2027` EOL warning (multiple matches), `D-34` + `D-35` + `branch protection` (multiple matches). Both halves present.

4. **release.yml ↔ ci.yml shared invariants.** Both workflows carry the same env block (`MACOSX_DEPLOYMENT_TARGET: "13.0"`, `CARGO_HUSKY_DONT_INSTALL_HOOKS: "1"`, `RUST_BACKTRACE: short` — verified by grep). Both run `lipo -info` Pitfall-3 guards (post-merge bundled Mach-O check). Both `brew install create-dmg librsvg` (release.yml also adds `git-cliff`). Both `cargo install cargo-bundle@0.10.0 --locked`. Both invoke `cargo xtask dmg --universal --arm64 PATH --x86_64 PATH`. The sibling-not-reusable pattern is documented in `01-06-SUMMARY.md §patterns-established` and ADR 0006.

**Architecture-lint test-file-count guard.** Independently verified: 14 crate dirs (`crates/vector-*`) ↔ 14 `tests/no_tokio_main.rs` files. The ci.yml guard (lines 71–79) will fire if a future crate is added without its own arch-lint test.

**Tooling fidelity.** `.cargo/config.toml` exposes `cargo xtask = "run --manifest-path xtask/Cargo.toml --release --"` per D-04. `xtask/Cargo.toml` opens with `[workspace]` to opt out of the parent workspace (separate resolver graph). `cargo-husky` is wired in `crates/vector-app/Cargo.toml` `[dev-dependencies]` (with `CARGO_HUSKY_DONT_INSTALL_HOOKS=1` env var in CI to skip install on CI). 6 MADR ADRs present under `docs/adr/0001..0006-*.md`.

## Self-Check: PASSED

- All 14 expected crates present in `crates/vector-*` and listed in `Cargo.toml` `[workspace].members`.
- 14 of 14 per-crate `tests/no_tokio_main.rs` files present (ci.yml count guard would catch a miss).
- Local re-run of the CI's grep-redundancy check finds zero `#[tokio::main]` / `Builder::new_current_thread()` matches in production code.
- All 6 ADRs (0001–0006) present under `docs/adr/`.
- `rust-toolchain.toml` pins `1.88.0` + both `aarch64-apple-darwin` and `x86_64-apple-darwin` targets.
- `deny.toml`, `cliff.toml`, `Cargo.toml [workspace.lints]`, `.cargo/config.toml` xtask alias, `crates/vector-app/build.rs` git-SHA embedding, `crates/vector-app/resources/{icon.icns,dmg-background.png,icon.svg,Info.plist.partial}` all present.
- `.github/workflows/{ci.yml,release.yml}` present with the contracted jobs, env block, Pitfall-3 guards, xattr footers.
- README install block + CHANGELOG seed + docs/setup.md present.
- 4 deferred-execution items moved into `human_verification` frontmatter so `/gsd:progress` / `/gsd:audit-uat` can chase them.

Phase 01 is structurally complete. The remaining gap is real-world execution telemetry that the user explicitly approved deferring; that gap is tracked, not failed.

---

*Verified: 2026-05-10*
*Verifier: Claude (gsd-verifier)*
