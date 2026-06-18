# Phase 10: Hardening & Release — Context

**Gathered:** 2026-05-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Ship Vector v1.0.0 as an unsigned Universal `.dmg` on GitHub Releases. Add CI gates that block regression on (a) GPU renderer output, (b) VT escape-sequence handling, (c) auth-token leakage in logs, (d) unaudited unsafe code in release-profile dependencies. Provide README-front-of-file install instructions that the audience of ~5 Adobe teammates can follow without ceremony.

Strict no-go list (carried from PROJECT.md): no Apple Developer subscription, no code signing, no notarization, no Sparkle/auto-update, no public open-source push. These are v2 (DIST-V2-01, DIST-V2-02).

</domain>

<decisions>
## Implementation Decisions

### HARDEN-01 — Renderer Snapshot Suite

- **D-01:** Scene-based snapshot fixtures (not full-frame, not glyph-atlas-only). Author 4–8 curated test scenes that exercise the visible compositing stack end-to-end.
- **D-02:** Initial fixture set covers: (a) plain text with mixed Unicode + emoji, (b) full alt-screen with colors + cursor + selection, (c) reconnect status bar + tab badge (re-uses Phase 9 ReconnectPass), (d) split panes + scrollback. Plan may add up to 4 more scenes during research.
- **D-03:** Comparator is **perceptual** — delta-E or SSIM with threshold ~2.0 to absorb sub-pixel/antialias drift across arm64 and x86_64 CI runners without flapping. Planner picks the exact library (`image-compare`, `imageproc`, or hand-rolled on `image` crate) based on what's lightest in the dep graph.
- **D-04:** Pinned font for all snapshot tests = the existing `crates/vector-app/resources/Fonts/JetBrainsMono-Regular.ttf` (the only bundled font). No system fallback during snapshot tests — failures must reproduce locally.
- **D-05:** Snapshot crate placement: new `crates/vector-render-snapshots/` test-only crate (keeps `vector-render` build-time clean of `image`/diff deps; lets snapshot tests pull in heavy comparators without leaking into the binary).
- **D-06:** `insta` is the runner (already a workspace-wide dev-dep). Goldens committed to git (PNGs are ~4–16 KB each at terminal sizes; no git-lfs needed for ≤16 fixtures).

### HARDEN-02 — VT Conformance Corpus

- **D-07:** Hand-craft an 8-scenario corpus mapped 1:1 to the ROADMAP success criterion (alt-screen, scroll regions, tab stops, ED/EL, mouse 1006, OSC 52 round-trip, bracketed paste, DECSCUSR). Each test maps to a documented PITFALLS.md item. Lives in `crates/vector-term/tests/vt_conformance/` (one file per scenario or a single `vt_conformance.rs` — planner decides).
- **D-08:** Drive the corpus **against `alacritty_terminal::Term` in unit tests** — feed escape sequences, assert resulting grid/cursor/mode state via Term's API. The VT parser IS `alacritty_terminal` in our stack, so testing it tests our pipe. Fast (<1s for full corpus), zero windowing/GPU infrastructure required.
- **D-09:** Out of scope for v1 corpus: vendoring vttest's full input set (too much obsolete VT100 noise); spawning the real `vector` binary end-to-end for VT input (deferred to v2 if/when we add a true e2e harness). Both are captured as deferred ideas.
- **D-10:** Perf gate metrics for success criterion #2 — Claude's Discretion (see below).

### HARDEN-03 — Hardening (`cargo deny` + token redaction)

- **D-11:** Token redaction = **heavy audit**:
  1. Sweep every token-bearing struct (Microsoft tokens, GitHub OAuth tokens, Codespaces RPC tokens, Dev Tunnels access tokens, SSH key material). Confirm manual `Debug` impls exist per PITFALL 14. Add missing ones.
  2. Add a workspace clippy lint or arch-test that fails the build on `#[derive(Debug)]` for any struct in the auth/token modules.
  3. Add a CI step that records a smoke run (login + list codespaces / list tunnels) with `RUST_LOG=debug` and greps the tracing output for `gho_`, `ghp_`, `eyJ`. Zero matches required. This is the literal promise of HARDEN-03's success criterion #3.
- **D-12:** `cargo deny` — add `[bans] unsafe = "deny"` with an explicit allowlist of crates we accept unsafe in:
  - `objc2`, `objc2-app-kit`, `objc2-foundation` (AppKit FFI is unsafe by definition)
  - `wgpu` (GPU bindings)
  - `alacritty_terminal` (VT parser)
  - `crossfont` (CoreText FFI)
  - `portable-pty` (PTY syscalls)
  - Any other unsafe-bearing dep added later **must** be explicitly added to the allowlist with a one-line reason comment. Otherwise CI fails.
- **D-13:** Existing `deny.toml` advisories/licenses/bans/sources blocks stay as-is. Only the `unsafe` knob is new for Phase 10.
- **D-14:** Release-profile binding — the `unsafe` policy applies to the full dep graph regardless of profile (cargo-deny doesn't natively profile-filter; the REQUIREMENTS wording "release profile" is interpreted as "everything that ships in the release binary," which is the same as the workspace dep tree for a single-binary app).

### HARDEN-04 — Tagged Release

- **D-15:** DMG asset name = `Vector-{version}-universal.dmg` (e.g. `Vector-1.0.0-universal.dmg`). Version + arch in the filename so teammates know which build sits on disk; GH Releases requires unique asset names per release anyway.
- **D-16:** Companion artifact: `Vector-{version}-universal.dmg.sha256` checksum file uploaded alongside the DMG. One-line content; helps teammates verify download integrity.
- **D-17:** README install instructions:
  - First content section is `## Install` (above any feature pitch, screenshots, etc.).
  - Copy-paste block contains both `xattr -dr com.apple.quarantine /Applications/Vector.app` and `open /Applications/Vector.app`.
  - A one-paragraph `### Why the xattr step?` follows the copy block explaining why (unsigned, no Apple Developer subscription, v2 will sign). Keeps teammates from googling.
- **D-18:** Release notes:
  - v1.0.0 ships with a hand-written note: "what's in v1" (the 51 mapped requirements at a teammate-readable level), "what's out of v1" (signing, notarization, auto-update, public OSS push, the 999.x backlog).
  - Future point releases (v1.0.1, v1.1.0) use `gh release create --generate-notes`.
- **D-19:** Universal-binary assembly happens in `release.yml`: existing arm64 + x86_64 build jobs feed a new `package` job that runs `lipo` → `cargo bundle --release` → `hdiutil create` → `shasum -a 256` → `gh release upload`. The existing two build jobs already work — only the `release` job needs to grow.
- **D-20:** Tag style = `v1.0.0` (matches existing `release.yml` trigger `on: push: tags: ['v*']`).

### Phase 9 Coupling

- **D-21:** Phase 10 planning may proceed in parallel with Phase 9 HUMAN-UAT walks (09-05, 09-06) and 09-SMOKE.md sign-off. **However, the v1.0.0 tag itself must wait until PERSIST-04 is signed off** — Phase 10 success criterion #4 requires "all v1 features in place," which includes PERSIST-04. The plan's final release-cut task is gated on PERSIST-04 = Complete in REQUIREMENTS.md.

### Claude's Discretion

- Perf gate measurement approach (idle CPU < 1%, `cat large.log` at vsync cap). Planner chooses: `criterion` benches vs custom probe vs CI-time `ps`/`time` sampling. Whatever measures cleanly on `macos-14` and `macos-15-intel` runners.
- Exact perceptual-tolerance library for HARDEN-01 D-03 (decided in planner; the threshold ~2.0 is locked, the lib isn't).
- Whether the token-redaction grep gate runs against a recorded tracing-output file in git or against a freshly-recorded one each CI run (recorded is reproducible; fresh is robust to silent regressions — planner picks).
- Whether the VT conformance tests live as `vt_conformance.rs` with sub-tests or one file per scenario (`alt_screen.rs`, `scroll_regions.rs`, …).
- Whether `lipo` runs in a new GH Actions job or inside the existing `release` job.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project-level guardrails
- `.planning/PROJECT.md` — non-negotiables (unsigned v1, no Apple Developer, no notarization, no Sparkle, ~5 Adobe teammates as the v1 audience)
- `.planning/REQUIREMENTS.md` — HARDEN-01..04 acceptance lines + Out-of-Scope list (DIST-V2-01/02)
- `.planning/ROADMAP.md` §"Phase 10: Hardening & Release" — the four success criteria verbatim
- `./CLAUDE.md` — Rust workspace tech-stack table (cargo-bundle 0.10, lipo, hdiutil, rust-toolchain.toml pin to 1.88, `cargo deny` already listed)

### Existing surface to extend (not rewrite)
- `deny.toml` — current advisories/licenses/bans/sources; Phase 10 adds the `[bans] unsafe` block per D-12
- `.github/workflows/release.yml` — existing arm64 + x86_64 build jobs; Phase 10 grows the `release` job per D-19
- `.github/workflows/ci.yml` — current lint + commitlint + (Phase 9-added) `persist-e2e` job; Phase 10 layers snapshot + VT-corpus + perf + grep gates
- `crates/vector-app/resources/Fonts/JetBrainsMono-Regular.ttf` — the pinned font per D-04
- `crates/vector-render/` — host crate for the renderer pipelines that snapshot tests will drive
- `crates/vector-term/` — host crate where the VT conformance corpus lives per D-07
- All `crates/*/Cargo.toml` — `insta` is already a dev-dep workspace-wide; no Cargo.toml churn for HARDEN-01 lib choice

### Pitfalls that must hold
- `PITFALLS.md` PITFALL 14 — manual Debug on every token-bearing struct (D-11 audits this)
- `PITFALLS.md` PITFALL 22 — no mosh-style protocol; tmux is the user's job (out of scope here)
- The full "looks done but isn't" PITFALLS checklist — Phase 10 plan should include a gate that each item has a corresponding test or explicit acceptance

### Prior verifications to honor (no regressions)
- `.planning/phases/09-persistence-reconnect-tmux-auto-attach/09-VERIFICATION.md` — Phase 9 status `human_needed`; v1.0.0 tag waits on PERSIST-04 sign-off per D-21
- `.planning/phases/01-foundation-ci-dmg-pipeline/` SUMMARYs — the DMG pipeline groundwork
- `.planning/phases/08-vs-code-remote-tunnels-connect/` SUMMARYs — Dev Tunnels token handling (D-11 sweep target)
- `.planning/phases/06-github-auth-codespaces-picker/` SUMMARYs — GitHub OAuth + Codespaces token handling (D-11 sweep target)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`insta`** — workspace-wide dev-dep already declared in 10 of 10 crates. Snapshot infrastructure is plug-and-play.
- **`deny.toml`** — well-formed config with advisories/licenses/bans/sources; needs only the new `[bans] unsafe` clause per D-12.
- **`release.yml`** workflow — arm64 + x86_64 build jobs work; the `release` job (currently a stub) is where `lipo` + `cargo bundle` + `hdiutil` + asset upload land per D-19.
- **`JetBrainsMono-Regular.ttf`** — only bundled font; pinning satisfies D-04.
- **`vector-render`** glyph-atlas + ReconnectPass + ToastPass pipelines — directly snapshottable per D-02 scenes (c) and (b).

### Established Patterns
- Manual `Debug` impls on token-bearing structs (PITFALL 14) — D-11's audit verifies this scaled across all auth/token modules.
- `rust-toolchain.toml` pinned to 1.88; `MACOSX_DEPLOYMENT_TARGET=13.0` env var across CI and release. Phase 10 plans keep these pins.
- CI jobs use `Swatinem/rust-cache@v2` with named shared keys (`ci-lint`, `rel-arm64`, `rel-x86_64`). Phase 10 jobs follow the same pattern.

### Integration Points
- **`crates/vector-render-snapshots/`** (new crate per D-05) — depends on `vector-render`, `image`, and a comparator crate. New addition to workspace `members`.
- **`crates/vector-term/tests/vt_conformance/`** (new test dir per D-07) — extends existing `crates/vector-term/tests/` integration tests; depends on `alacritty_terminal` (already a dep).
- **`.github/workflows/ci.yml`** — adds 2–3 new jobs (snapshot, vt-conformance, token-redaction-grep) following the existing job-shape; perf gate likely a fourth job or rolled into snapshot.
- **`.github/workflows/release.yml`** — `release` job gains lipo + bundle + dmg + checksum + upload steps. No new workflows; only the existing one grows.
- **`README.md`** — new top-of-file `## Install` section per D-17; current README is sparse so the install block becomes section 1.

</code_context>

<specifics>
## Specific Ideas

- DMG name format `Vector-1.0.0-universal.dmg` plus a sibling `Vector-1.0.0-universal.dmg.sha256` — explicit per D-15/D-16.
- README copy block contains literally `xattr -dr com.apple.quarantine /Applications/Vector.app` followed by `open /Applications/Vector.app` — that exact two-line shell snippet is the audience's install ritual.
- v1.0.0 release notes hand-written; structure suggested: "What's in v1" (51-requirement summary at teammate-readable granularity) + "What's out of v1" (signing, auto-update, public OSS, backlog 999.x ideas) + "How to install" (link back to README).
- Snapshot scene list (initial 4, expandable to ~8 during planning):
  1. plain text with mixed Unicode + emoji
  2. alt-screen with colors + cursor + selection
  3. reconnect status bar + tab badge (Phase 9 ReconnectPass)
  4. split panes + scrollback

</specifics>

<deferred>
## Deferred Ideas

- Vendoring vttest's full corpus as `#[ignore]`d aspirational tests — bigger coverage of obsolete VT100 sequences we don't care about for v1. Re-evaluate if v2 expands terminal compatibility claims.
- True end-to-end VT tests that spawn the `vector` binary and drive it over PTY. Requires a windowing+GPU CI rig; deferred to a v2 perf/integration harness phase.
- `scripts/trust-vector.sh` helper for xattr. Not needed for 5 teammates; reconsider when the audience grows beyond `gh release` curl-ability.
- Per-PR snapshot baseline preview comments (`actions/snapshot-bot`) — nice-to-have for snapshot-noisy PRs; out of scope when only one developer is regularly building.
- Auto-generated release notes for v1.0.0 — explicitly rejected per D-18 (v1 deserves a real story).
- Code signing + notarization + Sparkle auto-update — already deferred via DIST-V2-01 / DIST-V2-02.
- Public open-source push, contributor docs, CODEOWNERS, issue templates — out of v1 scope per PROJECT.md audience constraint.
- Apple Silicon vs Intel binary size profiling / lipo dead-stripping — defer until v1.0.0 lands and we measure actual DMG bloat.

</deferred>

<addenda>
## Addenda (2026-05-26, post-research resolutions)

Following 10-RESEARCH.md's "Open Questions" surface, the following decisions are now locked. These supersede or refine the original D-01..D-21 set where noted.

- **D-12 superseded → D-22 (cargo-geiger swap):** The `cargo-deny [bans] unsafe` knob does not exist in any cargo-deny version. HARDEN-03's unsafe gate uses **`cargo-geiger 0.13 --forbid-only`** with a JSON allowlist. Same intent, working tool. The allowlist values from D-12 (`objc2`, `objc2-app-kit`, `objc2-foundation`, `wgpu`, `alacritty_terminal`, `crossfont`, `portable-pty`) carry forward into the cargo-geiger allowlist verbatim. cargo-deny continues to handle advisories/licenses/bans/sources per D-13.
- **D-23 (perf gate scope):** Perf gate (idle CPU <1%, `cat large.log` at vsync cap) is a **hard merge gate on macos-14 arm64 only**. macos-15-intel runs the same probe as **advisory/log-only** (records the number, never fails). Reason: intel runners' Metal driver + float precision differ enough to cause flapping; arm64 is the primary v1 target.
- **D-24 (snapshot cadence):** HARDEN-01 renderer snapshot suite runs on **every PR** as a merge-blocking CI gate. Matches the ROADMAP wording ("CI gate that blocks merges on regression"). Goldens churn is acceptable cost.
- **D-25 (PERSIST-04 hard gate):** Phase 10 plans treat **PERSIST-04 = Complete** as a hard prerequisite for the v1.0.0 tag task. The final release-cut task includes an explicit pre-flight check that grep-asserts `PERSIST-04` is `Complete` in REQUIREMENTS.md (or its tracking field). If not, the release task halts — no soft-fail, no "ship with debt." This preserves ROADMAP's "Depends on: Phase 9 (all v1 features in place)" wording.
- **D-26 (insta is NOT a workspace dev-dep):** CONTEXT.md's "insta workspace-wide dev-dep" claim was inaccurate (verified via Cargo.lock — zero hits). Plans MUST add `insta 1.47.2` fresh, scoped to the new `crates/vector-render-snapshots/` per D-05. Do not add `insta` to the workspace root.
- **D-27 (snapshot harness is reusable):** `crates/vector-render/tests/common/offscreen.rs` already provides `RenderContext::new_offscreen` + `FontStack::load_bundled(1.0, 14.0)` + `Compositor::render_offscreen_with`. Snapshot tests are glue over this harness, not a new init from scratch.
- **D-28 (VT corpus precedents exist):** `crates/vector-term/tests/` already has 1:1 precedents for every D-07 scenario — `alt_screen_1049.rs`, `decstbm_scroll_region.rs`, `ed_el_erase.rs`, `dcs_dispatch.rs`, `osc52.rs`. HARDEN-02 is "relocate + extend into a single corpus directory" per D-07, not from-scratch authoring.
- **D-29 (Pitfall-14 static gate already ships):** `crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs` enforces both the `#[derive(Debug)]` ban and the `tracing::*!` token-field ban statically. HARDEN-03's D-11 audit can rely on this gate for the static side; D-11's NEW work is the **runtime** grep gate (record `RUST_LOG=debug` from wiremock-backed auth tests, regex for `gho_|ghp_|eyJ`).
- **D-30 (v1.0.0 release-notes location — planner's discretion):** Whether v1.0.0's hand-written notes live in a committed `CHANGELOG.md` or only in the GitHub release body is left to the planner. Future point releases (v1.0.1+) use `gh release create --generate-notes` per D-18 regardless.
- **D-31 (workspace version bump):** `release.yml` currently uses CalVer `2026.5.10`. Phase 10's final release-cut plan bumps workspace version to `1.0.0` (matches D-20's `v1.0.0` tag style).

</addenda>

---

*Phase: 10-hardening-release*
*Context gathered: 2026-05-26*
*Addenda appended: 2026-05-26*
