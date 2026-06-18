---
phase: 10
slug: hardening-release
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-26
---

# Phase 10 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust 1.88) + `insta 1.47.2` (HARDEN-01 only, new) + `cargo-geiger 0.13` (HARDEN-03) + `cargo-deny 0.19.7` (HARDEN-03 advisories) |
| **Config file** | `Cargo.toml` (workspace), `deny.toml` (existing), `cargo-geiger.json` (new, HARDEN-03), `crates/vector-render-snapshots/Cargo.toml` (new, HARDEN-01) |
| **Quick run command** | `cargo test -p vector-term --tests vt_conformance` (HARDEN-02 only, ~1s) |
| **Full suite command** | `cargo test --workspace --all-features && cargo deny check && cargo geiger --forbid-only --output-format Json --update-readme=false` |
| **Estimated runtime** | ~90 seconds local; ~6–8 minutes CI (snapshot suite dominates) |

---

## Sampling Rate

- **After every task commit:** Run `cargo check --workspace` (fast type-check) + the touched crate's `cargo test -p <crate>`
- **After every plan wave:** Run `cargo test --workspace --all-features`
- **Before `/gsd:verify-work`:** Full CI matrix (lint, snapshot, vt-conformance, perf, cargo-deny, cargo-geiger, token-grep, release smoke) must be green on macos-14
- **Max feedback latency:** 90 seconds local; PR-blocking gates may take up to 10 minutes CI

---

## Per-Task Verification Map

> Plan-level skeleton. Final task IDs filled in by `gsd-planner` as 10-01-PLAN.md..10-04-PLAN.md crystallize.

| Plan | Requirement | Test Type | Automated Command | Notes |
|------|-------------|-----------|-------------------|-------|
| 10-01 (HARDEN-01) | HARDEN-01 | snapshot (insta) | `cargo test -p vector-render-snapshots --tests` | Goldens committed; `INSTA_UPDATE=no` in CI. Hard gate macos-14 arm64. |
| 10-01 (HARDEN-01) | HARDEN-01 | perceptual diff | Built into snapshot test via `image-compare 0.5` SSIM ≥ ~0.98 (locked threshold ~delta-E 2.0 per D-03) | Fails snapshot if SSIM below threshold |
| 10-02 (HARDEN-02) | HARDEN-02 | unit (vt_conformance corpus) | `cargo test -p vector-term --tests vt_conformance` | 8 scenarios: alt-screen, scroll regions, tab stops, ED/EL, mouse 1006, OSC 52 r/t, bracketed paste, DECSCUSR. Relocates D-28 precedents. |
| 10-02 (HARDEN-02) | HARDEN-02 | perf gate | `cargo bench -p vector-term --bench idle_cpu` + `cargo bench --bench cat_large_log` (or custom probe per D-10) | Hard gate macos-14 arm64; advisory macos-15-intel (D-23) |
| 10-03 (HARDEN-03) | HARDEN-03 | unsafe-ban | `cargo geiger --forbid-only --output-format Json` checked against `cargo-geiger.json` allowlist | Allowlist per D-22 (objc2*, wgpu, alacritty_terminal, crossfont, portable-pty) |
| 10-03 (HARDEN-03) | HARDEN-03 | advisories/licenses/sources | `cargo deny check` | Existing deny.toml policy continues unchanged per D-13 |
| 10-03 (HARDEN-03) | HARDEN-03 | static token-leak gate | `cargo test -p vector-arch-tests --test no_token_in_debug_or_log` | Already shipped per D-29; verify still green |
| 10-03 (HARDEN-03) | HARDEN-03 | runtime token-leak grep | `RUST_LOG=debug cargo test -p vector-auth --tests integration -- --nocapture 2>&1 \| grep -E "gho_\|ghp_\|eyJ" \| wc -l` == 0 | New; wiremock-backed auth flow |
| 10-04 (HARDEN-04) | HARDEN-04 | release-pipeline smoke (no tag) | `cargo xtask dmg --universal` then assert `Vector-1.0.0-universal.dmg` + `.sha256` exist | Runs in CI on push to non-tag branches as dry-run |
| 10-04 (HARDEN-04) | HARDEN-04 | README install block grep | `grep -F "xattr -dr com.apple.quarantine /Applications/Vector.app" README.md && grep -F "## Install" README.md` | Static check, sub-1s |
| 10-04 (HARDEN-04) | HARDEN-04 | PERSIST-04 pre-flight | `grep -E "^- ?\*\*PERSIST-04\*\*.+(Complete\|Done)" .planning/REQUIREMENTS.md` | Per D-25 — release task halts if not Complete |
| 10-04 (HARDEN-04) | HARDEN-04 | release publication (one-shot) | Manual: tag `v1.0.0`, observe `gh release view v1.0.0` shows DMG + checksum + hand-written notes | One-shot; not a regression gate. Manual sign-off per D-18. |

*Status filled in as tasks materialize.*

---

## Wave 0 Requirements

Wave 0 establishes test infrastructure that downstream task tests depend on. For Phase 10:

- [ ] `crates/vector-render-snapshots/Cargo.toml` — new crate scaffold with `insta = "1.47.2"` and `image-compare = "0.5.0"` as dev-deps (HARDEN-01, D-05/D-26)
- [ ] `crates/vector-render-snapshots/tests/scenes/` — directory for the 4 (expandable to 8) initial scene fixtures
- [ ] `crates/vector-render-snapshots/snapshots/` — `insta` golden directory; pre-create with empty `.gitkeep`
- [ ] `crates/vector-term/tests/vt_conformance/` — new test directory consolidating D-28's precedent files (HARDEN-02, D-07)
- [ ] `cargo-geiger.json` allowlist file at workspace root (HARDEN-03, D-22)
- [ ] `Cargo.toml` workspace `members` extended with `crates/vector-render-snapshots` (HARDEN-01)
- [ ] `.github/workflows/ci.yml` job slots: `snapshot-suite` (macos-14), `vt-conformance` (macos-14), `perf-gate` (macos-14 hard + macos-15-intel advisory per D-23), `cargo-deny` (existing or extended), `cargo-geiger` (new), `token-redaction-grep` (new)
- [ ] `.github/workflows/release.yml` `release` job grown with `lipo` + bundle + dmg + `shasum -a 256` + `gh release upload` per D-19

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| GitHub Release v1.0.0 publishes the Universal DMG + checksum to the Releases page | HARDEN-04 | One-shot tag-driven workflow; the release either exists or doesn't. Not a regression test — a "did we ship?" gate. | After PERSIST-04 sign-off + green CI on master: `git tag v1.0.0 && git push --tags`. Then on GitHub Releases: confirm `Vector-1.0.0-universal.dmg` + `Vector-1.0.0-universal.dmg.sha256` are attached; release body contains hand-written notes per D-18. |
| Teammate install dry-run | HARDEN-04 | The `xattr` + `open` ritual exercises macOS Gatekeeper behavior that CI cannot simulate. | On a teammate's machine (or fresh user account): download DMG, drag to `/Applications`, run the two-line README install block. Confirm Vector launches and a terminal session works. |
| Snapshot golden review on first land | HARDEN-01 | Initial PNGs need human "yes, that looks right" before becoming baseline. | First PR landing HARDEN-01 includes the 4 initial PNGs; reviewer eyeballs each and confirms expected rendering before approving. |
| v1.0.0 release-notes accuracy | HARDEN-04 | The "what's in v1 / what's out" narrative needs human judgement. | Author drafts; reviewer cross-checks against `.planning/REQUIREMENTS.md` (51 v1 IDs in) + `.planning/REQUIREMENTS.md#out-of-scope` (DIST-V2-01/02 + 999.x backlog out). |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (insta + image-compare + cargo-geiger + vt_conformance dir + workspace members + CI job slots)
- [ ] No watch-mode flags
- [ ] Feedback latency < 90s local
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
