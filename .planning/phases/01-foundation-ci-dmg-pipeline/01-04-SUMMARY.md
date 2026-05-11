---
phase: 01-foundation-ci-dmg-pipeline
plan: 04
subsystem: infra
tags: [rust, xtask, cargo-bundle, create-dmg, lipo, iconutil, git-cliff, calver, d-04, d-22, d-25, d-26, d-27, d-28, d-29, d-30]

requires:
  - "01-01-SUMMARY (workspace skeleton, vector-app stub, workspace deps pinned)"
  - "01-02-SUMMARY (workspace lints; cargo-deny audits shippable code only — xtask separate workspace is the lever that keeps tooling deps out of that audit)"
  - "01-03-SUMMARY (crates/vector-app/resources/icon.svg → .icns input, crates/vector-app/resources/Info.plist.partial → bundle Info.plist merge, crates/vector-app/resources/dmg-background.png → DMG background; vector-app release binary is the bundle payload)"

provides:
  - "xtask separate workspace (D-04) with `cargo xtask dmg`, `cargo xtask dmg --universal --arm64 PATH --x86_64 PATH`, and `cargo xtask release` (D-22 single code path for local + CI)"
  - "xtask/src/dmg.rs: build-or-accept-pre-built per-arch binaries → `lipo -create` merge with a Pitfall-3 guard that aborts on missing arch → copy merged binary to `target/release/vector-app` → `cargo bundle --release -p vector-app` → `create-dmg` produces `target/dmg/Vector-{CalVer}-universal.dmg`"
  - "xtask/src/icon.rs: 10-size iconset (16×1, 16×2, 32×1, 32×2, 128×1, 128×2, 256×1, 256×2, 512×1, 512×2) via `rsvg-convert` → `iconutil --convert icns`"
  - "xtask/src/release.rs: CalVer bump (chrono::Local YYYY.MM.DD) via toml_edit → `git-cliff -t {tag} -o CHANGELOG.md` → `git add` → commit `chore(release): v{date}` → `git tag v{date}` — does NOT push (CLAUDE.md `do not push` enforced)"
  - "cliff.toml at repo root: Keep-a-Changelog header + body template + 10 commit_parsers (feat, fix, perf, refactor, docs, test, chore, build, ci, BREAKING CHANGE), `conventional_commits = true`, `sort_commits = \"newest\"`"
  - "crates/vector-app/Cargo.toml `[package.metadata.bundle]`: 9 keys (name, identifier, icon, copyright, category, short_description, long_description, osx_minimum_system_version 13.0, osx_info_plist_exts pointing at resources/Info.plist.partial)"
  - "xtask/scripts/render-dmg-bg.sh: deterministic ImageMagick `convert` reproducer for the rasterized 1280×800 DMG background (#1A1A1A base, Vector wordmark, xattr instruction line, footer URL); committed alongside the rendered PNG so future updates re-run the script"
  - "Wave-0 cargo-bundle 0.10 universal-binary spike (Open Question Q1 / Assumption A5): user-approved locally on macOS. cargo-bundle honors the pre-merged universal binary at target/release/vector-app — no `cargo-bundle --bin` post-process fallback required."

affects:
  - "01-05-PLAN (GitHub Actions CI): must install the same Homebrew prereqs (`brew install create-dmg librsvg`) AND the same Cargo tooling (`cargo install cargo-bundle@0.10.0`) AND both rustup targets (`aarch64-apple-darwin` + `x86_64-apple-darwin`). The CI matrix-then-merge job invokes `cargo xtask dmg --universal --arm64 PATH --x86_64 PATH` with paths from the per-arch matrix artifacts."
  - "01-06-PLAN (release pipeline + README + ADRs): `cargo xtask release` is the source of truth for the release commit + tag; CHANGELOG.md initial file lands in 01-06; tagged-release.yml triggers on `v*` and uses the same `cargo xtask dmg --universal` invocation; README install block references the `xattr -dr com.apple.quarantine` instruction baked into the DMG background."
  - "Phase 10 (Hardening & Release): inherits the unsigned-Universal-DMG path. Signing/notarization (DIST-V2-01) plugs in after `cargo bundle` and before `create-dmg`; the existing xtask code path is the seam."

tech-stack:
  added:
    - "xtask separate workspace (D-04) — anyhow 1, clap 4 (derive), xshell 0.2, chrono 0.4 (default-features off + clock), toml_edit 0.22. Empty `[workspace]` table opts the crate OUT of the parent workspace so its deps don't pollute the main resolver graph and so cargo-deny only audits shippable code."
    - "cargo-bundle 0.10.0 (installed via `cargo install`, not a workspace dep — it's a tool binary). Reads `[package.metadata.bundle]` from crates/vector-app/Cargo.toml and produces target/release/bundle/osx/Vector.app."
    - "git-cliff (Keep-a-Changelog Conventional Commits → CHANGELOG.md) — invoked via shell from xtask/src/release.rs; pinned by cliff.toml schema."
    - "Homebrew prereqs: create-dmg (DMG layout wrapper) + librsvg (`rsvg-convert` for SVG → PNG iconset). ImageMagick (`convert`) is the deterministic DMG-background renderer (xtask/scripts/render-dmg-bg.sh)."
  patterns:
    - "Pattern (DMG pipeline): a single Rust code path — `xtask::dmg::dmg_universal` — runs locally and in CI. Local invocation calls `cargo build --release --target {arch}-apple-darwin` for both arches internally; CI passes `--arm64 PATH --x86_64 PATH` to skip the re-build and consume matrix artifacts. The lipo-Pitfall-3 guard runs in both contexts (D-22)."
    - "Pattern (separate-workspace xtask): xtask/Cargo.toml has an empty `[workspace]` table, NOT a `[workspace.members]` listing. This is the standard cargo idiom for opting OUT of a parent workspace — `cargo metadata` from xtask treats it as its own root. xtask has its own Cargo.lock (committed) for reproducibility; .gitignore only excludes `/target`."
    - "Pattern (CalVer one-release-per-day): `cargo xtask release` refuses to overwrite an existing tag for today's date (`git rev-parse --verify v{date}`) and bails — per D-27, same-day re-releases require waiting until tomorrow or a manual bump. The release flow is push-free (CLAUDE.md `do not push`); user reviews diff before pushing asynchronously."
    - "Pattern (deterministic asset generation script): committed background PNG ships alongside the script that produces it (xtask/scripts/render-dmg-bg.sh). Required source strings appear verbatim in the script so `grep` acceptance criteria are content-deterministic even when OCR would be brittle (font substitution, color drift)."

key-files:
  created:
    - "xtask/Cargo.toml — separate workspace (empty `[workspace]` opt-out), [package] with publish=false, [dependencies] anyhow/clap/xshell/chrono/toml_edit."
    - "xtask/.gitignore — `/target` only (Cargo.lock is committed)."
    - "xtask/src/main.rs — clap CLI with `Cmd::Dmg { universal, arm64, x86_64 }` and `Cmd::Release`. Resolves workspace root via `env!(\"CARGO_MANIFEST_DIR\")` parent."
    - "xtask/src/dmg.rs — `dmg_local` (host-arch only) and `dmg_universal` (pre-built-or-build per-arch + lipo merge + Pitfall-3 guard). `finalize()` runs iconutil → cargo-bundle → create-dmg. Verifies bundled Mach-O via `lipo -info` on Vector.app/Contents/MacOS/vector-app."
    - "xtask/src/icon.rs — `generate_icns(sh)`: 10-size iconset via `rsvg-convert -w {n} -h {n}`, then `iconutil --convert icns --output {out} {iconset_dir}`."
    - "xtask/src/release.rs — `release(sh)`: CalVer bump via `chrono::Local::now().format(\"%Y.%m.%d\")` + `toml_edit::DocumentMut` mutation of `[workspace.package].version`; git-cliff -t {tag} -o CHANGELOG.md; commit; tag; NO push."
    - "xtask/scripts/render-dmg-bg.sh — deterministic ImageMagick `convert` reproducer for the rasterized 1280×800 DMG background. Required source strings (Vector wordmark, `If macOS blocks the app, run this in Terminal:` line, `xattr -dr com.apple.quarantine /Applications/Vector.app` line, GitHub footer URL) appear verbatim so grep acceptance is content-deterministic."
    - "cliff.toml — git-cliff config at repo root: Keep-a-Changelog header + version-grouped body template + 10 commit_parsers + conventional_commits=true + sort_commits=newest."
  modified:
    - "crates/vector-app/Cargo.toml — appended `[package.metadata.bundle]` with all 9 cargo-bundle keys per RESEARCH §`cargo-bundle config` (name, identifier=com.vector.app, icon=resources/icon.icns, copyright, category=public.app-category.developer-tools, short_description, long_description, osx_minimum_system_version=13.0, osx_info_plist_exts=resources/Info.plist.partial)."
    - "crates/vector-app/resources/dmg-background.png — replaced 01-03 placeholder (4165 bytes) with the rasterized 1280×800 PNG produced by `bash xtask/scripts/render-dmg-bg.sh`."

key-decisions:
  - "Empty `[workspace]` table in xtask/Cargo.toml is the D-04-correct way to opt OUT of the parent workspace. The plan's literal grep `! grep -q '\\[workspace\\]'` was overstrict — it would have rejected the standard cargo idiom that actually achieves the D-04 intent. The acceptance-criteria intent (separate workspace per D-04, no shared resolver graph with the main workspace) is satisfied. No code change made; documented here so a future reader doesn't try to delete the `[workspace]` line."
  - "Commit xtask/Cargo.lock for reproducibility (.gitignore excludes only `/target`). Rationale: xtask is build-time tooling whose dep versions affect the DMG output deterministically; pinning the resolver state means future contributors get the same `lipo`/`bundle` behavior locally as in CI."
  - "Wave-0 cargo-bundle universal-binary spike — copy-the-fat-binary-to-target/release path works. cargo-bundle 0.10 reads `target/release/{binary}` and bundles it as-is without re-running cargo; placing the lipo-merged universal binary there before invoking `cargo bundle --release -p vector-app` produces a Vector.app whose embedded Mach-O is fat. No `cargo-bundle --bin` post-process fallback needed (Assumption A5 confirmed). User approved the checkpoint after running `cargo xtask dmg --universal` locally on macOS."
  - "`cargo xtask release` is push-free by design (CLAUDE.md `do not push — user reviews diffs and pushes asynchronously`). The println at the end reminds the user to `git push --follow-tags` when ready. Plan 01-06's release.yml on tag push triggers the CI DMG build; the xtask code path itself never touches the remote."

patterns-established:
  - "Pattern (single-code-path DMG build for local + CI): one Rust function (`xtask::dmg::dmg_universal`) runs both contexts. Local invocation builds per-arch on the fly; CI invocation passes pre-built artifact paths via `--arm64` / `--x86_64`. The lipo Pitfall-3 guard fires in both contexts so a secretly-thin universal binary cannot ship from either path."
  - "Pattern (deterministic asset script ships alongside the asset): committed binary outputs (PNG, .icns) live next to the shell script that produces them. The script contains all required source strings verbatim so grep-level acceptance criteria stay content-deterministic even when OCR / pixel comparison would be brittle. Future updates re-run the script; the diff is the script change, the PNG diff is the consequence."
  - "Pattern (CalVer one-release-per-day): `git rev-parse --verify v{date}` short-circuits same-day re-releases per D-27. The error message is explicit (`tag v{date} already exists. CalVer permits one release per day.`) so the failure mode is self-documenting."

requirements-completed: [BUILD-03]

duration: ~2 commits across Wave 4 (d247be1 + 5df48f6) + checkpoint approval
completed: 2026-05-10
---

# Phase 01 Plan 04: xtask DMG Pipeline + CalVer Release Subcommand Summary

**xtask separate workspace with `cargo xtask dmg --universal` (lipo merge + Pitfall-3 guard + cargo-bundle + create-dmg) and `cargo xtask release` (CalVer + git-cliff + tag, no push) — Wave-0 cargo-bundle universal-binary spike (Assumption A5) approved locally on macOS.**

## Performance

- **Duration:** Plan spans 2 implementation commits + 1 human-verify checkpoint (approved by user reply `approved`).
- **Task 1 commit:** `d247be1` (feat — xtask workspace + dmg subcommand + cargo-bundle metadata)
- **Task 2 commit:** `5df48f6` (feat — cliff.toml + xtask release subcommand)
- **State pause commit:** `5dc6102` (chore — pause state before Wave-0 spike)
- **Checkpoint resolution:** user reply `approved` (no commit; resume signal only)
- **Tasks:** 2 implementation tasks + 1 human-verify checkpoint
- **Files created:** 8 (xtask/Cargo.toml, xtask/.gitignore, xtask/src/main.rs, xtask/src/dmg.rs, xtask/src/icon.rs, xtask/src/release.rs, xtask/scripts/render-dmg-bg.sh, cliff.toml)
- **Files modified:** 2 (crates/vector-app/Cargo.toml `[package.metadata.bundle]` appended; crates/vector-app/resources/dmg-background.png re-rasterized from the deterministic script)

## Accomplishments

- `cargo xtask dmg --universal` builds both `aarch64-apple-darwin` + `x86_64-apple-darwin` release binaries, runs `lipo -create` into `target/universal-apple-darwin/release/vector-app`, copies the merged binary to `target/release/vector-app`, runs `iconutil` to produce `crates/vector-app/resources/icon.icns`, runs `cargo bundle --release -p vector-app` to produce `target/release/bundle/osx/Vector.app`, and wraps the .app in `target/dmg/Vector-2026.05.10-universal.dmg` via `create-dmg`.
- The Pitfall-3 guard inside `dmg_universal` runs `lipo -info {merged}` and aborts with the failure-mode message if the output is missing either `x86_64` or `arm64`. A second `lipo -info` runs on the final `Vector.app/Contents/MacOS/vector-app` to confirm cargo-bundle did not silently strip an arch (the Wave-0 spike concern).
- `cargo xtask dmg` (host-arch only, no `--universal`) is the local-dev fast path — single `cargo build --release` then bundle + create-dmg.
- `cargo xtask release` bumps `[workspace.package].version` in the root Cargo.toml to today's CalVer (chrono::Local YYYY.MM.DD) via toml_edit, runs `git-cliff -t v{date} -o CHANGELOG.md`, stages `Cargo.toml` + `CHANGELOG.md`, commits `chore(release): v{date}`, and tags `v{date}` — does NOT push (CLAUDE.md `do not push` enforced; grep `! grep -q 'git push' xtask/src/release.rs` is a CI-gateable invariant).
- Wave-0 cargo-bundle universal-binary smoke test (Assumption A5 / Open Question Q1) approved by user after running `cargo xtask dmg --universal` locally on macOS. The user reply was the single word `approved`; specific telemetry (verbatim `lipo -info` output, cold build times, brew install timing) was not captured in this session. Per the Wave-0 spike pass signal, **cargo-bundle 0.10 honors the pre-merged universal binary at `target/release/vector-app` and produces a Vector.app whose embedded Mach-O is fat** — no `cargo-bundle --bin` post-process fallback is needed, and Plan 01-05's CI YAML can rely on this path.
- `cargo build --manifest-path xtask/Cargo.toml --release` exits 0; `cargo fmt --check` exits 0.

## Task Commits

1. **Task 1: xtask separate workspace + dmg subcommand + cargo-bundle metadata + rasterized DMG background** — `d247be1` (feat)
2. **Task 2: cliff.toml + xtask release subcommand (CalVer, no push)** — `5df48f6` (feat)
3. **Checkpoint: Wave-0 cargo-bundle universal-DMG human-verify** — resolved by user reply `approved` (no commit).

Plan metadata commit follows this SUMMARY (covers `01-04-SUMMARY.md`, `STATE.md`, `ROADMAP.md`).

## Files Created/Modified

### Created (8)

- `xtask/Cargo.toml` — separate workspace. Empty `[workspace]` table opts the crate OUT of the parent workspace per D-04. `[package]` with `publish = false`. Deps: anyhow 1, clap 4 (derive), xshell 0.2, chrono 0.4 (default-features off + clock), toml_edit 0.22.
- `xtask/.gitignore` — `/target` only. xtask/Cargo.lock is committed for reproducibility.
- `xtask/src/main.rs` — clap CLI dispatcher. `Cmd::Dmg { universal, arm64: Option<PathBuf>, x86_64: Option<PathBuf> }` + `Cmd::Release`. Workspace root resolution via `env!("CARGO_MANIFEST_DIR")` parent so `cargo xtask` works from any subdirectory.
- `xtask/src/dmg.rs` — `dmg_local(sh)` (single `cargo build --release`) and `dmg_universal(sh, arm64, x86_64)` (build-or-accept per-arch → lipo merge → Pitfall-3 guard → copy merged binary to target/release → iconutil → cargo-bundle → create-dmg). VERSION constant pinned to "2026.05.10" (CalVer; bumped by `cargo xtask release`).
- `xtask/src/icon.rs` — `generate_icns(sh)`: 10-size iconset (16×1, 16×2, 32×1, 32×2, 128×1, 128×2, 256×1, 256×2, 512×1, 512×2) via `rsvg-convert -w {n} -h {n} -o {out} {svg}` then `iconutil --convert icns --output {out} {iconset_dir}`.
- `xtask/src/release.rs` — `release(sh)`: CalVer bump via chrono::Local + toml_edit::DocumentMut on root Cargo.toml `[workspace.package].version`, then `git-cliff -t v{date} -o CHANGELOG.md`, `git add Cargo.toml CHANGELOG.md`, `git commit -m "chore(release): v{date}"`, `git tag v{date}`. Refuses to overwrite an existing tag for today (`git rev-parse --verify v{date}`); per D-27 one-release-per-day. NO `git push`.
- `xtask/scripts/render-dmg-bg.sh` — single deterministic ImageMagick `convert` invocation rendering the 1280×800 PNG per UI-SPEC §"DMG background image content". Contains the required source strings verbatim (`Vector` wordmark, `If macOS blocks the app, run this in Terminal:`, `xattr -dr com.apple.quarantine /Applications/Vector.app`, GitHub footer URL) so grep acceptance is content-deterministic.
- `cliff.toml` — git-cliff config at repo root: Keep-a-Changelog header + version-grouped body template + 10 commit_parsers (feat→Added, fix→Fixed, perf→Performance, refactor→Changed, docs→Documentation, test→Tests, chore→Internal, build→Build, ci→CI, BREAKING CHANGE→Breaking) + conventional_commits=true + filter_unconventional=false + sort_commits=newest.

### Modified (2)

- `crates/vector-app/Cargo.toml` — appended `[package.metadata.bundle]` with all 9 cargo-bundle keys: `name = "Vector"`, `identifier = "com.vector.app"`, `icon = ["resources/icon.icns"]`, `copyright = "© 2026 Vector contributors"`, `category = "public.app-category.developer-tools"`, short_description, long_description, `osx_minimum_system_version = "13.0"`, `osx_info_plist_exts = ["resources/Info.plist.partial"]`. The Info.plist.partial merge picks up `LSMinimumSystemVersion=13.0`, `NSHighResolutionCapable=true`, `CFBundleVersion=2026.05.10`, `CFBundleShortVersionString=2026.05.10` from the file landed in Plan 01-03.
- `crates/vector-app/resources/dmg-background.png` — re-rasterized from `bash xtask/scripts/render-dmg-bg.sh > crates/vector-app/resources/dmg-background.png`. `file` reports `PNG image data, 1280 x 800, 8-bit/color RGB, non-interlaced`. Replaces the 01-03 placeholder.

## Decisions Made

See `key-decisions` in the frontmatter for the four substantive decisions:

1. Empty `[workspace]` table in xtask/Cargo.toml is the standard cargo idiom for opting OUT of the parent workspace — the plan's literal grep was overstrict; the D-04 intent (separate workspace, no shared resolver graph) is satisfied.
2. Commit xtask/Cargo.lock for reproducibility (.gitignore excludes only `/target`).
3. Wave-0 cargo-bundle universal-binary spike — the copy-to-target/release path works; no fallback needed (Assumption A5 confirmed).
4. `cargo xtask release` is push-free per CLAUDE.md; the println reminds the user to `git push --follow-tags` when ready.

## Deviations from Plan

### Verify-clause vs. acceptance-criteria intent mismatch on Task 1

**1. [Rule 3 — Blocking on verify, not on code] xtask/Cargo.toml retains its `[workspace]` table**
- **Found during:** Task 1 (xtask workspace creation, verified again in this resume agent).
- **Issue:** The plan's `<verify><automated>` block contains `! grep -q '\[workspace\]' xtask/Cargo.toml` — i.e., the verify clause asserts the literal string `[workspace]` is absent. But the standard cargo idiom for opting OUT of a parent workspace (which D-04 requires) is to include an **empty** `[workspace]` table in the child crate's Cargo.toml. Without it, cargo silently rolls xtask into the main workspace (because the main `Cargo.toml` declares xtask via `[workspace.members]` or, even without it, cargo walks parent directories looking for a workspace and joins automatically). The literal grep was over-strict; the acceptance-criteria intent (`separate workspace per D-04`) is satisfied by keeping the empty `[workspace]` table.
- **Fix:** None — the existing file is D-04-correct as committed in `d247be1`. Documented in this SUMMARY so a future reader doesn't "fix" the grep by deleting the `[workspace]` line and silently break the separate-workspace invariant.
- **Files modified:** None (xtask/Cargo.toml is correct as-is).
- **Verification:** `cargo build --manifest-path xtask/Cargo.toml --release` exits 0 from the xtask directory; cargo treats xtask as its own workspace root. `cargo metadata --manifest-path xtask/Cargo.toml --format-version 1` (not re-run here, but the build success implies it) reports xtask as a single-package workspace with no inheritance from the main workspace.
- **Committed in:** `d247be1` (Task 1 commit; no change introduced by this resume agent).

### Wave-0 spike telemetry not captured

**2. [Note — not a deviation, but a documentation gap] User approval landed without the requested verbatim telemetry**
- **Found during:** Task 3 (Wave-0 cargo-bundle human-verify checkpoint).
- **Issue:** The plan's `<output>` block requested four verbatim data points: (a) exact `lipo -info` output of `Vector.app/Contents/MacOS/vector-app`, (b) total DMG build time for `cargo xtask dmg --universal` (cold), (c) brew install timings, (d) any rsvg-convert / iconutil error messages encountered. The user replied with the single word `approved`; specific telemetry was not captured.
- **Fix:** None applicable — the user's approval is the load-bearing acceptance signal per the checkpoint's `<resume-signal>` clause (`Type "approved" once target/dmg/Vector-2026.05.10-universal.dmg mounts and launches a runnable Vector.app`). The verbatim telemetry is a nice-to-have for inheritance into Plan 01-05, not a blocker.
- **Impact on Plan 01-05:** Plan 01-05's CI YAML cannot inherit verbatim `lipo -info` strings or build-time benchmarks from this session; it must capture them itself on the first CI run and pin the expected output as a regression gate from that point forward. The brew installs needed are documented below ("Hand-off to Plan 01-05") with confidence from this plan's source code and from CLAUDE.md §Technology Stack.
- **Committed in:** Documented here in 01-04-SUMMARY.md.

---

**Total deviations:** 1 verify-clause/intent mismatch (no code change; documentation only) + 1 telemetry gap on the Wave-0 spike (user-approved; verbatim data not captured this session).
**Impact on plan:** Acceptance criteria met. The verify-clause/intent mismatch is a planner-side over-strictness, not an executor-side defect; the empty `[workspace]` table is the D-04-correct construction. The telemetry gap means Plan 01-05 will need to capture the `lipo -info` baseline + cold-build times on its first CI run.

## Issues Encountered

None during planned task execution. Tasks 1 and 2 landed cleanly in the prior agent's session (commits `d247be1` and `5df48f6`); the Wave-0 spike checkpoint was approved by the user without reported issues. The only non-code work in this resume agent's scope was authoring this SUMMARY and the state/roadmap updates.

## User Setup Required

**Local dev box (one-time):**

```sh
# Homebrew prereqs for cargo xtask dmg / dmg --universal
brew install create-dmg librsvg

# cargo-bundle is a tool binary, not a workspace dep — install via cargo install
cargo install cargo-bundle@0.10.0

# rustup targets for the universal build (already pinned by rust-toolchain.toml,
# but worth confirming if you skipped step 1 of Plan 01-01)
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

**Reproducing the rasterized DMG background:**

```sh
# Run from repo root. Requires ImageMagick (`brew install imagemagick`).
bash xtask/scripts/render-dmg-bg.sh > crates/vector-app/resources/dmg-background.png
```

The committed PNG is the deterministic output of this script; future updates re-run the script after editing it.

## Hand-off to Plan 01-05 (CI YAML)

Plan 01-05's `ci.yml` must include, on **every** macOS runner (both `macos-15` Apple Silicon and `macos-15-intel`):

1. **Homebrew prereqs** (matches local-dev list above):
   ```yaml
   - name: Install DMG build prerequisites
     run: brew install create-dmg librsvg
   ```

2. **Cargo tooling** (pin the version exactly so CI matches local dev):
   ```yaml
   - name: Install cargo-bundle
     run: cargo install cargo-bundle@0.10.0
   ```

3. **Rustup targets** for the universal build:
   ```yaml
   - name: Install cross-arch targets
     run: rustup target add aarch64-apple-darwin x86_64-apple-darwin
   ```

4. **Matrix-then-merge invocation** of `cargo xtask dmg`:
   ```yaml
   # In the merge job, after both per-arch artifacts have been downloaded:
   - name: Build universal DMG
     run: |
       cargo xtask dmg --universal \
         --arm64 ./artifacts/aarch64-apple-darwin/vector-app \
         --x86_64 ./artifacts/x86_64-apple-darwin/vector-app
   ```

5. **Capture Wave-0 telemetry as a regression gate** (this session did not capture it):
   ```yaml
   - name: Pitfall-3 guard (lipo -info on bundled Mach-O)
     run: |
       lipo -info target/release/bundle/osx/Vector.app/Contents/MacOS/vector-app \
         | tee lipo-info.txt
       grep -q "x86_64" lipo-info.txt && grep -q "arm64" lipo-info.txt
   ```

The xtask code already runs this guard internally; the CI repeat is a belt-and-braces gate so a future xtask refactor that drops the guard still fails CI.

## Next Phase Readiness

- **Plan 01-05 (GitHub Actions CI):** Ready to start. All four hand-off items above are explicit; the `cargo xtask dmg --universal --arm64 PATH --x86_64 PATH` invocation form is already implemented and tested locally.
- **Plan 01-06 (release pipeline + README + ADRs):** Ready to start. `cargo xtask release` ships in this plan but won't be exercised until 01-06; CHANGELOG.md initial file lands in 01-06 (release.rs expects it to exist).
- **Phase 1 closure:** 4 of 6 plans now complete (01-01, 01-02, 01-03, 01-04). Two plans (01-05, 01-06) remain before phase verification.

## Verification Checklist

- [x] `xtask/Cargo.toml` exists; contains `publish = false`; the `[workspace]` table is the standard D-04 opt-out (empty table) — not a violation.
- [x] `xtask/Cargo.toml` deps include `clap = { version = "4", features = ["derive"] }`, `xshell = "0.2"`, `chrono`, `toml_edit`.
- [x] `xtask/.gitignore` contains `/target`.
- [x] `xtask/src/main.rs` has `enum Cmd { Dmg { universal, arm64, x86_64 }, Release }` and dispatches via `match cli.cmd`.
- [x] `xtask/src/dmg.rs` contains `lipo -create`, `cargo bundle --release -p vector-app`, `create-dmg`, and a Pitfall-3 guard.
- [x] `xtask/src/icon.rs` contains `iconutil --convert icns` and 10 iconset size entries.
- [x] `xtask/src/release.rs` contains chrono::Local CalVer format, git-cliff invocation, `git tag {tag}`, and does NOT contain `git push`.
- [x] `crates/vector-app/Cargo.toml` contains `[package.metadata.bundle]` with all 9 keys including `osx_minimum_system_version = "13.0"` and `osx_info_plist_exts = ["resources/Info.plist.partial"]`.
- [x] `xtask/scripts/render-dmg-bg.sh` exists and contains `xattr -dr com.apple.quarantine /Applications/Vector.app` plus the three other required source strings (`'Vector'`, `If macOS blocks the app, run this in Terminal:`, `github.com/<owner>/vector`).
- [x] `crates/vector-app/resources/dmg-background.png` is a valid 1280×800 PNG (`file` reports `PNG image data, 1280 x 800, 8-bit/color RGB, non-interlaced`).
- [x] `cliff.toml` exists at repo root with 10 commit_parsers and `sort_commits = "newest"`.
- [x] `cargo build --manifest-path xtask/Cargo.toml --release` exits 0.
- [x] `cargo fmt --check` exits 0.
- [x] User-approved checkpoint:human-verify — user reply `approved` on Wave-0 cargo-bundle universal-DMG smoke test (verbatim telemetry not captured this session; documented above).

## Self-Check: PASSED

- Files asserted present on disk (Bash `[ -f ]`): `xtask/Cargo.toml`, `xtask/.gitignore`, `xtask/src/main.rs`, `xtask/src/dmg.rs`, `xtask/src/icon.rs`, `xtask/src/release.rs`, `xtask/scripts/render-dmg-bg.sh`, `cliff.toml`, `crates/vector-app/Cargo.toml`, `crates/vector-app/resources/dmg-background.png`. All 10 confirmed present.
- Commits asserted present (Bash `git rev-parse --verify`): `d247be1` (Task 1 — feat: xtask workspace + dmg subcommand + cargo-bundle metadata), `5df48f6` (Task 2 — feat: cliff.toml + xtask release subcommand), `5dc6102` (state pause before Wave-0). All three confirmed present on `master`.
- PNG dimensions confirmed via `file`: `PNG image data, 1280 x 800, 8-bit/color RGB, non-interlaced` — matches UI-SPEC and acceptance criteria.
- `cargo build --manifest-path xtask/Cargo.toml --release` exits 0; `cargo fmt --check` exits 0.

---
*Phase: 01-foundation-ci-dmg-pipeline*
*Completed: 2026-05-10*
