---
phase: 01-foundation-ci-dmg-pipeline
plan: 05
subsystem: infra
tags: [github-actions, ci, yaml, macos-15-intel, lipo, cargo-bundle, cargo-deny, tip-release, conventional-commits, convco, swatinem-rust-cache, d-08, d-17, d-18, d-19, d-20, d-21, d-22, d-23, d-24, d-26, d-28, d-30, pitfall-3, pitfall-6]

requires:
  - "01-02-SUMMARY (workspace lints + cargo-deny + per-crate architecture-lint tests — the test job runs `cargo test --workspace --tests` against the 14 architecture-lint tests; the architecture-lint grep redundancy step belt-and-braces the same invariants)"
  - "01-04-SUMMARY (xtask separate workspace + `cargo xtask dmg --universal --arm64 PATH --x86_64 PATH` — the package job invokes this exact form; Wave-0 cargo-bundle spike already approved means no `--bin` post-process fallback needed in CI)"

provides:
  - ".github/workflows/ci.yml — single workflow file with 7 jobs forming the PR-vs-push DAG: PR fans out to { lint, commitlint, test, deny } and stops; push-to-main runs the full DAG { lint, commitlint, test, deny } → { build-arm64 on macos-14, build-x86_64 on macos-15-intel } → { package on macos-14, downloads both per-arch binaries, runs Pitfall-3 guards before + after merge via `lipo -info`, invokes `cargo xtask dmg --universal`, uploads the DMG as a 90-day-retention workflow artifact, then overwrites the pinned `tip` GitHub Release via `gh release delete tip --cleanup-tag || true; gh release create tip --prerelease ...`}"
  - "Architecture-lint grep redundancy step (D-08 belt-and-braces) using `rg -n --glob 'crates/**/*.rs' --glob '!crates/**/tests/no_tokio_main.rs' '#\\[tokio::main\\]|Builder::new_current_thread\\(\\)'` — fails the build if any production crate file re-introduces a forbidden tokio pattern. Pairs with the per-crate `tests/no_tokio_main.rs` file-count guard (a new `crates/vector-*` directory added without the architecture-lint test file fails CI immediately)."
  - "Pinned 7 required-status-check job names for Plan 01-06's branch protection: lint, commitlint, test, deny, build-arm64, build-x86_64, package. Any rename in ci.yml must update Plan 01-06's branch-protection setup script in lock-step (or branch protection silently no-ops)."
  - "Tip-release DMG naming convention (D-28): `Vector-{CalVer}-tip-{shortsha}-universal.dmg` — the package job renames `Vector-{CalVer}-universal.dmg` (produced by `cargo xtask dmg --universal`) before publishing so the tip release asset name carries the source commit for traceability."
  - "Conventional-commits enforcement on PRs via convco (D-30): `convco check ${{ github.event.pull_request.base.sha }}..HEAD` on PRs; tolerant `convco check HEAD~10..HEAD || true` on direct main pushes (deliberate trade-off for Phase 1 bootstrap commits). convco prebuilt binary installed via curl in ~5s."
  - "xattr-bypass instruction baked into the tip-release body (1 of D-26's 3 places — README is the second, tagged-release body is the third and lands in Plan 01-06)."

affects:
  - "01-06-PLAN (release pipeline + README + ADRs + branch protection): MUST register exactly these 7 required-status-check job names — lint, commitlint, test, deny, build-arm64, build-x86_64, package. release.yml on `v*` tag push reuses the same `cargo xtask dmg --universal` invocation and the same `brew install create-dmg librsvg` + `cargo install cargo-bundle@0.10.0` prereqs from this workflow. The xattr instruction lands in README + tagged-release body for D-26 closure."
  - "Phase 10 (Hardening & Release): inherits ci.yml as the test + perf gate seam — renderer snapshot tests + VT conformance corpus + perf gates plug into the existing `test` job (or sibling jobs that share the runner image + Swatinem cache key)."
  - "Every future phase: ci.yml is the source of truth for what ships. A push to main produces a downloadable Vector.dmg via the tip release; that artifact is the proof of every later phase's user-visible outcome on macOS."

tech-stack:
  added:
    - "GitHub Actions CI (`macos-14` for arm64 jobs, `macos-15-intel` for x86_64 — D-21 amendment encoded in ci.yml line 111; macos-13 retired 2025-12-04 per CONTEXT-CONSTRAINT-DRIFT)."
    - "EmbarkStudios/cargo-deny-action@v2 — supply-chain audit (`check advisories licenses bans sources`) on every PR and push."
    - "Swatinem/rust-cache@v2 — per-job shared-key caching (`ci-lint`, `ci-test`, `ci-build-arm64`, `ci-build-x86_64`) to keep warm builds in the 1-minute range."
    - "convco prebuilt macOS binary — installed via `curl -sSL https://github.com/convco/convco/releases/latest/download/convco-macos.zip` in the commitlint job (~5s)."
    - "dtolnay/rust-toolchain@1.88.0 — explicit toolchain pin matching rust-toolchain.toml. Used in every job that touches cargo."
  patterns:
    - "Pattern (PR-vs-push DAG with conditional jobs): build-arm64, build-x86_64, and package jobs all carry `if: github.event_name == 'push' && github.ref == 'refs/heads/main'` so PRs run only lint+commitlint+test+deny (D-17 minute conservation). Push-to-main runs the full DAG end-to-end."
    - "Pattern (Pitfall-3 belt-and-braces): the `lipo -info` Pitfall-3 guard runs in THREE places — (1) inside `xtask::dmg::dmg_universal` (Plan 01-04), (2) in CI before `cargo xtask dmg` merges them (`file artifacts/aarch64/vector-app | grep -q arm64; file artifacts/x86_64/vector-app | grep -q x86_64`), and (3) in CI after `cargo xtask dmg` produces the bundle (`lipo -info Vector.app/Contents/MacOS/vector-app | grep arm64; ... | grep x86_64`). A secretly-thin Universal cannot ship even if the xtask guard is accidentally removed in a future refactor."
    - "Pattern (architecture-lint redundancy at the CI seam): the per-crate `tests/no_tokio_main.rs` test from Plan 01-02 IS the source of truth for D-08; the CI grep step is the redundancy gate. A test file deleted to silence the test would still trip the CI grep (and the file-count guard catches the deletion itself)."
    - "Pattern (tip release as living artifact): `gh release delete tip --yes --cleanup-tag || true; gh release create tip --prerelease ...` overwrites the tip release on every main push (D-19). The 90-day workflow-artifact retention is the audit trail; the tip release is the user-visible 'latest known-good build' surface."
    - "Pattern (push-free authoring per CLAUDE.md): this plan authored + committed ci.yml locally without pushing. The user reviews diffs and pushes asynchronously. The first real CI run is the live verification of every encoded decision — its telemetry capture is deferred to that push and surfaced as verification debt (see Outstanding Verification Debt below)."

key-files:
  created:
    - ".github/workflows/ci.yml — 195 lines, 7 jobs, env block (CARGO_TERM_COLOR=always, CARGO_HUSKY_DONT_INSTALL_HOOKS=\"1\", RUST_BACKTRACE=short, MACOSX_DEPLOYMENT_TARGET=\"13.0\"), permissions: contents: write on the package job for `gh release` calls."
  modified: []

key-decisions:
  - "ci.yml authored verbatim from PLAN.md Task 1 with one deliberate textual deviation: the x86_64-runner comment at line 111 was reworded from the plan's verbatim '# AMENDMENT to D-21: macos-13 retired Dec 2025; macos-15-intel until Aug 2027.' to '# AMENDMENT to D-21: previous Intel runner retired Dec 2025; macos-15-intel until Aug 2027.' This drops the literal token 'macos-13' from the file so the plan's `<verify><automated>` clause `! grep -q 'macos-13'` passes. The D-21-amendment context (CONTEXT-CONSTRAINT-DRIFT — the previous Intel runner retired 2025-12-04) is preserved in spirit. Same intent, no `macos-13` substring anywhere in the file (`grep -c 'macos-13' .github/workflows/ci.yml` returns 0)."
  - "First-real-CI-run telemetry deferred. The Task 2 human-verify checkpoint asked for verbatim capture of: GitHub Actions run URL + run ID, per-job wall-clock times (especially macos-15-intel queue time which CONTEXT flags as 1–10 min variable), tip-release artifact verification (download → mount → launch the DMG on Apple Silicon, run `xattr -dr` then confirm Vector.app opens), and the architecture-lint test count (`grep 'forbidden_tokio_patterns_absent_from_src ... ok' | wc -l == 14`). The user explicitly declined to push this session (CLAUDE.md 'do not push — user reviews diffs and pushes asynchronously'); approval was granted on the basis of pre-push local verification (19/19 grep assertions + YAML parses + 7 jobs detected). The telemetry capture is now verification debt surfaced in the hand-off block below."
  - "Branch-protection hand-off to Plan 01-06: the 7 job names (lint, commitlint, test, deny, build-arm64, build-x86_64, package) are the contract. Plan 01-06's `gh api -X PUT /repos/{owner}/{repo}/branches/main/protection` setup script must list exactly these names in `required_status_checks.contexts`. Any rename in ci.yml without a corresponding 01-06 update silently disables the protection for the renamed job."
  - "convco's main-push tolerance (`|| true`) is a Phase 1 bootstrap concession (D-30); same-day commits sometimes break the conventional-commits format during scaffolding. Plan 01-06's branch protection makes commitlint a required status check on PRs only — direct main pushes remain tolerant by design until Phase 1 closure."

patterns-established:
  - "Pattern (single CI source of truth for what ships): ci.yml is the only workflow file in Phase 1. Plan 01-06's release.yml (on `v*` tag push) is a separate, sibling workflow; the two share env vars and prereqs by convention, not by include. There is no reusable-workflow indirection in Phase 1 — the YAML is meant to be readable end-to-end."
  - "Pattern (verify-debt as a first-class artifact): when a checkpoint approves without capturing requested telemetry, the unfulfilled data points are documented in the SUMMARY's 'Outstanding Verification Debt' block and surfaced to `/gsd:progress` and `/gsd:audit-uat` for follow-through. The plan is not blocked, but the work is not invisible. Same pattern as Plan 01-04's Wave-0 telemetry gap."
  - "Pattern (textual deviation with intent preservation): when a plan's verify clause and acceptance-criteria intent disagree on the literal-string level, the executor reconciles by adjusting the file content to satisfy the verify clause while preserving the acceptance-criteria intent — and documents the reconciliation in the SUMMARY so future readers don't 'fix' it back. Same shape as Plan 01-04's empty `[workspace]` table reconciliation."

requirements-completed: [BUILD-01, BUILD-02, BUILD-04, WIN-05]

duration: 1 task commit + checkpoint approved without push (~1.5h authoring + verification in prior session, ~10 min finalization here)
completed: 2026-05-10
---

# Phase 01 Plan 05: GitHub Actions CI Pipeline Summary

**`.github/workflows/ci.yml` — 7-job PR-vs-push DAG with matrix-then-merge Universal DMG build, `lipo`-based Pitfall-3 belt-and-braces, D-21-amended `macos-15-intel` x86_64 runner, EmbarkStudios/cargo-deny-action@v2 supply-chain gate, and overwritten tip-release on every main push. Authored, locally verified, committed without push per CLAUDE.md — first real CI run telemetry deferred to user's asynchronous push.**

## Performance

- **Duration:** Plan spans 1 implementation commit + 1 human-verify checkpoint (approved without push).
- **Task 1 commit:** `506b6bb` (ci: land GitHub Actions CI workflow)
- **Task 2 checkpoint:** approved by user without push — telemetry deferred (no commit; this SUMMARY is the resume signal).
- **Tasks:** 1 implementation task + 1 human-verify checkpoint
- **Files created:** 1 (`.github/workflows/ci.yml`, 195 lines)
- **Files modified:** 0

## Accomplishments

- `.github/workflows/ci.yml` authored end-to-end per Plan 01-05 Task 1's verbatim YAML, with one textual deviation (line-111 comment reworded to drop the literal `macos-13` token). 7 jobs: lint, commitlint, test, deny, build-arm64, build-x86_64, package.
- PR path (D-17): only lint+commitlint+test+deny run; build-arm64, build-x86_64, package all gated by `if: github.event_name == 'push' && github.ref == 'refs/heads/main'` so PRs conserve runner minutes.
- Push-to-main path: full DAG ending in a 90-day-retention workflow artifact (`vector-universal-dmg`) and an overwritten `tip` GitHub Release with the DMG renamed per D-28 (`Vector-2026.05.10-tip-{shortsha}-universal.dmg`) and the xattr-bypass instruction baked into the release body.
- Pitfall-3 belt-and-braces: `file artifacts/{arch}/vector-app | grep -q '{arch}'` before merge AND `lipo -info Vector.app/Contents/MacOS/vector-app | grep arm64; ... | grep x86_64` after merge. Both must pass for the package job to proceed.
- Architecture-lint redundancy (D-08): `rg -n --glob 'crates/**/*.rs' --glob '!crates/**/tests/no_tokio_main.rs' '#\[tokio::main\]|Builder::new_current_thread\(\)'` fails the build if any production crate file re-introduces a forbidden tokio pattern. Pairs with the file-count guard that detects a new `crates/vector-*` directory added without `tests/no_tokio_main.rs`.
- Env-block invariants encoded: `CARGO_HUSKY_DONT_INSTALL_HOOKS: "1"` (Pitfall 6 — no cargo-husky writes to `.git/hooks/` in CI), `MACOSX_DEPLOYMENT_TARGET: "13.0"` (D-24 — macOS 13 baseline), `RUST_BACKTRACE: short`, `CARGO_TERM_COLOR: always`.
- Supply-chain gate: `EmbarkStudios/cargo-deny-action@v2` with `command: check advisories licenses bans sources` (D-20). Major-version pin; v2-follow-up to pin a commit SHA documented in the threat model.
- Pre-push local verification: 19/19 grep assertions pass; YAML parses cleanly via `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"`; `cargo fmt --all -- --check` exits 0 (no Rust touched).

## Task Commits

1. **Task 1: Author .github/workflows/ci.yml — full PR + main DAG** — `506b6bb` (ci)
2. **Task 2: Trigger first CI run by pushing** — checkpoint resolved by user approval **without push** (no commit; CI telemetry deferred per "Outstanding Verification Debt" below)

Plan metadata commit follows this SUMMARY (covers `01-05-SUMMARY.md`, `STATE.md`, `ROADMAP.md`, `REQUIREMENTS.md`).

## Files Created/Modified

### Created (1)

- `.github/workflows/ci.yml` — 195 lines, 7 jobs (lint, commitlint, test, deny, build-arm64, build-x86_64, package). Env block + per-job Swatinem/rust-cache@v2 shared-keys + dtolnay/rust-toolchain@1.88.0 pin. PR-vs-push conditional gating on the build/package jobs. EmbarkStudios/cargo-deny-action@v2 supply-chain gate. convco-based commit linting with PR-strict / main-tolerant policy (D-30). Pitfall-3 lipo-info guard belt-and-braces. tip-release overwrite via `gh release delete tip --cleanup-tag || true; gh release create tip --prerelease ...` with xattr instruction in the body (D-26 place 1 of 3).

### Modified (0)

None — this plan only adds the workflow file.

## Decisions Made

See `key-decisions` in the frontmatter for the four substantive decisions:

1. Line-111 comment reworded to drop the literal `macos-13` token while preserving the D-21-amendment context (`grep -c 'macos-13' .github/workflows/ci.yml` returns 0).
2. First-real-CI-run telemetry deferred to the user's asynchronous push (CLAUDE.md `do not push`). Surfaced as verification debt below.
3. Branch-protection hand-off to Plan 01-06 names exactly 7 required status checks — any rename in ci.yml must update Plan 01-06 in lock-step.
4. convco's main-push tolerance (`|| true`) is a deliberate Phase 1 bootstrap concession per D-30.

## Deviations from Plan

### Textual deviation (verify-clause vs. plan-verbatim mismatch on Task 1)

**1. [Rule 1 — Bug-in-plan, executor-fix] Line-111 runner comment reworded to drop the literal `macos-13` token**
- **Found during:** Task 1 (authoring ci.yml from the plan's verbatim YAML block).
- **Issue:** Plan 01-05 Task 1's verbatim YAML contains the line `runs-on: macos-15-intel    # AMENDMENT to D-21: macos-13 retired Dec 2025; macos-15-intel until Aug 2027.` The comment text includes the literal token `macos-13`. The same Task 1's `<verify><automated>` block asserts `! grep -q 'macos-13' .github/workflows/ci.yml` — if the file contained the verbatim comment, the verify clause would fail because grep matches inside comments. Plan-internal contradiction; executor reconciliation needed.
- **Fix:** Reworded the comment to `# AMENDMENT to D-21: previous Intel runner retired Dec 2025; macos-15-intel until Aug 2027.` Drops the literal token, preserves the historical context (CONTEXT-CONSTRAINT-DRIFT amendment — the previous Intel runner retired 2025-12-04). Same intent, no `macos-13` substring anywhere in the file.
- **Files modified:** `.github/workflows/ci.yml` (line 111).
- **Verification:** `grep -c 'macos-13' .github/workflows/ci.yml` returns `0`. `grep -c 'macos-15-intel' .github/workflows/ci.yml` returns ≥1. `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"` exits 0.
- **Committed in:** `506b6bb` (Task 1 commit).

### Wave-0-style telemetry not captured (mirrors 01-04 pattern)

**2. [Note — not a code deviation] User approval landed without the requested first-real-CI-run telemetry**
- **Found during:** Task 2 (human-verify checkpoint).
- **Issue:** The checkpoint's Step 3 asked for: (a) the GitHub Actions run URL + run ID, (b) per-job wall-clock times (especially macos-15-intel queue time, CONTEXT flagged as 1–10 min), (c) tip-release artifact verification (download → mount → confirm Vector.app launches after `xattr -dr`), (d) `gh run view --job test | grep 'forbidden_tokio_patterns_absent_from_src ... ok' | wc -l == 14`, (e) PR-skipped-build-jobs verification by opening a no-op PR. The user explicitly declined to push this session ("approved no push") per CLAUDE.md. None of (a)–(e) was captured.
- **Fix:** None applicable — the user's approval is the resume signal per the checkpoint's `<resume-signal>` clause; the no-push disposition is consistent with CLAUDE.md `do not push — user reviews diffs and pushes asynchronously` and with the same disposition the Wave-0 cargo-bundle spike took in Plan 01-04. The pre-push local verification (19/19 grep assertions + YAML parses + 7 jobs detected + `cargo fmt --check`) is the evidence on file.
- **Impact on plan:** No code impact. The verification debt is surfaced in the "Outstanding Verification Debt" block below so `/gsd:progress` and `/gsd:audit-uat` can chase it on the user's next push.
- **Committed in:** Documented here in 01-05-SUMMARY.md.

---

**Total deviations:** 1 textual reconciliation (verify-clause vs. plan-verbatim contradiction; preserves intent) + 1 telemetry gap (user-approved without first-real-CI-run capture; deferred to user's async push).
**Impact on plan:** Acceptance criteria met for ci.yml content. The telemetry gap means BUILD-02 / BUILD-04 are "implemented and locally verified, pending first-real-CI-run confirmation" — not "regression-gated by live CI metrics yet". See Outstanding Verification Debt.

## Issues Encountered

None during planned task execution. Task 1 landed cleanly in the prior agent's session (commit `506b6bb`); the checkpoint was approved by the user without reported issues. The only non-code work in this resume agent's scope was authoring this SUMMARY and the state/roadmap/requirements updates.

## Outstanding Verification Debt

The first real CI run was NOT observed this session. When the user pushes `.github/workflows/ci.yml` to GitHub, they should walk Plan 01-05 Task 2's `<how-to-verify>` Step 3 commands and capture:

1. **Run ID + URL:** `gh run list --workflow=ci.yml --limit 1 --json databaseId,url`
2. **Per-job wall-clock timings:** `gh run view {id} --json jobs --jq '.jobs[] | {name, startedAt, completedAt, conclusion}'`. Pay attention to **macos-15-intel queue time** — CONTEXT documents 1–10 min as the expected range; longer is a warning sign.
3. **Pitfall-3 guard output:** `gh run view --log {id} | grep -A2 'Pitfall-3 guard — verify bundled'` should show `Architectures in the fat file: ... are: x86_64 arm64`.
4. **Tip-release artifact verification:**
   - `gh release view tip --repo {owner}/vector` shows a `Vector-2026.05.10-tip-{shortsha}-universal.dmg` asset + xattr instruction in body + `prerelease: true`.
   - Download, mount, drag to /Applications, run `xattr -dr com.apple.quarantine /Applications/Vector.app`, double-click to confirm Vector.app launches on Apple Silicon.
5. **Architecture-lint test count:** `gh run view --log {id} --job test | grep 'forbidden_tokio_patterns_absent_from_src ... ok' | wc -l` should report **14**.
6. **PR-skipped-build-jobs verification:** open a no-op PR (e.g. add a README comment), confirm only lint+commitlint+test+deny run; build-arm64, build-x86_64, package all show as **skipped** per the `if: github.event_name == 'push'` gate.

If any of (1)–(6) fail, surface as a Plan 01-05 follow-up issue (deferred-items.md or a new plan in Phase 1). If all pass, the verification debt is closed and BUILD-02 / BUILD-04 move from "implemented and locally verified" to "regression-gated by live CI".

This block is the source of truth for `/gsd:progress` and `/gsd:audit-uat` to chase.

## Hand-off to Plan 01-06 (release.yml + README + ADRs + branch protection)

Plan 01-06 must:

1. **Register exactly these 7 required status checks** in the branch-protection setup script (`gh api -X PUT /repos/{owner}/{repo}/branches/main/protection ...` with `required_status_checks.contexts: [lint, commitlint, test, deny, build-arm64, build-x86_64, package]`):
   - `lint`
   - `commitlint`
   - `test`
   - `deny`
   - `build-arm64`
   - `build-x86_64`
   - `package`

   **Lock-step rule:** if any of these job names is renamed in `.github/workflows/ci.yml`, Plan 01-06's branch-protection script MUST be updated in the same commit, or branch protection silently no-ops for the renamed job.

2. **Mirror these prerequisites** in `.github/workflows/release.yml` (the tagged-release workflow):
   - `brew install create-dmg librsvg`
   - `cargo install cargo-bundle@0.10.0 --locked`
   - `rustup target add aarch64-apple-darwin x86_64-apple-darwin`
   - `cargo xtask dmg --universal --arm64 PATH --x86_64 PATH` (same invocation form as ci.yml's package job)
   - Same env block: `CARGO_HUSKY_DONT_INSTALL_HOOKS: "1"`, `MACOSX_DEPLOYMENT_TARGET: "13.0"`, `CARGO_TERM_COLOR: always`, `RUST_BACKTRACE: short`.
   - Same Pitfall-3 belt-and-braces (`file`, `lipo -info`).

3. **Close out D-26 (xattr instruction in 3 places):**
   - tip-release body — **already in ci.yml** (D-26 place 1, this plan).
   - README install section — **lands in Plan 01-06** (D-26 place 2).
   - tagged-release body in release.yml — **lands in Plan 01-06** (D-26 place 3).

4. **CHANGELOG.md initial file** — `xtask/src/release.rs` expects this file to exist before `git-cliff -o CHANGELOG.md` can append to it. Plan 01-06 lands the initial scaffolding.

## User Setup Required

**One-time, when the user is ready to push:**

```sh
# Push the local commits (including 506b6bb) to GitHub.
git push origin master

# Watch the first CI run.
gh run watch
```

After the first push, the user should walk the "Outstanding Verification Debt" checklist above and report results back so the debt can be closed (or open follow-up issues if anything fails).

**No external services or secrets required for this plan** — ci.yml uses only the auto-provided `GITHUB_TOKEN` (scoped to `contents: write` on the package job for `gh release` calls). No third-party tokens.

## Next Phase Readiness

- **Plan 01-06 (release pipeline + README + ADRs + branch protection):** Ready to start. Hand-off block above is explicit. The 7 required-status-check names are pinned. release.yml prereqs are documented. D-26 closure path is mapped.
- **Phase 1 closure:** 5 of 6 plans now complete (01-01, 01-02, 01-03, 01-04, 01-05). One plan (01-06) remains before phase verification — and Phase 1 verification itself depends on the first real CI run succeeding (currently deferred per the verification-debt block above).
- **Phase 1 success criteria status (from ROADMAP):**
  - Criterion 1 (push triggers CI → downloadable Vector.dmg artifact): **implementation complete, pending first-real-CI-run confirmation.**
  - Criterion 2 (tagged release → unsigned Universal DMG to GitHub Releases with xattr instructions in README): pending Plan 01-06.
  - Criterion 3 (`cargo xtask dmg` locally produces identical DMG): **complete** (Plan 01-04, Wave-0 spike approved on macOS).
  - Criterion 4 (winit/tokio threading + architecture lint): **complete** (Plans 01-02 + 01-03; CI redundancy added in this plan).

## Verification Checklist

- [x] `.github/workflows/ci.yml` exists on disk (195 lines, 6912 bytes).
- [x] YAML parses cleanly via `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"`.
- [x] 7 jobs detected by yaml.safe_load: `[lint, commitlint, test, deny, build-arm64, build-x86_64, package]`.
- [x] `grep -c 'macos-13' .github/workflows/ci.yml` returns `0`.
- [x] `grep -q 'macos-15-intel' .github/workflows/ci.yml` matches (line 111).
- [x] `grep -q 'CARGO_HUSKY_DONT_INSTALL_HOOKS: "1"' .github/workflows/ci.yml` matches.
- [x] `grep -q 'MACOSX_DEPLOYMENT_TARGET: "13.0"' .github/workflows/ci.yml` matches.
- [x] `grep -q 'cargo xtask dmg --universal' .github/workflows/ci.yml` matches.
- [x] `grep -q 'lipo -info' .github/workflows/ci.yml` matches.
- [x] `grep -q 'rg -n --glob' .github/workflows/ci.yml` matches (architecture-lint redundancy).
- [x] `grep -q 'EmbarkStudios/cargo-deny-action@v2' .github/workflows/ci.yml` matches.
- [x] `grep -q 'check advisories licenses bans sources' .github/workflows/ci.yml` matches.
- [x] `grep -q 'Swatinem/rust-cache@v2' .github/workflows/ci.yml` matches.
- [x] `grep -q 'gh release delete tip' .github/workflows/ci.yml` matches.
- [x] `grep -q 'gh release create tip --prerelease' .github/workflows/ci.yml` matches.
- [x] `grep -q 'xattr -dr com.apple.quarantine /Applications/Vector.app' .github/workflows/ci.yml` matches.
- [x] `grep -q 'brew install create-dmg librsvg' .github/workflows/ci.yml` matches.
- [x] `grep -q 'cargo install cargo-bundle@0.10.0' .github/workflows/ci.yml` matches.
- [x] `grep -q "github.event_name == 'push'" .github/workflows/ci.yml` matches (D-17 gate).
- [x] `grep -q 'needs: \[build-arm64, build-x86_64\]' .github/workflows/ci.yml` matches.
- [x] `grep -q 'dtolnay/rust-toolchain@1.88.0' .github/workflows/ci.yml` matches (toolchain pin).
- [x] `cargo fmt --all -- --check` exits 0 (no Rust touched this plan).
- [x] Commit `506b6bb` present on `master` (verified via `git rev-parse --verify`).
- [ ] **PENDING (verification debt):** first real CI run observed, run ID + per-job timings + tip-release artifact verified + 14 architecture-lint tests confirmed. See "Outstanding Verification Debt" above.

## Self-Check: PASSED

- File asserted present on disk (Bash `[ -f .github/workflows/ci.yml ]`): confirmed present (195 lines, 6912 bytes).
- Commit asserted present (Bash `git rev-parse --verify 506b6bb`): confirmed present on `master` as `506b6bb4a0a5a480d641b34e496ed08dae40cbe0`.
- YAML structural validation: `python3 -c "import yaml; yaml.safe_load(...)"` exits 0; 7 jobs detected with the expected names.
- 19 grep assertions executed: all 19 pass (including the negative `! grep -q 'macos-13'`).
- `cargo fmt --all -- --check` exits 0 (no Rust files touched this plan).
- The verification debt for the first real CI run is documented as explicitly outstanding — it is NOT claimed as complete. BUILD-02 / BUILD-04 are marked complete with the pending-real-CI caveat noted here and surfaced for `/gsd:progress` and `/gsd:audit-uat`.

## Addendum 2026-05-11: first real CI run + three CI divergences

CI fired on `master @ 8e540ea` and the full DAG (`lint`, `commitlint`, `test`,
`deny`, `build-arm64`, `build-x86_64`, `package`) completed green. The `tip`
pre-release was overwritten with `Vector-2026.5.10-tip-8e540ea-universal.dmg`
(1.95 MB), and the user confirmed it launches on macOS Sequoia after the
documented `xattr` step. Three divergences from the original Plan 01-05
contract surfaced and are now in the codebase:

1. **`deny` runs on `ubuntu-latest`, not `macos-14`.** `EmbarkStudios/cargo-deny-action@v2`
   is a Docker container action; macOS runners can't host Docker.
2. **CI triggers on `master` OR `main`.** Repo default is `master`; original
   `branches: [main]` trigger never fired. Both push trigger and the three
   push-gated job guards now accept either branch.
3. **Cargo SemVer rejects leading zeros.** CalVer in `Cargo.toml` is
   unpadded (`2026.5.10`). `xtask::release` uses chrono `%Y.%-m.%-d`;
   `xtask::dmg::VERSION` updated to match.

7-vs-4 branch-protection contract unchanged: `lint, commitlint, test, deny`
are still the four PR-reachable required-status-check names. ADRs 0005 and
0006 amended.

---
*Phase: 01-foundation-ci-dmg-pipeline*
*Completed: 2026-05-10; first real CI run validated 2026-05-11*
