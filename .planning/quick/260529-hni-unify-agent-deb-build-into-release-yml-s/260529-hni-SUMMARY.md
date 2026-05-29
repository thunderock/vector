---
phase: quick
plan: 260529-hni
subsystem: ci-release
tags: [ci, github-actions, release, cargo-deb, dev-tunnel-agent]
requires:
  - ".github/workflows/release.yml (existing macOS DMG release flow)"
  - "crates/vector-tunnel-agent/Cargo.toml [package.metadata.deb]"
provides:
  - "Unified release.yml: one v* tag = one workflow producing Universal DMG + both agent .debs on one Release"
affects:
  - "GitHub Release asset set on every v* tag / published Release"
tech-stack:
  added: []
  patterns: ["single-workflow release fan-in (macOS + Linux matrix -> one release job)"]
key-files:
  created: []
  modified:
    - ".github/workflows/release.yml"
  deleted:
    - ".github/workflows/agent-release.yml"
decisions:
  - "Deleted agent-release.yml: identical triggers to release.yml + no independent trigger = fully redundant after fold-in"
metrics:
  duration: ~4min
  completed: 2026-05-29
---

# Quick 260529-hni: Unify Agent .deb Build into release.yml Summary

Folded the Linux agent `.deb` matrix build into `release.yml` so a single `v*` tag (or published Release) runs ONE workflow that produces the Universal `Vector-*-universal.dmg` AND both `vector-tunnel-agent_*_{amd64,arm64}.deb`, all attached to the same GitHub Release; retired the now-redundant `agent-release.yml`.

## What Changed

**Task 1 — Fold build-deb into release.yml (commit b54858f):**
- Added a `build-deb` matrix job (`amd64` via `x86_64-unknown-linux-gnu`, `arm64` via `aarch64-unknown-linux-gnu`) on `ubuntu-22.04`, `fail-fast: false`, copied verbatim from `agent-release.yml` — including the aarch64 cross-toolchain `~/.cargo/config.toml` heredoc, `cargo install --locked cargo-deb`, `cargo deb --no-build --output`, the dpkg-deb sanity grep, and the 90-day artifact upload (`name: agent-deb-${{ matrix.deb-arch }}`).
- Placed it between `build-x86_64` and `release`. No top-level `permissions:` added — `build-deb` only uploads workflow artifacts.
- `release` job: `needs: [build-arm64, build-x86_64, build-deb]`; added a third `download-artifact` (`pattern: agent-deb-*`, `path: artifacts/deb`, `merge-multiple: true`); added a "Verify agent .debs present" guard (asserts both amd64 + arm64 land); included `artifacts/deb/vector-tunnel-agent_*.deb` in BOTH the `gh release upload` branch (with `--clobber`) and the `gh release create` else branch, alongside the unchanged DMG glob.

**Task 2 — Retire agent-release.yml (commit f211a85):**
- `git rm .github/workflows/agent-release.yml`.

## Retirement Rationale

`agent-release.yml` triggered ONLY on `push tags ['v*']` and `release types [published]` — the exact triggers of `release.yml`. After Task 1, every artifact it produced (both `.deb`s, the 90-day CI-staging artifacts, and the Release attachment) is produced by `release.yml`. It had no independent trigger (no schedule, no `workflow_dispatch`, no PR/CI trigger), so it was fully redundant. Deleting it removes the dual-workflow race where both files fired on the same tag and competed to attach to the same Release.

## Glob Consistency (verified)

- cargo-deb `--output vector-tunnel-agent_<ver>_<deb-arch>.deb` -> upload path `target/<target>/debian/*.deb` -> download/upload glob `vector-tunnel-agent_*.deb`. Consistent.
- DMG glob `target/dmg/Vector-*-universal.dmg` unchanged and still present in the publish step.
- cargo-deb asset `usr/bin/vector-tunnel-agent` (Cargo.toml `[package.metadata.deb]`) matches the `dpkg-deb --contents | grep 'usr/bin/vector-tunnel-agent'` sanity step.
- `maintainer-scripts = "debian/"` (postinst + prerm at `crates/vector-tunnel-agent/debian/`) untouched.

## Verification

- `python3 yaml.safe_load(release.yml)` parses; `build-deb` present with amd64+arm64 on ubuntu-22.04; `release.needs == [build-arm64, build-x86_64, build-deb]`. (Task 1 automated check: PASS)
- `agent-release.yml` absent; release.yml retains DMG glob, adds `vector-tunnel-agent_*.deb`, and `usr/bin/vector-tunnel-agent` matches both workflow grep and Cargo.toml asset. (Task 2 automated check: PASS)
- actionlint not installed locally; YAML + glob-consistency checks used per plan.

## Deviations from Plan

None - plan executed exactly as written.

## Known Stubs

None.

## Self-Check: PASSED

- Commit b54858f: FOUND
- Commit f211a85: FOUND
- .github/workflows/release.yml: FOUND
- .github/workflows/agent-release.yml: deleted (FOUND absent)
- SUMMARY.md: FOUND
