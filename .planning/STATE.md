---
gsd_state_version: 1.0
milestone: v1.0.0
milestone_name: milestone
status: Executing Phase 01
stopped_at: Completed Plan 01-04 (xtask DMG pipeline + CalVer release subcommand + Wave-0 cargo-bundle spike approved). Ready for Plan 01-05 (GitHub Actions CI).
last_updated: "2026-05-11T02:33:07.298Z"
progress:
  total_phases: 10
  completed_phases: 0
  total_plans: 6
  completed_plans: 4
---

# Project State: Vector

**Last updated:** 2026-05-10 (after Wave 4 — plan 01-04 complete, Wave-0 cargo-bundle spike approved)

## Project Reference

**Core value:** Open the app, pick a Codespace, get a fast remote shell — no VS Code, no browser, no clunky `gh codespace ssh` plumbing. Local-terminal niceties are table-stakes; the differentiator is that a Codespaces / Dev-Tunnels session feels native, not bolted on.

**Current focus:** Phase 01 — foundation-ci-dmg-pipeline

## Current Position

Phase: 01 (foundation-ci-dmg-pipeline) — EXECUTING
Plan: 5 of 6 (01-01..01-04 complete; next is 01-05 GitHub Actions CI)

## Phase Map

| # | Phase | Status |
|---|-------|--------|
| 1 | Foundation & CI/DMG Pipeline | In progress (4/6 plans) |
| 2 | Headless Terminal Core | Not started |
| 3 | GPU Renderer & First Paint | Not started |
| 4 | Mux — Tabs & Splits | Not started |
| 5 | Polish (Local Daily-Driver) | Not started |
| 6 | GitHub Auth + Codespaces Picker | Not started |
| 7 | SSH Transport + Codespaces Connect | Not started |
| 8 | Dev Tunnels Integration | Not started (spike-gated) |
| 9 | Persistence + Reconnect + tmux Auto-Attach | Not started |
| 10 | Hardening & Release | Not started |

## Performance Metrics

| Metric | Value |
|--------|-------|
| Phases planned | 10 |
| Phases complete | 0 |
| Plans complete | 4 |
| v1 requirements mapped | 51 / 51 (100%) |
| v1 requirements completed | 3 / 51 (WIN-05, BUILD-01, BUILD-03) |

## Accumulated Context

### Key Decisions

- **Build fresh in Rust** (not fork ghostty/VS Code). Rationale: ghostty is Zig, VS Code is Electron; the Rust ecosystem (`alacritty_terminal`, `wgpu`, `tokio`, `russh`, `octocrab`) is mature enough to build cleanly without a fork.
- **Codespaces SSH v1 transport = subprocess `gh codespace ssh --stdio`.** Native `russh + tonic` over the port-16634 gRPC management API is v1.x. This eliminates the gnarliest protocol work from the v1 critical path while delivering the full user-facing feature.
- **Dev Tunnels Phase 8 is spike-gated.** Day 1 of Phase 8 is a 1–2 day spike that commits a decision document among (a) subprocess `code tunnel client`, (b) vendor `microsoft/dev-tunnels/rs/` at pinned SHA, (c) defer to v2. Defer-to-v2 is an acceptable outcome.
- **Persistence strategy = remote tmux, not Mosh.** `tmux new -A -s vector-{profile-id}` on connect; reconnect re-attaches. No custom remote agent, no predictive-echo state-sync protocol.
- **Defer signing/notarization to v2.** Unsigned Universal DMG with documented `xattr -dr com.apple.quarantine` bypass for v1.
- **TOML config only.** No Lua, no DSL. Hot-reload via `notify` (FSEvents).
- **`Domain` / `Pane` / `PtyTransport` seam** (WezTerm pattern) is the only boundary between terminal model and transport. Established in Phase 4; load-bearing for Phases 7, 8, 9.
- **`winit::EventLoop` on the main thread, `tokio` multi-thread runtime on background threads, `EventLoopProxy::send_event` as the only cross-thread signal.** Established in Phase 1 skeleton.
- **xtask separate workspace (D-04):** empty `[workspace]` table in `xtask/Cargo.toml` is the standard cargo idiom for opting OUT of the parent workspace. xtask deps don't pollute the main resolver graph and cargo-deny only audits shippable code.
- **Wave-0 cargo-bundle universal-binary spike (A5):** cargo-bundle 0.10 honors the pre-merged universal binary at `target/release/vector-app`. No `cargo-bundle --bin` post-process fallback needed.
- **`cargo xtask` is the single DMG build code path for both local + CI (D-22):** CI passes pre-built per-arch binaries via `--arm64 PATH --x86_64 PATH`; local invocation builds them on the fly. Pitfall-3 (`lipo -info` guard) fires in both contexts.
- **CalVer one-release-per-day (D-27):** `cargo xtask release` refuses to overwrite an existing tag for today's date; push-free per CLAUDE.md.

### Open Questions / Risk Register

- **Phase 8 Dev Tunnels** — highest known v1 risk. Spike outcome unknown until phase start.
- **Phase 7 native russh + gRPC path** (v1.x) — requires careful read of `cli/cli/internal/codespaces/grpc/client.go` and the `?internal=true&refresh=true` parameter behavior. Not a v1 blocker.
- **`russh 0.37` vs `0.60` version conflict** — only matters if Phase 8 spike picks "vendor SDK". Resolution: fork + bump, or accept ~3MB binary duplication.
- **Universal binary on CI** — `macos-14` runners are arm64-only, `macos-13` are x86_64. Matrix + `lipo` validated end-to-end in Phase 1, not assumed.
- **Basic IME (NSTextInputClient composition display) only in Phase 5.** Full IME with candidate window UI is v2 (TERM-V2-01).

### Research Artifacts

- `/home/colligo/vector/.planning/research/SUMMARY.md` — executive summary, 10-phase ordering converged on by all four research dimensions
- `/home/colligo/vector/.planning/research/STACK.md` — verified crate versions on crates.io as of 2026-05-10
- `/home/colligo/vector/.planning/research/FEATURES.md` — table-stakes vs differentiators, anti-feature list
- `/home/colligo/vector/.planning/research/ARCHITECTURE.md` — Domain/Pane/PtyTransport pattern, threading model
- `/home/colligo/vector/.planning/research/PITFALLS.md` — pitfalls ordered by damage potential; Dev Tunnels contingency plan

### Active Todos

- [x] Wave 1 (plan 01-01) — workspace scaffold complete
- [x] Wave 2 (plan 01-02) — architectural invariants complete
- [x] Wave 3 (plan 01-03) — AppKit window + threading skeleton complete (on macOS, user-approved checkpoint)
- [x] Wave 4 (plan 01-04) — DMG xtask pipeline complete (Wave-0 cargo-bundle spike approved on macOS)
- [ ] Wave 5 (plan 01-05) — GitHub Actions CI
- [ ] Wave 6 (plan 01-06) — release pipeline + README + ADRs
- [ ] Phase 1 verification + roadmap completion

### Blockers

- None. Development is now on macOS (resumed); Plan 01-03 landed cleanly with user-approved
  checkpoint. Plan 01-04 (DMG xtask pipeline) and later waves can proceed on this host.

## Session Continuity

**Last session:** 2026-05-11T02:33:07.295Z

**Stopped at:** Completed Plan 01-04 (xtask DMG pipeline + CalVer release subcommand + Wave-0 cargo-bundle spike approved). Ready for Plan 01-05 (GitHub Actions CI).

**Next action:**

```bash

# Continue execution from Wave 5 (GitHub Actions CI) on macOS

/gsd-execute-phase 1
```

The `/gsd-execute-phase` workflow auto-skips plans 01-01 / 01-02 / 01-03 / 01-04
(their SUMMARY.md files exist) and resumes from 01-05.

**Files to re-read on resume:**

1. `.planning/ROADMAP.md` — phase structure and success criteria
2. `.planning/REQUIREMENTS.md` — v1 requirements + traceability
3. `.planning/PROJECT.md` — core value, constraints, key decisions
4. `.planning/phases/01-foundation-ci-dmg-pipeline/01-01-SUMMARY.md` — workspace scaffold details
5. `.planning/phases/01-foundation-ci-dmg-pipeline/01-02-SUMMARY.md` — lints + cargo-deny + arch-lint details
6. `.planning/phases/01-foundation-ci-dmg-pipeline/01-03-SUMMARY.md` — threading skeleton + AppKit window + menu + overlay details
7. `.planning/phases/01-foundation-ci-dmg-pipeline/01-04-SUMMARY.md` — xtask DMG pipeline + CalVer release subcommand + Wave-0 cargo-bundle spike details (incl. brew/cargo-install prereqs hand-off to Plan 01-05's CI YAML)

---
*State initialized: 2026-05-10*
*Plan 01-04 completed: 2026-05-10*
