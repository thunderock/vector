# Changelog

All notable changes to Vector are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow CalVer (`YYYY.MM.DD`).
## [2026.05.10] - 2026-05-11

### Added
- Land cliff.toml + xtask release subcommand (CalVer, no push) (5df48f6)
- Land xtask separate workspace + dmg subcommand + cargo-bundle metadata (d247be1)
- Land threading skeleton + AppKit window + menu + overlay (25c59e3)
- Wire vector-app deps + build.rs + resources (a0d9027)
- Lock workspace lints, cargo-deny policy, and cargo-husky hook (14ecd78)
- Scaffold 14 crate stubs — workspace compiles clean (3d37c3a)
- Scaffold cargo workspace root + toolchain pin + xtask alias (e1b5b40)


### CI
- Land release.yml + README install block + CHANGELOG seed (4dd0c4e)
- Land GitHub Actions CI workflow (506b6bb)


### Documentation
- Complete phase execution + evolve PROJECT.md (fddaccc)
- Complete release pipeline + docs plan — approved no-action, GitHub UI deferred (abdd46b)
- Land 6 MADR ADRs + setup.md branch-protection guide (75b77b1)
- Complete CI pipeline plan — approved no-push, CI telemetry deferred (2f2d773)
- Complete xtask DMG pipeline plan — Wave-0 spike approved (326ff1d)
- Complete threading-skeleton + AppKit-window + menu + overlay plan (889210b)
- Pause state after Wave 2 — resume on macOS for Waves 3-6 (fd4bb80)
- Complete workspace-lints/cargo-deny/architecture-lint plan (c1119e3)
- Update tracking after wave 1 (b6cd2cb)
- Complete workspace scaffolding plan (4b3821c)
- Plan revision pass 1 — address 3 plan-checker warnings (9211b2f)
- Plan phase 1 (foundation + ci/dmg pipeline) (5df67fa)
- Research + validation strategy (0fec3ac)
- Mark UI-SPEC approved (084268f)
- UI design contract (40cc865)
- Add gsd state frontmatter after phase 1 context (7e9f104)
- Capture phase context (152670a)
- Create roadmap (10 phases, 51 requirements) (7353daf)
- Define v1 requirements (69a53e8)
- Add project research (8a7c87d)
- Initialize project (9074b09)


### Fixed
- Apply cargo fmt + mute 3 pedantic lints for stub scaffolding (cd05f27)


### Internal
- Pause state at Wave-0 cargo-bundle universal-DMG checkpoint (5dc6102)
- Merge executor worktree (worktree-agent-aff462b3a028e4bfd) (50c7631)
- Merge executor worktree (worktree-agent-a96f97392139113cf) (b384294)
- Remove stale submodule references for ghostty and vscode (40a8554)
- Add project config (b4b775d)


### Tests
- Add per-crate architecture-lint test no_tokio_main.rs (e3fb5df)

