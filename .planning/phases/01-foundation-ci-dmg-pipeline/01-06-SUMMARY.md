---
phase: 01-foundation-ci-dmg-pipeline
plan: 06
subsystem: infra
tags: [github-actions, release-yml, readme, changelog, madr, adr, branch-protection, git-cliff, xattr, d-21, d-26, d-27, d-29, d-30, d-33, d-34, d-35, calver, conventional-commits, macos-15-intel]

requires:
  - "01-04-SUMMARY (xtask separate workspace + `cargo xtask dmg --universal --arm64 PATH --x86_64 PATH` + `cargo xtask release` CalVer push-free flow + cliff.toml at repo root — release.yml's package job reuses the same xtask invocation form and brew/cargo-install prereqs)"
  - "01-05-SUMMARY (`.github/workflows/ci.yml` 7-job DAG — pinned the 7 required-status-check job names for branch protection: lint, commitlint, test, deny, build-arm64, build-x86_64, package; release.yml mirrors ci.yml's env block + Pitfall-3 guards + xattr footer to close D-26 place 3 of 3)"

provides:
  - ".github/workflows/release.yml — 124-line tag-triggered Universal DMG publish workflow. Trigger: `on: push: tags: ['v*']`. Three jobs: build-arm64 (macos-14) + build-x86_64 (macos-15-intel) → release (macos-14, needs both, permissions: contents: write). Release job downloads per-arch artifacts, runs Pitfall-3 guards before merge (`file artifacts/{arch}/vector-app | grep -q '{arch}'`) AND after merge (`lipo -info Vector.app/Contents/MacOS/vector-app | grep arm64; ... | grep x86_64`), brews `create-dmg librsvg git-cliff` + installs `cargo-bundle@0.10.0 --locked`, invokes `cargo xtask dmg --universal --arm64 PATH --x86_64 PATH`, runs `git-cliff --latest -o RELEASE_NOTES.md`, appends xattr install footer (D-26 place 3 of 3), publishes via `gh release create \"${{ github.ref_name }}\" --title \"Vector ${{ github.ref_name }}\" --notes-file RELEASE_NOTES.md target/dmg/Vector-*-universal.dmg`."
  - "README.md install block (D-26 place 2 of 3) — 35 lines replacing the prior single-line stub. H1 + tagline + ## Install section (4 numbered steps: download DMG, drag to /Applications, run `xattr -dr com.apple.quarantine /Applications/Vector.app` in a fenced `sh` code block, open Vector) + ## Status + ## Build from source + ## License. The 'unsigned app' phrase appears explicitly so the xattr step doesn't feel arbitrary."
  - "CHANGELOG.md seed — Keep a Changelog header + CalVer reference + `## [Unreleased]` section. Compatible with cliff.toml's template (Plan 01-04) so `cargo xtask release` → `git-cliff -t v{date} -o CHANGELOG.md` appends version sections cleanly on first run."
  - "Six MADR-format ADRs under docs/adr/ documenting D-01..D-35: 0001-rust-workspace-layout (D-01..D-04), 0002-winit-tokio-threading (D-08..D-11; tags include WIN-05 + pitfall-5), 0003-architecture-lint-mechanism (D-08 per-crate test mechanism + CI grep redundancy), 0004-dmg-pipeline (D-17..D-26; tags BUILD-02..05), 0005-versioning-calver (D-27..D-30), 0006-runner-labels-and-branch-protection (D-21 amendment with macos-15-intel + August 2027 EOL warning, D-34..D-35 branch protection setup)."
  - "docs/setup.md — 88-line one-time manual configuration guide with 6 sections (local dev tools, Rust toolchain, GitHub branch protection, cargo-husky hooks, first build/DMG, first release). The branch protection section enumerates the 4 PR-required status check names (lint, commitlint, test, deny) and includes the `gh api repos/colligo/vector/branches/main/protection` verification command."

affects:
  - "Phase 01 close-out: this is the final plan in Phase 1. The phase verifier inherits four cross-plan integrity invariants to re-check (see 'Phase 01 Close-out Hand-off' below): (1) the seven required-status-check job names match between ci.yml and the branch-protection setup in docs/setup.md, (2) the xattr literal `xattr -dr com.apple.quarantine /Applications/Vector.app` matches byte-identically across all D-26 surfaces (README install, ci.yml tip-release body, release.yml release body, DMG background PNG), (3) ADR 0006 captures both the D-21 amendment (macos-15-intel replacing the retired Intel runner, Aug 2027 EOL) and the D-34/D-35 branch-protection setup, (4) release.yml uses the same env block, lipo-info guards, and xtask invocation form as ci.yml so a single refactor never desynchronizes the two."
  - "Phase 2 (Headless Terminal Core): inherits a frozen ADR-documented architecture surface — the per-crate `tests/no_tokio_main.rs` lint mechanism (ADR 0003) is the seam Phase 2's new crates must mirror; CalVer + Conventional Commits (ADR 0005) is the commit-message contract Phase 2 must honour; the 14-crate workspace layout (ADR 0001) is the addition pattern. Phase 2's planner can read the ADRs to learn 'why the workspace is this shape' before adding crates."
  - "Phase 10 (Hardening & Release): inherits release.yml as the v1.0.0-tag publish path. Apple Developer ID signing + notarization (DIST-V2-01) plugs into the release job between `cargo xtask dmg --universal` and `gh release create`; Sparkle auto-update (DIST-V2-02) reuses the tagged-release asset URL pattern."

tech-stack:
  added:
    - "GitHub Actions release workflow on `v*` tag push — 3-job DAG matching ci.yml's matrix-then-merge topology. Same runner labels (macos-14 arm64, macos-15-intel x86_64), same dtolnay/rust-toolchain@1.88.0 pin, same Swatinem/rust-cache@v2 shared-keys (rel-arm64, rel-x86_64). Difference vs. ci.yml: tag-trigger only; no PR path; permissions block scoped to `contents: write` on the release job for `gh release create`."
    - "MADR 4.0 ADR template — accepted/Date/Deciders/Tags header + 5 sections (Context and Problem Statement, Decision Drivers, Considered Options, Decision Outcome, Consequences). Each ADR ≤80 lines per CLAUDE.md `succinct comments` rule; 1–3 sentences per section, references CONTEXT.md decision IDs (D-XX) by number rather than re-stating rationale."
    - "Keep a Changelog header + git-cliff compatibility for CHANGELOG.md — file scaffolded with H1 + `## [Unreleased]` so cliff.toml's body template appends version sections above the Unreleased marker without conflict on next `cargo xtask release`."
  patterns:
    - "Pattern (sibling-workflow release.yml ↔ ci.yml without shared YAML): release.yml is a sibling, not a reusable workflow `uses:` reference. The two files share env vars, prereqs, and Pitfall-3 guards by convention (literal copy) — meant to be readable end-to-end without `gh actions` round-trips. Lock-step rule: a refactor of either env block, lipo-info guard, or xtask invocation form must update both files in the same commit, or the two pipelines drift."
    - "Pattern (xattr footer appended via heredoc to git-cliff output): git-cliff produces RELEASE_NOTES.md; the release job appends a fixed install footer via `cat >> RELEASE_NOTES.md <<'EOF' ... EOF` before `gh release create`. Two guarantees: (1) the xattr instruction is present even if git-cliff produces an empty body for a no-changes tag (D-26 closure unconditional on commit history), (2) the `${{ github.ref_name }}` and `${{ github.sha }}` template tokens are interpolated by GitHub Actions before the heredoc lands in the file."
    - "Pattern (MADR succinctness over MADR completeness): each ADR uses the 5-section MADR template but keeps each section to 1–3 sentences and defers full rationale to the referenced CONTEXT.md decision IDs (D-XX). The ADRs are durable architecture trail markers (~50–65 lines each), not full design docs. Future readers follow the D-XX trail back to the locked CONTEXT for the verbose rationale."
    - "Pattern (push-free authoring per CLAUDE.md): this plan authored release.yml + README + CHANGELOG + 6 ADRs + setup.md locally and committed in two atomic commits (4dd0c4e, 75b77b1) without pushing. The branch protection configuration + first tagged release are the user's asynchronous responsibility; both are surfaced in 'Outstanding Verification Debt' below for `/gsd:progress` and `/gsd:audit-uat` to chase. Same disposition pattern as Plans 01-04 (Wave-0 spike approved-no-telemetry) and 01-05 (CI authored-not-pushed)."

key-files:
  created:
    - ".github/workflows/release.yml (124 lines) — 3 jobs (build-arm64 on macos-14, build-x86_64 on macos-15-intel, release on macos-14 needing both). Env block: MACOSX_DEPLOYMENT_TARGET=\"13.0\", CARGO_HUSKY_DONT_INSTALL_HOOKS=\"1\", RUST_BACKTRACE=short. Permissions: contents: write on release job only. Pitfall-3 belt-and-braces (file before merge + lipo-info after merge). Brew + cargo install prereqs match ci.yml. xattr install footer appended to RELEASE_NOTES.md before `gh release create`."
    - "README.md (35 lines — replaces 1-line stub) — H1, tagline, ## Install with 4 numbered steps (xattr in fenced `sh` block), ## Status, ## Build from source, ## License (MIT). 'unsigned app' phrase makes the xattr step intentional rather than mysterious."
    - "CHANGELOG.md (9 lines) — Keep a Changelog header, CalVer reference, `## [Unreleased]` section. cliff.toml-compatible scaffold for git-cliff to populate on next tagged release."
    - "docs/adr/0001-rust-workspace-layout.md (50 lines) — MADR for D-01..D-04 (14-crate workspace, [workspace.dependencies] pin once, xtask separate workspace)."
    - "docs/adr/0002-winit-tokio-threading.md (60 lines) — MADR for D-08..D-11 (winit on main + dedicated tokio I/O thread + EventLoopProxy::send_event + clippy::await_holding_lock=deny). Tags: WIN-05, pitfall-5."
    - "docs/adr/0003-architecture-lint-mechanism.md (50 lines) — MADR for D-08 per-crate `tests/no_tokio_main.rs` + CI grep redundancy (Plan 01-05 ci.yml). Explicitly notes xtask is exempt because it's in a separate workspace."
    - "docs/adr/0004-dmg-pipeline.md (63 lines) — MADR for D-17..D-26 (lipo + cargo-bundle + create-dmg + tip-and-tagged release pattern). Notes xattr appears in 4 places (one more than D-26's 3, harmless redundancy)."
    - "docs/adr/0005-versioning-calver.md (52 lines) — MADR for D-27..D-30 (CalVer YYYY.MM.DD + Conventional Commits + git-cliff CHANGELOG + convco enforcement)."
    - "docs/adr/0006-runner-labels-and-branch-protection.md (65 lines) — MADR for D-21 amendment (macos-15-intel replacing retired Intel runner, **August 2027 EOL warning**) AND D-34/D-35 branch protection (required status checks: lint, commitlint, test, deny; linear history; force-push disabled; PR review = 0 reviewers)."
    - "docs/setup.md (88 lines) — 6-section one-time manual setup guide. Section 3 'GitHub branch protection' enumerates the 4 PR-required check names (lint, commitlint, test, deny) verbatim from ci.yml; `gh api repos/colligo/vector/branches/main/protection` verification command included."
  modified: []

key-decisions:
  - "Branch protection PR-required checks limited to 4 (lint, commitlint, test, deny) — not the full 7 ci.yml job names. Rationale: build-arm64, build-x86_64, and package are gated by `if: github.event_name == 'push' && github.ref == 'refs/heads/main'` in ci.yml (Plan 01-05 D-17 design), so they never run on PRs and therefore can NEVER produce a status that branch protection could require. Listing them as required would deadlock PR merges. The CONTEXT-level D-34 'Universal-DMG build is intentionally NOT a required check' is the explicit guidance; ADR 0006 records the rationale; docs/setup.md and the Plan 01-06 frontmatter both enumerate only the 4 reachable checks. The Plan 01-05 hand-off block (`Plan 01-06 must register exactly these 7 required status checks`) is overstated for this reason — the 3 push-gated jobs cannot be required without changing the PR-vs-push DAG design, which would burn ~10 min of macOS runner minutes per PR per D-17."
  - "xattr footer appended to RELEASE_NOTES.md via heredoc rather than baked into cliff.toml's body template. Rationale: cliff.toml's body template applies to every release, but the install instructions logically belong on the tagged-release surface only (the tip-release body in ci.yml already has them inline; adding them to cliff.toml would double-print in tagged releases since release.yml uses cliff.toml AND appends). The heredoc append is local to release.yml and keeps the cliff.toml template stable for any future use case (e.g. generating a standalone CHANGELOG.md without install instructions)."
  - "ADRs use the MADR 4.0 5-section template but keep each section to 1–3 sentences per the CLAUDE.md `succinct comments` rule. Rationale: ADRs are durable trail markers for future-self / future-contributor onboarding; the full rationale lives in CONTEXT.md (D-XX decision IDs) which is the locked source of truth. The ADR's job is 'name the decision + name the alternatives + name the consequence', not 're-state the rationale'. Each ADR is ≤80 lines (verified: max is 65 lines for ADR 0006). The MADR template structure (headers) is doc structure, not prose comments — CLAUDE.md's anti-multi-paragraph rule applies to prose, not to the 5 MADR section headers."
  - "Push-free authoring per CLAUDE.md `do not push` — the two task commits (4dd0c4e, 75b77b1) landed locally; the human-action checkpoint (configure branch protection + cut first tagged release) was approved by the user WITHOUT touching the GitHub UI this session. Neither branch protection state nor the first tagged release was observed. Same disposition as Plans 01-04 (Wave-0 telemetry not captured) and 01-05 (first-real-CI-run not observed). The verification debt is surfaced explicitly so `/gsd:progress` and `/gsd:audit-uat` can chase it on the user's next push."

patterns-established:
  - "Pattern (ADRs ≤80 lines as trail markers, not design docs): each ADR uses the MADR 5-section structure but each section is 1–3 sentences. The verbose rationale lives in the locked CONTEXT.md decision IDs (D-XX). Future ADRs in Phases 2–10 should follow this length budget (50–80 lines max) and reference the locked decision-ID source of truth rather than restating it."
  - "Pattern (release.yml ↔ ci.yml sibling, not reusable): no `uses:` reusable-workflow indirection in Phase 1. The two workflows share env block, brew prereqs, cargo-install lines, Pitfall-3 guard pairs, and xtask invocation form by literal copy. A refactor of one MUST update the other in the same commit (or the pipelines drift). Phase 10 (or a future PR-rate-of-change moment) may consolidate via a `.github/actions/setup-vector-mac` composite action; deferred until justified by maintenance pain."
  - "Pattern (xattr literal in 4 places for D-26 belt-and-braces): README ## Install (this plan), DMG background PNG (Plan 01-04), ci.yml tip-release body (Plan 01-05), release.yml tagged-release body (this plan). D-26 specifies 3; the 4th is harmless redundancy and improves discoverability. Phase verifier should grep all 4 for byte-identical `xattr -dr com.apple.quarantine /Applications/Vector.app` (single-space, lowercase, /Applications path)."
  - "Pattern (approved-no-action human-action checkpoint disposition): when a `checkpoint:human-action` requires GitHub UI / API state changes that the user explicitly declines to perform this session (per CLAUDE.md `do not push`), the executor accepts the approval as the resume signal and documents the unfulfilled state-change as Outstanding Verification Debt. Same shape as Plans 01-04 Wave-0 spike telemetry and 01-05 first-real-CI-run. The plan is not blocked; the work is surfaced for future closure."

requirements-completed: [BUILD-04, BUILD-05, BUILD-02]

duration: ~2 implementation commits (4dd0c4e + 75b77b1) + 1 human-action checkpoint approved without GitHub UI action (~45 min authoring + verification + state housekeeping)
completed: 2026-05-10
---

# Phase 01 Plan 06: Release Pipeline + README + ADRs + Branch Protection Summary

**`.github/workflows/release.yml` (tag-triggered Universal DMG publish with xattr footer in release body — D-26 place 3 of 3), 35-line README install block (D-26 place 2 of 3), 6 MADR-format ADRs (0001..0006) covering D-01..D-35 plus the D-21 macos-15-intel runner amendment with August 2027 EOL warning, and docs/setup.md branch-protection guide enumerating the 4 PR-required status check names. Two task commits landed; the terminal human-action checkpoint (configure GitHub branch protection + cut first tagged release) was user-approved without GitHub UI action per CLAUDE.md `do not push` — branch protection state and first tagged release were NOT observed this session; surfaced as Outstanding Verification Debt for `/gsd:progress` and `/gsd:audit-uat` to chase on the user's asynchronous push.**

## Disposition

**User-approved without GitHub UI action.** The user replied `approved` to the human-action checkpoint with the explicit caveat "no action" — neither the branch-protection configuration in the GitHub UI nor `cargo xtask release` + `git push --follow-tags` was performed this session. Same disposition pattern as Plans 01-04 (Wave-0 spike approved-no-telemetry) and 01-05 (CI authored-not-pushed). The local task work is complete and committed (4dd0c4e + 75b77b1); the remote-state verification of `gh api repos/colligo/vector/branches/main/protection` JSON and `gh release view v2026.05.10` is deferred to the user's asynchronous push and documented as Outstanding Verification Debt below.

## Performance

- **Duration:** 2 implementation commits (Tasks 1 and 2) + 1 human-action checkpoint approved without action (~45 min finalization + state housekeeping in this resume agent's scope).
- **Task 1 commit:** `4dd0c4e` (ci — release.yml + README install block + CHANGELOG seed)
- **Task 2 commit:** `75b77b1` (docs — 6 MADR ADRs + setup.md branch-protection guide)
- **Task 3 (checkpoint):** user reply `approved (no action)` — no commit; resume signal only.
- **Tasks:** 2 implementation tasks + 1 human-action checkpoint (resolved no-action).
- **Files created:** 10 (.github/workflows/release.yml, README.md replacement, CHANGELOG.md, 6 ADRs, docs/setup.md).
- **Files modified:** 0 (README.md was replaced as a near-net-new file rather than incrementally edited; the prior single-line stub had no install content to preserve).

## Pre-checkpoint Local Verification (what DID happen)

The following local verifications passed before the human-action checkpoint was raised; each is re-verified by this resume agent:

- `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"` exits 0 — release.yml YAML parses.
- `cargo fmt --all -- --check` exits 0 — no Rust files touched this plan.
- All Task 1 grep assertions pass (15/15): `tags: ['v*']`, `macos-15-intel`, NO `macos-13`, `cargo xtask dmg --universal`, `xattr -dr com.apple.quarantine /Applications/Vector.app` in release.yml, `git-cliff --latest -o RELEASE_NOTES.md`, `gh release create`, `lipo -info`, README `# Vector` H1, README `xattr -dr com.apple.quarantine /Applications/Vector.app`, README `unsigned app` mention, CHANGELOG `# Changelog` header, CHANGELOG `Keep a Changelog` reference, CHANGELOG `## [Unreleased]` section.
- All Task 2 grep assertions pass: 6 ADR files present with all 5 MADR sections each, ADR 0002 references D-08..D-11, ADR 0006 references D-21 + `macos-15-intel` + `Aug 2027`, docs/setup.md contains `gh api repos/colligo/vector/branches/main/protection` + `required_linear_history`.
- All 6 ADRs ≤80 lines (verified line counts: 50, 60, 50, 63, 52, 65). The MADR 5-section template is doc structure (headers), not multi-paragraph prose comments — does not violate CLAUDE.md's `succinct comments` rule.
- D-26 xattr literal `xattr -dr com.apple.quarantine /Applications/Vector.app` lands byte-identically in 4 surfaces:
  1. DMG background PNG content (Plan 01-04 — verified via xtask/scripts/render-dmg-bg.sh contains the literal in script source).
  2. README.md ## Install section (this plan, line in fenced `sh` block).
  3. .github/workflows/ci.yml tip-release body (Plan 01-05).
  4. .github/workflows/release.yml tagged-release body (this plan, via heredoc appended to RELEASE_NOTES.md).
- No Rust comments / doc comments touched this plan; CLAUDE.md `succinct comments` rule trivially satisfied.

## Accomplishments

- `release.yml` lands the tagged-release half of BUILD-04 (per Plan 01-05 SUMMARY's hand-off: tip-release was Plan 01-05; tagged-release is this plan). 3-job DAG: build-arm64 on macos-14 → build-x86_64 on macos-15-intel → release on macos-14 (downloads both per-arch artifacts, runs Pitfall-3 guards before + after merge via `lipo -info`, invokes `cargo xtask dmg --universal`, runs `git-cliff --latest -o RELEASE_NOTES.md`, appends xattr install footer via heredoc, publishes via `gh release create`).
- D-26 closure: xattr instruction now byte-identical across 4 surfaces (README, DMG bg PNG, ci.yml tip body, release.yml tag body). Single-source-of-truth is the verbatim `xattr -dr com.apple.quarantine /Applications/Vector.app` literal; a global rename of /Applications would require updating all 4 places in lock-step.
- README transitioned from a 1-line stub to a 35-line install-and-build guide. ## Install is 4 numbered steps with xattr in a fenced `sh` block; ## Build from source documents the brew/cargo prereqs; ## Status sets reader expectations (Phase 1 early bootstrap). No status badges above H1 (intentional; too early).
- CHANGELOG.md seeded with the Keep a Changelog header + `## [Unreleased]` so cliff.toml's body template appends version sections cleanly on the next `cargo xtask release` invocation.
- 6 MADR ADRs document D-01..D-35 (workspace layout, threading, architecture-lint, DMG pipeline, CalVer, runner-labels-and-branch-protection). Each ADR ≤80 lines, references CONTEXT.md decision IDs by number. ADR 0006 records the D-21 amendment with the **August 2027** Intel-runner EOL warning so the calendar reminder is on the durable trail.
- docs/setup.md captures the one-time manual configuration steps: xcode-select, brew installs (create-dmg + librsvg + git-cliff + imagemagick), cargo subcommand installs (cargo-bundle@0.10.0, cargo-deny), rustup verification, **GitHub branch protection setup with the 4 PR-required status check names** (lint, commitlint, test, deny), cargo-husky hook installation, first build + DMG, first release.

## Task Commits

1. **Task 1: release.yml + README install block + initial CHANGELOG** — `4dd0c4e` (ci)
2. **Task 2: ADRs 0001..0006 (MADR template) + docs/setup.md** — `75b77b1` (docs)
3. **Task 3 (checkpoint:human-action): configure GitHub branch protection + cut first tagged release** — resolved by user reply `approved (no action)`; no commit; GitHub UI not touched this session.

Plan metadata commit follows this SUMMARY (covers `01-06-SUMMARY.md`, `STATE.md`, `ROADMAP.md`, `REQUIREMENTS.md`).

## Files Created/Modified

### Created (10)

- `.github/workflows/release.yml` — 124 lines, 3 jobs, tag-triggered (`on: push: tags: ['v*']`). Permissions: contents: write on release job. Pitfall-3 belt-and-braces, xattr install footer appended to RELEASE_NOTES.md before `gh release create`.
- `README.md` — 35 lines (replaces 1-line stub). H1 + tagline + ## Install (xattr in fenced `sh`) + ## Status + ## Build from source + ## License.
- `CHANGELOG.md` — 9 lines. Keep a Changelog header + CalVer reference + ## [Unreleased].
- `docs/adr/0001-rust-workspace-layout.md` — 50 lines, MADR for D-01..D-04.
- `docs/adr/0002-winit-tokio-threading.md` — 60 lines, MADR for D-08..D-11.
- `docs/adr/0003-architecture-lint-mechanism.md` — 50 lines, MADR for D-08 lint mechanism.
- `docs/adr/0004-dmg-pipeline.md` — 63 lines, MADR for D-17..D-26.
- `docs/adr/0005-versioning-calver.md` — 52 lines, MADR for D-27..D-30.
- `docs/adr/0006-runner-labels-and-branch-protection.md` — 65 lines, MADR for D-21 amendment + D-34..D-35 branch protection.
- `docs/setup.md` — 88 lines, 6-section one-time manual setup guide.

### Modified (0)

None. README.md replacement is net-new content (the prior 1-line stub had no installable surface to preserve).

## Decisions Made

See `key-decisions` in the frontmatter for the four substantive decisions:

1. Branch protection PR-required checks limited to 4 (lint, commitlint, test, deny), NOT the full 7 ci.yml job names — the 3 push-gated jobs (build-arm64, build-x86_64, package) never run on PRs and would deadlock merges if required. ADR 0006 records the rationale; this is the documented reconciliation with Plan 01-05's overstated hand-off.
2. xattr footer appended via heredoc in release.yml rather than baked into cliff.toml — keeps cliff.toml clean for future reuse, avoids double-print in tagged releases.
3. ADRs use MADR 5-section template but each section is 1–3 sentences (CLAUDE.md succinct rule) with full rationale deferred to CONTEXT.md D-XX IDs. All 6 ADRs ≤80 lines.
4. Push-free authoring per CLAUDE.md — user-approved no-action; branch protection state + first tagged release deferred to user's async work; surfaced as verification debt.

## Deviations from Plan

### Reconciliation of Plan 01-05's overstated branch-protection hand-off

**1. [Rule 1 — Plan/CONTEXT mismatch, executor-reconciliation] Branch-protection setup names 4 PR-required checks, not 7**
- **Found during:** Task 2 (authoring docs/setup.md §3 branch protection).
- **Issue:** Plan 01-05's SUMMARY hand-off block says "Plan 01-06 must register exactly these 7 required status checks: lint, commitlint, test, deny, build-arm64, build-x86_64, package." But CONTEXT.md D-34 explicitly states "Universal-DMG build is intentionally NOT a required check (too slow for the PR feedback loop)." And ci.yml gates build-arm64, build-x86_64, and package behind `if: github.event_name == 'push' && github.ref == 'refs/heads/main'` so those 3 jobs literally cannot run on PRs and would therefore never produce a status that branch protection could require. Listing them as required would deadlock all PR merges. Plan 01-06's `<read_first>` for Task 2 quotes CONTEXT D-34 directly; the discrepancy is between 01-05's hand-off and CONTEXT.
- **Fix:** docs/setup.md §3 enumerates only the 4 PR-reachable required status checks (lint, commitlint, test, deny). ADR 0006 records the rationale ("Universal-DMG build is NOT a required check (too slow for the PR feedback loop, per D-34)"). The 3 push-gated jobs still run on every main-push and produce visible status (just not branch-protection-required).
- **Files modified:** None (docs/setup.md and ADR 0006 both authored with the 4-check version; this deviation is the reconciliation between Plan 01-05's hand-off prose and CONTEXT D-34).
- **Verification:** `grep -c 'lint\|commitlint\|test\|deny' docs/setup.md` matches the 4 checks; `grep -q 'Universal-DMG build is intentionally NOT' docs/adr/0006-*.md` matches.
- **Committed in:** `75b77b1` (Task 2 commit).

### Wave-0-style verification debt on the terminal human-action checkpoint (mirrors 01-04 + 01-05 pattern)

**2. [Note — not a code deviation] User approval landed without GitHub UI / API action**
- **Found during:** Task 3 (checkpoint:human-action — configure branch protection + cut first tagged release).
- **Issue:** The checkpoint's Steps 1–4 asked the user to: (1) configure branch protection in the GitHub UI per docs/setup.md §3, (2) verify via `gh api repos/colligo/vector/branches/main/protection` and capture the JSON, (3) run `cargo xtask release` + `git push --follow-tags` to cut the first tagged release, (4) smoke-test the downloaded DMG (mount, drag-install, run xattr, launch). The user replied `approved` with the explicit "no action" caveat — the GitHub UI was not touched, `cargo xtask release` was not run, and the first tagged release was not observed.
- **Fix:** None applicable — the user's approval is the resume signal per the checkpoint's `<resume-signal>` clause. The no-action disposition is consistent with CLAUDE.md `do not push` and with the same disposition Plans 01-04 (Wave-0 telemetry) and 01-05 (first-real-CI-run) took. The pre-checkpoint local verification (15/15 Task 1 grep assertions + 6 MADR section checks per ADR + 6 line-count checks + cargo fmt --check + YAML parses + xattr literal byte-identity across 4 surfaces) is the evidence on file.
- **Impact on plan:** No code impact. The verification debt for branch-protection state + first tagged release is surfaced in the "Outstanding Verification Debt" block below so `/gsd:progress` and `/gsd:audit-uat` can chase it on the user's next push.
- **Committed in:** Documented here in 01-06-SUMMARY.md.

---

**Total deviations:** 1 plan-vs-CONTEXT reconciliation (docs/setup.md authored with the 4 PR-reachable checks per CONTEXT D-34, deviating from Plan 01-05's overstated hand-off of 7) + 1 verification-debt note (user-approved without GitHub UI action; deferred to user's async push).
**Impact on plan:** Acceptance criteria met for release.yml content, README install block, CHANGELOG seed, 6 MADR ADRs, and docs/setup.md. The verification debt means BUILD-02 and BUILD-04 remain in the "implemented and locally verified, pending first-real-CI-run AND first-real-tagged-release confirmation" state. BUILD-05 is fully complete (README contains the xattr instruction in a fenced sh block; D-26 is closed at the artifact level — the 4 surfaces all carry the literal byte-identically).

## Issues Encountered

None during planned task execution. Tasks 1 and 2 landed cleanly in the prior agent's session (commits 4dd0c4e and 75b77b1); the checkpoint was approved without action by the user. The only work in this resume agent's scope was authoring this SUMMARY, updating STATE / ROADMAP / REQUIREMENTS, and running the metadata commit. No lint failures detected (`cargo fmt --check` exits 0; no project-level markdown lint configured per CLAUDE.md discovery order — Makefile, justfile, CI workflows, pre-commit-config.yaml all checked, no markdown linter found).

## Outstanding Verification Debt

The first real branch-protection configuration and first tagged release were NOT exercised this session. When the user is ready to push (asynchronously per CLAUDE.md), they should walk Plan 01-06 Task 3's `<how-to-verify>` Steps 1–4 and capture:

1. **Branch protection configured** on `main` per `docs/setup.md` §3 with the 4 required-status-check names verbatim (`lint`, `commitlint`, `test`, `deny`). Verified via the GitHub Settings → Branches UI showing the rule active.
2. **`gh api` JSON inspection** — `gh api repos/colligo/vector/branches/main/protection` should report:
   - `required_status_checks.contexts: [lint, commitlint, test, deny]` (or a permutation; order is not part of the contract)
   - `required_linear_history.enabled: true`
   - `allow_force_pushes.enabled: false`
   - `allow_deletions.enabled: false`
3. **First tagged release exercised:** `cargo xtask release` (CalVer bump + git-cliff + commit + tag, no push per Plan 01-04 design) → `git push --follow-tags` → `release.yml` runs to completion. Watch via `gh run watch`.
4. **`gh release view v2026.05.10`** (or whatever today's CalVer resolves to at push time) shows:
   - Asset `Vector-{CalVer}-universal.dmg` attached
   - Body contains the literal `xattr -dr com.apple.quarantine /Applications/Vector.app` in a fenced `sh` code block
   - Built-from-commit SHA matches the tagged commit
5. **Downloaded DMG smoke-test:** mount → drag Vector.app to /Applications → `xattr -dr com.apple.quarantine /Applications/Vector.app` → double-click → confirm the Plan 01-03 visual appears (window, ticking title, version overlay, native menu bar).

If any of (1)–(5) fail, surface as a Plan 01-06 follow-up issue (deferred-items.md or a new plan in Phase 1). If all pass, the verification debt is closed and BUILD-02 / BUILD-04 / BUILD-05 move from "implemented and locally verified" to "regression-gated by live CI + first-tagged-release".

This block is the source of truth for `/gsd:progress` and `/gsd:audit-uat` to chase. It supersedes 01-05's verification debt only for the tagged-release half; the first-real-CI-run telemetry items from 01-05 remain outstanding in parallel.

## Phase 01 Close-out Hand-off (this is the final plan in phase 01)

Phase 01 verification follows this commit. The phase verifier should re-check these four cross-plan integrity invariants:

1. **Required-status-check name matching:** the 4 PR-reachable job names (`lint`, `commitlint`, `test`, `deny`) must match byte-identically between `.github/workflows/ci.yml` (Plan 01-05) and `docs/setup.md` §3 (Plan 01-06). A future rename in ci.yml without a lock-step update to docs/setup.md silently no-ops branch protection. `grep -E '^  (lint|commitlint|test|deny):' .github/workflows/ci.yml` should match exactly 4 lines; `grep -E '^\s+- `?(lint|commitlint|test|deny)' docs/setup.md` should match the same 4 names.

2. **xattr literal byte-identity across all 4 D-26 surfaces:** `grep -h 'xattr -dr com.apple.quarantine /Applications/Vector.app' README.md .github/workflows/ci.yml .github/workflows/release.yml xtask/scripts/render-dmg-bg.sh | sort -u | wc -l` should report `1` — meaning the literal is byte-identical (single-space, lowercase, `/Applications` path) across all 4 surfaces. A drift in any of these 4 files (e.g., changing /Applications to /Applications/, or removing the space) breaks D-26.

3. **ADR 0006 captures both halves:** the D-21 amendment (macos-15-intel replacing the retired Intel runner, **August 2027** EOL warning) AND the D-34/D-35 branch-protection setup. Verifier should `grep -q 'macos-15-intel' docs/adr/0006-*.md && grep -q 'August 2027\|Aug 2027' docs/adr/0006-*.md && grep -q 'D-34\|D-35\|branch protection' docs/adr/0006-*.md`.

4. **release.yml ↔ ci.yml shared invariants:** both workflows MUST share the same env block (CARGO_HUSKY_DONT_INSTALL_HOOKS="1", MACOSX_DEPLOYMENT_TARGET="13.0", RUST_BACKTRACE=short), the same Pitfall-3 lipo-info guards, the same brew prereqs (create-dmg + librsvg), the same cargo-install pin (cargo-bundle@0.10.0 --locked), and the same `cargo xtask dmg --universal` invocation form. A refactor of one without a matching update to the other constitutes drift; the phase verifier should diff the relevant blocks.

**No further work in Phase 1 plans.** The next agent invocation should be the phase regression gate + verifier + ROADMAP / Phase-Map close-out — handled by the orchestrator, not this executor.

## User Setup Required

**One-time, when the user is ready to push (asynchronously per CLAUDE.md `do not push`):**

```sh
# Push the local commits (including 4dd0c4e + 75b77b1) and 01-05's 506b6bb + 2f2d773 to GitHub.
git push origin master

# Configure branch protection per docs/setup.md §3 in the GitHub UI:
#   https://github.com/colligo/vector/settings/branches
# Add rule for `main`:
#   - Require status checks: lint, commitlint, test, deny
#   - Require linear history
#   - Disallow force pushes
#   - Disallow deletions

# Verify the rule landed:
gh api repos/colligo/vector/branches/main/protection

# Cut the first tagged release:
cargo xtask release
git push --follow-tags

# Watch the release run:
gh run watch

# Confirm the published release + asset:
gh release view v2026.05.10
```

After the first push, the user should walk the "Outstanding Verification Debt" checklist above and report results back so the debt can be closed (or open follow-up issues if anything fails).

**No external services or secrets required for this plan** — release.yml uses only the auto-provided `GITHUB_TOKEN` (scoped to `contents: write` on the release job for `gh release create`). No third-party tokens.

## Next Phase Readiness

- **Phase 1 closure:** 6 of 6 plans now complete (01-01 through 01-06). Phase verifier runs next (handled by the `/gsd:execute-phase` orchestrator, not this executor).
- **Phase 1 success criteria status (from ROADMAP):**
  - Criterion 1 (push triggers CI → downloadable Vector.dmg artifact): **implementation complete (Plan 01-05), pending first-real-CI-run confirmation per 01-05 Outstanding Verification Debt.**
  - Criterion 2 (tagged release → unsigned Universal DMG to GitHub Releases with xattr instructions in README): **implementation complete (this plan: release.yml + README install block + xattr footer), pending first-tagged-release confirmation per 01-06 Outstanding Verification Debt.**
  - Criterion 3 (`cargo xtask dmg` locally produces identical DMG): **complete** (Plan 01-04, Wave-0 spike approved on macOS).
  - Criterion 4 (winit/tokio threading + architecture lint): **complete** (Plans 01-02 + 01-03; CI redundancy in Plan 01-05; ADR 0002 + 0003 document the pattern in this plan).
- **Phase 2 readiness:** ready to start. Inherits 14-crate workspace + threading skeleton + architecture-lint pattern + CI infrastructure + ADR practice (per ADR 0001..0005). Phase 2 planner should read ADRs 0001 (workspace shape), 0002 (threading), 0003 (lint mechanism) before adding new crates.

## Verification Checklist

- [x] `.github/workflows/release.yml` exists on disk (124 lines).
- [x] YAML parses cleanly via `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"`.
- [x] `grep -q "tags: \['v\*'\]" .github/workflows/release.yml` matches.
- [x] `grep -q 'macos-15-intel' .github/workflows/release.yml` matches; `grep -q 'macos-13' .github/workflows/release.yml` returns nothing.
- [x] `grep -q 'cargo xtask dmg --universal' .github/workflows/release.yml` matches.
- [x] `grep -q 'xattr -dr com.apple.quarantine /Applications/Vector.app' .github/workflows/release.yml` matches.
- [x] `grep -q 'git-cliff --latest -o RELEASE_NOTES.md' .github/workflows/release.yml` matches.
- [x] `grep -q 'gh release create' .github/workflows/release.yml` matches.
- [x] `grep -q 'lipo -info' .github/workflows/release.yml` matches.
- [x] `grep -q 'permissions:' .github/workflows/release.yml && grep -q 'contents: write' .github/workflows/release.yml` matches.
- [x] `README.md` exists, ≥20 lines (35 lines), contains `# Vector` H1, `xattr -dr com.apple.quarantine /Applications/Vector.app` in a fenced `sh` block, `unsigned app` literal.
- [x] `CHANGELOG.md` exists with `# Changelog` header, `Keep a Changelog` reference, `## [Unreleased]` section.
- [x] All 6 ADRs exist under `docs/adr/` with all 5 MADR sections each.
- [x] ADR 0002 references `D-08`/`D-09`/`D-10`/`D-11`.
- [x] ADR 0006 references `D-21`, contains `macos-15-intel`, mentions `Aug 2027`.
- [x] All 6 ADRs ≤80 lines (verified: 50, 60, 50, 63, 52, 65).
- [x] `docs/setup.md` exists, contains `gh api repos/colligo/vector/branches/main/protection`, `required_linear_history`, the 4 PR-required check names (lint, commitlint, test, deny), `brew install create-dmg librsvg`, `cargo install cargo-bundle@0.10.0`.
- [x] D-26 xattr literal byte-identical across 4 surfaces (README, ci.yml, release.yml, xtask/scripts/render-dmg-bg.sh).
- [x] `cargo fmt --all -- --check` exits 0.
- [x] Commits `4dd0c4e` and `75b77b1` present on `master` (verified via `git log --oneline`).
- [ ] **PENDING (verification debt):** branch protection configured per docs/setup.md §3; `gh api repos/colligo/vector/branches/main/protection` JSON matches the expected shape; first tagged release publishes Vector-{CalVer}-universal.dmg with the xattr footer in body; downloaded DMG mounts + drag-installs + launches after `xattr -dr` de-quarantine. See "Outstanding Verification Debt" above.

## Self-Check: PASSED

- Files asserted present on disk: `.github/workflows/release.yml` (124 lines), `README.md` (35 lines), `CHANGELOG.md` (9 lines), `docs/adr/{0001..0006}-*.md` (50/60/50/63/52/65 lines respectively), `docs/setup.md` (88 lines). All 10 confirmed present.
- Commits asserted present (Bash `git log --oneline | grep`): `4dd0c4e` (Task 1 — ci(01-06): land release.yml + README install block + CHANGELOG seed), `75b77b1` (Task 2 — docs(01-06): land 6 MADR ADRs + setup.md branch-protection guide). Both confirmed present on `master`.
- YAML structural validation: `python3 -c "import yaml; yaml.safe_load(...)"` exits 0 on release.yml.
- 15 Task 1 grep assertions + 6 ADR MADR-section checks + 6 ADR line-count checks + 6 docs/setup.md grep assertions executed: all pass.
- `cargo fmt --all -- --check` exits 0 (no Rust files touched this plan).
- D-26 xattr literal byte-identity verified across 4 surfaces (README, ci.yml, release.yml, xtask/scripts/render-dmg-bg.sh) — single canonical form.
- The verification debt for branch-protection state + first-tagged-release run is documented as explicitly outstanding — it is NOT claimed as complete. BUILD-02 / BUILD-04 / BUILD-05 are marked complete with the pending-real-tagged-release caveat noted here and surfaced for `/gsd:progress` and `/gsd:audit-uat`.

## Addendum 2026-05-11: release.yml dual-trigger + first published release

Two operational expansions landed during the first push session:

1. **`release.yml` now triggers on BOTH `push: tags: ['v*']` and `release: published`.**
   This makes the GitHub UI's "Draft a new release → Publish" flow work
   alongside the CLI `git push --follow-tags` flow. The publish step
   detects whether the release already exists (`gh release view`) and
   either creates it (`gh release create`) or attaches assets to it
   (`gh release upload --clobber` + `gh release edit`). A `concurrency:`
   group keyed on the tag prevents wasted double-runs when the UI
   creates tag+release in one click (which fires both events). All
   three jobs check out `${{ github.event.release.tag_name || github.ref }}`
   so a release-event run uses the tag's commit, not the default branch.

2. **Branch protection remains a manual user step.** Setup was not
   performed this session. `docs/setup.md §3` is still the canonical
   procedure; the 4 PR-reachable required-status-check names (`lint`,
   `commitlint`, `test`, `deny`) match `ci.yml` job names exactly.

The first real tagged release (`v2026.5.10`) had a multi-attempt path
(SemVer error → A5 cargo-bundle quirk → A5 fallback added → DMG built
successfully). Final release-artifact pipeline:
`tag push` → `release.yml` → matrix build → lipo merge → cargo-bundle →
A5 post-process → Pitfall-3 guard → `gh release create v2026.5.10` with
`Vector-2026.5.10-universal.dmg` attached + xattr install footer in body.

ADRs 0004, 0005, 0006 carry the relevant amendments.

---
*Phase: 01-foundation-ci-dmg-pipeline*
*Completed: 2026-05-10; first tagged release validated 2026-05-11*
