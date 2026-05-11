# 0003. Architecture-lint mechanism

- Status: accepted
- Date: 2026-05-10
- Deciders: solo (user)
- Tags: phase-1, ci, lint, win-05

## Context and Problem Statement

Decision 0002's threading rule must survive future contributors and future-self
under deadline pressure. A clippy lint can be silenced; an ADR can be ignored.
We need a structural enforcement that fails the build on regression.

## Decision Drivers

- Belt-and-braces: test in code + grep redundancy in CI
- Per-crate (not per-workspace) so xtask can legitimately use `block_on`
- Test failure surfaces within `cargo test` — same place all other failures live

## Considered Options

- Workspace-wide grep in a CI step only (rejected — no local feedback)
- Cargo lint plugin (rejected — heavy machinery for a 50-line check)
- Per-crate `tests/no_tokio_main.rs` integration test + CI grep redundancy

## Decision Outcome

Per-crate `tests/no_tokio_main.rs`, per D-08. Each of the 14 crates carries a
copy of the test that scans its own `src/` for `#[tokio::main]`,
`#[tokio::test]`, `Builder::new_current_thread()`, `Runtime::new()`, and
`block_on(` (allowlist: `vector-app/src/main.rs`). CI workflow's `test` job
also runs an `rg`-based grep step over `crates/**/*.rs` (excluding the test
files themselves) — same forbid list — as redundancy if a future crate is
added without copying the test.

## Pros and Cons of the Options

- **CI-only grep:** zero local feedback; violation only surfaces after push.
- **Cargo lint plugin:** custom toolchain dep; weight not justified.
- **Per-crate test + CI grep (chosen):** local `cargo test` fails fast;
  CI redundancy catches new crates that forgot to copy the test file.

## Consequences

- New crate added without copying the test → CI `test` job's file-count guard
  fails (Plan 01-05 ci.yml step "Architecture-lint per-crate test file count").
- Local dev sees the violation in `cargo test --workspace` within ~10s.
- xtask is in a separate workspace and is NOT touched by `cargo test
  --workspace`; xtask is ALLOWED to use `block_on` (it's a build tool, not
  the running app).
