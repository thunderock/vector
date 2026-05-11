# 0001. Rust workspace layout

- Status: accepted
- Date: 2026-05-10
- Deciders: solo (user)
- Tags: phase-1, build, structure

## Context and Problem Statement

Vector ships from a single repo. We need a structure that scales to ~14 logical
units (terminal core, renderer, mux, transports, config, secrets, fonts, input,
theme, app shell) without making `cargo build` glacial or hiding architectural
boundaries.

## Decision Drivers

- Incremental compile speed (touching one crate must not rebuild all)
- Architectural visibility (folder structure mirrors the layered diagram)
- Single source of truth for dep versions (no russh-style dual-version drift)
- xtask separate from main resolver graph

## Considered Options

- Single-crate workspace with module layout
- 14-crate workspace with shared `[workspace.dependencies]`
- 14-crate workspace plus xtask in the same workspace
- 14-crate workspace plus xtask as a SEPARATE workspace

## Decision Outcome

14-crate workspace + xtask SEPARATE workspace, per D-01..D-04. Workspace deps
pinned once at the root in `[workspace.dependencies]` (D-02); each crate uses
`dep.workspace = true`. xtask lives at `xtask/` with its own `Cargo.toml` and
`Cargo.lock`, invoked via `.cargo/config.toml` alias.

## Pros and Cons of the Options

- **Single-crate workspace:** simple; fails the architectural-visibility driver.
- **14-crate, xtask in same workspace:** keeps everything in one resolver graph;
  xtask deps pollute the audit surface (cargo deny sees tooling).
- **14-crate, xtask separate workspace (chosen):** clean audit boundary; one
  extra Cargo.lock to maintain; standard cargo idiom (`[workspace]` opt-out).

## Consequences

- Adding a new crate = `mkdir crates/vector-foo + Cargo.toml + src/lib.rs +
  tests/no_tokio_main.rs` + listing in `Cargo.toml [workspace.members]`.
- xtask deps don't pollute the main resolver — fast incremental builds.
- Single dep bump propagates to all 14 crates atomically.
- `cargo deny` audits the main workspace only; xtask deps are unaudited (accepted).
