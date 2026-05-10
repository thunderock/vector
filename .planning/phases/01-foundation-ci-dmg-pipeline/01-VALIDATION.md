---
phase: 1
slug: foundation-ci-dmg-pipeline
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-10
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. The planner will refine this once tasks are decomposed; this draft fills the Nyquist gate so planning can proceed.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (built-in) — Rust integration + unit tests across the workspace |
| **Architecture lint** | Per-crate `tests/no_tokio_main.rs` integration tests + a CI `grep` redundancy step |
| **Config file** | None beyond `Cargo.toml` `[workspace]` and per-crate `[dev-dependencies]` |
| **Quick run command** | `cargo test --workspace --no-fail-fast --quiet` |
| **Full suite command** | `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace --no-fail-fast && cargo deny check advisories licenses bans sources` |
| **Universal-DMG smoke** | `cargo xtask dmg --universal` (local) — produces `target/release/bundle/dmg/Vector-{version}-universal.dmg`; CI runs the same path on the `package` job |
| **Estimated runtime** | ~30s quick / ~3 min full local / ~6–8 min full CI matrix-then-merge |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p {crate-being-modified} --no-fail-fast --quiet` (or workspace-wide if the touch crosses crates).
- **After every plan wave:** Run the **full suite command** above.
- **Before `/gsd-verify-work`:** Full suite must be green AND `cargo xtask dmg` must succeed locally on Apple Silicon AND CI's `tip` Release must show a downloadable Universal DMG.
- **Max feedback latency:** ~30s for the quick path; the architecture-lint test runs as part of `cargo test` so violations surface within that budget.

---

## Per-Task Verification Map

> The planner will populate one row per task once PLAN.md files are decomposed. This draft enumerates the **success-criterion → verification mechanism** mapping the planner must respect.

| Success criterion (roadmap) | Requirement IDs | Verification mechanism | Automated? | Notes |
|----|----|----|----|----|
| 1. Push to `main` produces downloadable `Vector.dmg` artifact | BUILD-01, BUILD-02, BUILD-04 | CI workflow run on `main` shows green `package` job; artifact downloadable from the run page AND from the pinned `tip` Release | ✅ (CI) | False-positive guard: post-CI `lipo -info Vector.app/Contents/MacOS/vector` must report `x86_64 arm64`. |
| 2. Tag `v*` publishes Universal DMG to Releases with xattr instructions | BUILD-04, BUILD-05 | Tagged release run completes; `gh release view v{ver}` shows DMG asset and the `xattr -dr com.apple.quarantine` line in the body | ✅ (CI) | False-positive guard: the README, the DMG background image, and the Release body must each independently contain the `xattr -dr com.apple.quarantine /Applications/Vector.app` string (D-26). |
| 3. `cargo xtask dmg` locally produces an identical DMG on Apple Silicon | BUILD-03 | Local script test: `cargo xtask dmg --universal` then `lipo -info` on the embedded binary; `shasum -a 256` of the bundled `vector` binary matches CI's published binary for the same git SHA | 🟡 (local manual + scripted hash compare) | Manual gate during phase verification; the scripted hash compare belongs in `xtask/tests/` and runs only when `VECTOR_LOCAL_DMG_TEST=1` is set (CI skips it). |
| 4. winit/tokio threading invariant enforced | WIN-05, BUILD-04 | (a) `cargo test --workspace` runs every crate's `tests/no_tokio_main.rs` and FAILS if any forbidden token is found in `src/`; (b) CI grep step `! grep -rn '#\[tokio::main\]\|Builder::new_current_thread()' crates/*/src` returns non-zero on violation; (c) **smoke test:** running the built binary for ≥3s shows the window title cycle through `Vector — tick 0`, `Vector — tick 1`, `Vector — tick 2` (proves the cross-thread `EventLoopProxy::send_event` path is live). | ✅ (cargo test + CI grep) + 🟡 (smoke test manual unless we instrument it) | False-positive guard: `Builder::new_current_thread()` is allowed in `xtask/` (separate workspace, not under `cargo test --workspace`); the per-crate test must scope its scan to its own `src/` only, not pull in xtask. |

*Status legend: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky · 🟡 manual or partially manual*

---

## Wave 0 Requirements

Phase 1 is greenfield — Wave 0 establishes the test scaffolding the rest of the phase plugs into. Required Wave-0 outputs:

- [ ] `Cargo.toml` workspace root with `[workspace.dependencies]`, `[workspace.lints]`, and the 14-crate member list (D-01..D-06).
- [ ] `rust-toolchain.toml` exact pin to `1.88.0` with both Apple Darwin targets (D-05).
- [ ] One template `tests/no_tokio_main.rs` integration test under `crates/vector-app/tests/` proving the lint mechanism works (the per-crate replicas land in later tasks but the pattern lands here).
- [ ] A `tests/smoke_universal_binary.rs` (or equivalent xtask check) that invokes `lipo -info` on a CI-built binary and asserts `x86_64 arm64` — covers the false-positive trap where a build accidentally produces a single-arch fat-headed Mach-O.
- [ ] A `cargo-bundle` Wave-0 spike: `cargo xtask dmg-spike` on Apple Silicon dev machine to confirm cargo-bundle 0.10 + a pre-built universal binary produce the expected `.app` layout (researcher flagged this as MEDIUM confidence).
- [ ] A `cargo-husky` Wave-0 spike: confirm `CARGO_HUSKY_DONT_INSTALL_HOOKS=1` actually suppresses hook install on CI runners (researcher flagged this as MEDIUM confidence).

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----|----|----|----|
| Standard menu bar visible with functional Cmd-Q / Cmd-M / Cmd-W / Window→Zoom | WIN-05, BUILD-04 | AppKit menu state isn't readable via stdin/stdout; would need full UI automation (out of v1 scope). | Launch `target/release/Vector.app`; verify menu bar reads `Vector / File / Edit / View / Window / Help`; press Cmd-Q (quits), Cmd-M (minimizes), Cmd-W (closes window), Window→Zoom (toggles). |
| `Vector v{version} (build {sha})` overlay renders centered, readable, native-styled | BUILD-04 | Visual inspection of NSTextField rendering. | Launch the app; visually confirm overlay text matches `Vector v2026.MM.DD (build {short-sha})` against `git rev-parse --short HEAD`. |
| `xattr -dr com.apple.quarantine /Applications/Vector.app` shown on the DMG background image | BUILD-05 | Image asset; not text-greppable post-render. | Open the produced `Vector-{version}-universal.dmg` in Finder; visually confirm background image renders the xattr line legibly. |
| GitHub branch protection rules active on `main` | BUILD-04 | GitHub UI/API state, not a repo artifact. | `gh api repos/{owner}/vector/branches/main/protection` returns rules including required status checks (`fmt`, `clippy`, `test`, `deny`, `commitlint`), linear history required, force-push disabled. Document the call in ADR 0006 (or whichever ADR captures branch protection) so the audit is repeatable. |

---

## Validation Sign-Off

- [ ] Every PLAN.md task has either an `<automated>` verify field or a Wave 0 dependency
- [ ] Sampling continuity: no 3 consecutive tasks without an automated verify
- [ ] Wave 0 covers all MISSING references (esp. cargo-bundle universal-binary path behavior + cargo-husky CI-gate)
- [ ] No watch-mode flags in CI commands
- [ ] Feedback latency < 60s on the quick path
- [ ] `nyquist_compliant: true` set in frontmatter once planner refinements land

**Approval:** pending
