---
gsd_state_version: 1.0
milestone: v1.0.0
milestone_name: milestone
status: Ready to plan
stopped_at: Phase 2 context gathered (D-36..D-39 captured)
last_updated: "2026-05-11T06:03:38.148Z"
progress:
  total_phases: 10
  completed_phases: 1
  total_plans: 6
  completed_plans: 6
---

# Project State: Vector

**Last updated:** 2026-05-11 (Phase 1 operationally validated end-to-end on GitHub: ci.yml DAG green; release.yml published `v2026.5.10` with Universal DMG; user-confirmed DMG launches on macOS Sequoia. Five divergences from original plans captured in ADRs 0004/0005/0006 and per-plan SUMMARY addenda. Phase 2 next.)

## Project Reference

**Core value:** Open the app, pick a Codespace, get a fast remote shell — no VS Code, no browser, no clunky `gh codespace ssh` plumbing. Local-terminal niceties are table-stakes; the differentiator is that a Codespaces / Dev-Tunnels session feels native, not bolted on.

**Current focus:** Phase 2: Headless Terminal Core (Phase 1 complete + operationally validated; recommended entry: `/gsd:discuss-phase 2`)

## Current Position

Phase: 2
Plan: Not started

## Phase Map

| # | Phase | Status |
|---|-------|--------|
| 1 | Foundation & CI/DMG Pipeline | Complete + operationally validated (2026-05-11) |
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
| Phases complete | 1 |
| Plans complete | 6 |
| v1 requirements mapped | 51 / 51 (100%) |
| v1 requirements completed | 6 / 51 (WIN-05, BUILD-01, BUILD-02, BUILD-03, BUILD-04, BUILD-05) — all operationally validated end-to-end on GitHub on 2026-05-11 |
| Phase 01-foundation-ci-dmg-pipeline P05 | 1 task commit + checkpoint approved no-push | 2 tasks | 1 files |
| Phase 01-foundation-ci-dmg-pipeline P06 | 2 task commits + checkpoint approved no-action | 3 tasks | 10 files |

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
- **Wave-0 cargo-bundle spike result (Assumption A5 INVALIDATED, 2026-05-11):** cargo-bundle 0.10 re-runs `cargo build --release` for the host arch before bundling, overwriting any pre-merged universal binary at `target/release/vector-app`. The documented fallback (post-process: copy `target/universal-apple-darwin/release/vector-app` over `Vector.app/Contents/MacOS/vector-app` after `cargo bundle` runs) is now the default code path in `xtask::dmg::finalize`. Conditional on the merged file existing, so `dmg_local` (host-arch) is unaffected. ADR 0004 amended.
- **Cargo SemVer unpadded CalVer (2026-05-11):** Cargo's SemVer parser rejects leading zeros in any version component. CalVer in `Cargo.toml` is `2026.5.10` (unpadded), matching `xtask::release` `%Y.%-m.%-d` format and the tag `v2026.5.10` and the DMG filename `Vector-2026.5.10-universal.dmg`. ADR 0005 amended.
- **Annotated tags (2026-05-11):** `xtask::release` uses `git tag -a` to produce annotated tags. Lightweight tags are silently skipped by `git push --follow-tags`.
- **Default branch is `master`, not `main` (2026-05-11):** ci.yml triggers on `branches: [master, main]` and push-gated job guards accept either branch. ADR 0006 amended.
- **`deny` job runs on `ubuntu-latest` (2026-05-11):** `EmbarkStudios/cargo-deny-action@v2` is a Docker action; macOS runners can't host Docker. cargo-deny is platform-agnostic. ADR 0006 amended.
- **release.yml dual-trigger (2026-05-11):** triggers on both `push: tags: ['v*']` (CLI flow) AND `release: published` (GitHub UI flow). Publish step detects existing release via `gh release view` and either creates with `gh release create` or attaches with `gh release upload --clobber` + `gh release edit`. `concurrency:` group keyed on tag prevents double-runs.
- **`cargo xtask` is the single DMG build code path for both local + CI (D-22):** CI passes pre-built per-arch binaries via `--arm64 PATH --x86_64 PATH`; local invocation builds them on the fly. Pitfall-3 (`lipo -info` guard) fires in both contexts.
- **CalVer one-release-per-day (D-27):** `cargo xtask release` refuses to overwrite an existing tag for today's date; push-free per CLAUDE.md.
- **CI pipeline (Plan 01-05):** `.github/workflows/ci.yml` is the single source of truth for what ships. 7-job PR-vs-push DAG with Pitfall-3 belt-and-braces; authored and committed (506b6bb) without push per CLAUDE.md. First-real-CI-run telemetry deferred as verification debt — surfaced in 01-05-SUMMARY for `/gsd:progress` and `/gsd:audit-uat` to chase.
- **Plan 01-05 textual deviation:** the macos-15-intel runner comment in ci.yml line 111 was reworded to drop the literal `macos-13` token (plan's verify clause asserts `! grep -q 'macos-13'`). D-21-amendment context preserved as "previous Intel runner retired Dec 2025". Same intent, no `macos-13` substring.
- **Branch-protection contract for Plan 01-06:** the 7 required-status-check job names are `lint, commitlint, test, deny, build-arm64, build-x86_64, package`. Plan 01-06's setup script must list these exactly; any rename in ci.yml requires a lock-step update or branch protection silently no-ops.
- **Plan 01-06 reconciliation:** docs/setup.md §3 enumerates only the 4 PR-reachable required-status-check names (lint, commitlint, test, deny). The 3 push-gated jobs (build-arm64, build-x86_64, package) cannot be required because they never run on PRs (per ci.yml D-17 conditional gate) — listing them would deadlock PR merges. ADR 0006 records the rationale; this reconciles Plan 01-05's overstated hand-off with CONTEXT D-34 ("Universal-DMG build is intentionally NOT a required check").
- **Phase 1 implementation complete (Plan 01-06):** release.yml + README install block (D-26 place 2 of 3) + CHANGELOG seed + 6 MADR ADRs (D-01..D-35 documented) + docs/setup.md branch-protection guide all committed (4dd0c4e + 75b77b1). xattr literal byte-identical across 4 surfaces (README, ci.yml tip body, release.yml tag body, DMG bg PNG). Terminal human-action checkpoint user-approved without GitHub UI action — branch-protection state + first-tagged-release deferred to user's async push per CLAUDE.md `do not push`.

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
- [x] Wave 5 (plan 01-05) — GitHub Actions CI authored + committed (506b6bb); checkpoint approved without push (first-real-CI-run telemetry deferred)
- [x] Wave 6 (plan 01-06) — release.yml + README install block + CHANGELOG seed + 6 MADR ADRs + docs/setup.md branch-protection guide committed (4dd0c4e + 75b77b1); checkpoint approved without GitHub UI action (branch-protection state + first-tagged-release deferred)
- [x] First real CI run telemetry: ci.yml DAG green on master @ 8e540ea; `tip` release `Vector-2026.5.10-tip-8e540ea-universal.dmg` (1.95 MB) published (2026-05-11)
- [x] First tagged release exercised: `release.yml` produced `Vector-2026.5.10-universal.dmg` on `v2026.5.10` tag push (2026-05-11; multi-attempt path due to A5 fallback discovery)
- [x] Downloaded DMG smoke-test: user-confirmed DMG launches on macOS Sequoia after `xattr -dr com.apple.quarantine` (window "Vector — tick N" visible)
- [x] Phase 1 verification + roadmap completion (gsd-tools phase complete; ROADMAP/STATE/REQUIREMENTS auto-updated)
- [ ] Branch protection configured per docs/setup.md §3 with the 4 PR-required check names (lint, commitlint, test, deny); `gh api repos/thunderock/vector/branches/master/protection` verifies the rule (only remaining Phase-1 deferred item)

### Blockers

- None. Development is now on macOS (resumed); Plan 01-03 landed cleanly with user-approved
  checkpoint. Plan 01-04 (DMG xtask pipeline) and later waves can proceed on this host.

## Session Continuity

**Last session:** 2026-05-11T06:03:38.144Z

**Stopped at:** Phase 2 context gathered (D-36..D-39 captured)

**Next action:**

```bash

# Phase 1 implementation is complete. The orchestrator runs phase verification next.

/gsd-execute-phase 1
```

The `/gsd-execute-phase` workflow detects all 6 plan SUMMARY.md files exist and
transitions to phase-verification mode (regression gate + verifier + ROADMAP /
Phase-Map close-out).

**Asynchronous user work (CLAUDE.md `do not push` — user pushes asynchronously):**

After reviewing the Phase 1 commits (4dd0c4e, 75b77b1, plus all prior commits since 506b6bb), the user should:

1. Push to GitHub: `git push origin master`.
2. Walk `01-05-SUMMARY.md §"Outstanding Verification Debt"` to close the first-real-CI-run debt for BUILD-02 / BUILD-04.
3. Configure branch protection on `main` per `docs/setup.md §3` (4 required checks: lint, commitlint, test, deny; linear history; no force-push) and verify via `gh api repos/colligo/vector/branches/main/protection`.
4. Cut the first tagged release: `cargo xtask release` + `git push --follow-tags`; watch via `gh run watch`; confirm `gh release view v{CalVer}` shows the Vector-{CalVer}-universal.dmg asset with xattr footer in body.
5. Smoke-test the published DMG: download → mount → drag → xattr de-quarantine → launch.
6. Walk `01-06-SUMMARY.md §"Outstanding Verification Debt"` to close items (1)–(5) for BUILD-04 / BUILD-05.

**Files to re-read on resume:**

1. `.planning/ROADMAP.md` — phase structure and success criteria
2. `.planning/REQUIREMENTS.md` — v1 requirements + traceability
3. `.planning/PROJECT.md` — core value, constraints, key decisions
4. `.planning/phases/01-foundation-ci-dmg-pipeline/01-01-SUMMARY.md` — workspace scaffold details
5. `.planning/phases/01-foundation-ci-dmg-pipeline/01-02-SUMMARY.md` — lints + cargo-deny + arch-lint details
6. `.planning/phases/01-foundation-ci-dmg-pipeline/01-03-SUMMARY.md` — threading skeleton + AppKit window + menu + overlay details
7. `.planning/phases/01-foundation-ci-dmg-pipeline/01-04-SUMMARY.md` — xtask DMG pipeline + CalVer release subcommand + Wave-0 cargo-bundle spike details (incl. brew/cargo-install prereqs hand-off to Plan 01-05's CI YAML)
8. `.planning/phases/01-foundation-ci-dmg-pipeline/01-05-SUMMARY.md` — `.github/workflows/ci.yml` 7-job PR-vs-push DAG + Pitfall-3 belt-and-braces + Outstanding Verification Debt (first-real-CI-run telemetry deferred); hand-off block enumerates the 7 required-status-check job names Plan 01-06 must register in branch protection.
9. `.planning/phases/01-foundation-ci-dmg-pipeline/01-06-SUMMARY.md` — release.yml + README install block + CHANGELOG seed + 6 MADR ADRs (0001..0006 documenting D-01..D-35) + docs/setup.md branch-protection guide; D-26 closed at the artifact level (xattr literal byte-identical across 4 surfaces); reconciles Plan 01-05's 7-check hand-off down to 4 PR-reachable checks per CONTEXT D-34; Outstanding Verification Debt for branch-protection state + first-tagged-release run; Phase 1 close-out hand-off block enumerates 4 cross-plan integrity invariants the phase verifier should re-check.

---
*State initialized: 2026-05-10*
*Plan 01-04 completed: 2026-05-10*
*Plan 01-05 completed: 2026-05-10 (committed locally; user pushes asynchronously)*
*Plan 01-06 completed: 2026-05-10 (committed locally 4dd0c4e + 75b77b1; user-approved checkpoint no-action; branch protection + first tagged release deferred to user's async push)*
*Phase 1 implementation complete: 2026-05-10 — verifier runs next*
