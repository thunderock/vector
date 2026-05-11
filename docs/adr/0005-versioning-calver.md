# 0005. Versioning, CHANGELOG, and Conventional Commits

- Status: accepted
- Date: 2026-05-10
- Deciders: solo (user)
- Tags: phase-1, release, ci, dx

## Context and Problem Statement

Vector ships from a solo dev to a small audience (the user + a few Adobe
teammates). We need a version scheme that's calendar-meaningful, an automated
CHANGELOG generator, and commit-message structure that supports both.

## Decision Drivers

- CalVer reads as "when shipped" (matters for an unsigned dev tool)
- CHANGELOG hand-editing is a tax we don't want
- Conventional Commits → automated CHANGELOG via git-cliff
- CI must enforce commit format, otherwise the automation breaks silently

## Considered Options

- SemVer + manual CHANGELOG (rejected — premature semver theater for a v1)
- CalVer `YYYY.MM.DD` (chosen)
- CalVer with same-day -N suffix (rejected — D-27, one release per day)
- commitlint (Node) vs convco (Rust binary)

## Decision Outcome

Per D-27, D-29, D-30. CalVer `YYYY.MM.DD` set in `[workspace.package].version`;
`cargo xtask release` bumps it to today's date and tags `v{version}`.
git-cliff (cliff.toml) generates Keep-a-Changelog-format sections from
Conventional Commits. CI enforces commit format with `convco` (single Rust
binary; matches our Rust-only stance — see RESEARCH.md §"Conventional Commits").

## Pros and Cons of the Options

- **SemVer:** signals API stability we don't yet have; manual CHANGELOG burden.
- **CalVer YYYY.MM.DD (chosen):** "when did this ship" maps directly to support
  questions; sortable; no semantic-promise theater.
- **commitlint (Node):** drags Node into the toolchain for one check.
- **convco (chosen):** single binary, ~5s install in CI, Rust-only stance.

## Consequences

- Same-day re-releases are not supported (a second `cargo xtask release`
  fails because the tag exists). Acceptable for v1 cadence.
- The 11 commit-type → CHANGELOG-section mappings live in `cliff.toml`;
  unconventional commits don't break the build (filter_unconventional = false)
  but won't appear in CHANGELOG sections.
- `convco` is downloaded from GitHub Releases in CI (~5s); avoids dragging Node
  into the toolchain.

## Amendment 2026-05-11: unpadded CalVer in Cargo.toml

Cargo's SemVer parser rejects leading zeros in any version component
(`error: invalid leading zero in minor version number`). The original
`%Y.%m.%d` format produced `2026.05.10`, which fails to parse. Decision:
use **unpadded** CalVer (`%Y.%-m.%-d` → `2026.5.10`) consistently in
`Cargo.toml`, the git tag (`v2026.5.10`), and the DMG filename
(`Vector-2026.5.10-universal.dmg`). One format, one source of truth.
Sorting safety: use `git tag --sort=v:refname` (version-aware sort) when
listing tags chronologically — string sort gets `2026.5.10` vs `2026.5.7`
wrong, but version sort handles it correctly. Validated against the first
real `cargo xtask release` invocation on 2026-05-11.

Also: `git tag {name}` produces a lightweight tag, which is silently
skipped by `git push --follow-tags`. `xtask::release` now uses `git tag -a`
to produce annotated tags so the standard push flow ships them. The
lightweight-tag misfire was caught when the user's first push reported
"Everything up-to-date" with no tag on GitHub.
