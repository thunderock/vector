---
created: 2026-05-11T17:21:20.022Z
title: Code-quality hardening — workspace lints, arch-lint upgrade, pre-commit cargo-deny
area: tooling
target_phase: 5
files:
  - Cargo.toml
  - crates/*/tests/no_tokio_main.rs
  - .pre-commit-config.yaml (or equivalent)
  - deny.toml
---

## Problem

Phase 2 introduced cross-crate path dependencies for the first time (`vector-headless` → `vector-term`/`vector-pty`/`vector-mux`, `vector-mux` → `vector-pty`). Plan 02-01 omitted explicit `version = "..."` specifiers on those path deps, which cargo-deny treats as wildcards. CI broke on the post-phase docs commit (`de791b6`) with `bans FAILED` — `error[wildcard]: found 3 wildcard dependencies for crate 'vector-headless'`. Fixed in `d652c8b` by pinning all four path deps to `version = "2026.5.10"`.

The local arch-lint (`tests/no_tokio_main.rs` per crate) caught the file-count invariant (15==15) but didn't catch the Cargo.toml content regression. Pre-commit hooks run fmt/clippy but not `cargo deny check`, so the failure only surfaced on CI after the user pushed.

This will keep happening as new crates land. Need belt-and-suspenders: lint inheritance for newly-added crates, arch-lint that enforces version specifiers on path deps, and local cargo-deny in pre-commit.

## Solution

Phase 5 (Polish — local daily-driver) is the natural home; it already targets code-quality polish. Four sub-items:

1. **`[workspace.lints]` inheritance.** Add a workspace-level lint block in top-level `Cargo.toml`:
   ```toml
   [workspace.lints.rust]
   unsafe_code = "forbid"
   missing_docs = "warn"  # consider deny once stable
   [workspace.lints.clippy]
   all = { level = "warn", priority = -1 }
   pedantic = { level = "warn", priority = -1 }
   await_holding_lock = "deny"   # already enforced in code, now lint-gated
   ```
   Each crate adds `[lints] workspace = true` — Phase 2 crates already do, so this is additive.

2. **Arch-lint upgrade.** Extend each crate's `tests/no_tokio_main.rs` (or factor into a workspace-level integration test) to:
   - Parse own `Cargo.toml` via `cargo_metadata` or `toml`
   - Assert every `dependencies.*` entry that has `path` ALSO has `version`
   - Fail with a clear "missing version =" message naming the offending line
   This catches the regression at `cargo test --workspace --tests` time, before push.

3. **`cargo deny check` in pre-commit.** Add to `.pre-commit-config.yaml` (or create one — Phase 1 may not have set it up yet):
   ```yaml
   - id: cargo-deny
     name: cargo deny check
     entry: cargo deny check bans licenses sources advisories
     language: system
     pass_filenames: false
     stages: [pre-commit]
   ```
   Optionally `pre-push` stage to keep local commits fast.

4. **`cargo-machete` for unused-dep detection.** Lighter-weight than cargo-udeps; runs on stable. Adds dev-time signal that we're not dragging in libs we don't use.

## Origin / context

Surfaced during `/gsd:execute-phase 2` follow-up after CI run on PR #5 (`docs(phase-02): evolve PROJECT.md after phase completion`) showed cargo-deny `bans FAILED`. User asked: "is there a way to enforce code quality here in this repo and this remains easy to understand, contribute and add features later". This todo answers that question.

## Related

- Backlog 999.1 (AI autocomplete) — independent v2 ambition, NOT this todo
- ADRs 0004–0006 (Phase 1 CI divergences) — useful background on the current CI/lint surface
