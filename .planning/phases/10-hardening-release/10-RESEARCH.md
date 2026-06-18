# Phase 10: Hardening & Release — Research

**Researched:** 2026-05-26
**Domain:** CI hardening (renderer snapshot suite, VT conformance, unsafe-dep auditing, token-leak grep), unsigned Universal DMG release pipeline.
**Confidence:** HIGH

## Summary

Phase 10 wraps Vector v1.0.0 by **adding four CI gates and growing the existing `release.yml`** — no greenfield infrastructure. The codebase already provides every primitive needed: an offscreen-render harness in `crates/vector-render/tests/common/offscreen.rs` (Compositor + bundled JetBrainsMono load path), a battle-tested VT-test pattern in `crates/vector-term/tests/` (every CONTEXT D-07 scenario has a working analogue test today), an arch-lint regex gate in `crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs` already enforcing Pitfall 14, and a fully-working two-arch build matrix in `.github/workflows/release.yml` that already produces a Universal DMG via `cargo xtask dmg --universal` and uploads it via `gh release create`.

**Two findings invalidate / sharpen CONTEXT.md assumptions** and the planner must absorb them before writing tasks:

1. **`cargo-deny` does NOT have a `[bans] unsafe` knob.** D-12 names a feature that does not exist in any cargo-deny version (current is 0.19.7, 2026-05-22). The standard tool for "ban unaudited unsafe in dependencies" is **`cargo-geiger 0.13.0`** with its `--forbid-only` mode + allowlist file. The intent of D-12 is achievable; the mechanism named in CONTEXT.md is not. The planner must surface this to the user — either reframe D-12 onto `cargo-geiger`, or downgrade the gate to an advisory `unmaintained = "deny"` rule (already partially in `deny.toml`). See User Constraints → Conflict Surface below.
2. **`insta` is NOT currently in any Cargo.toml across the workspace.** CONTEXT.md "Reusable Assets" says insta is a workspace-wide dev-dep in 10 of 10 crates — `grep` confirms zero hits in any `Cargo.toml` and zero hits in `Cargo.lock`. The planner adds it fresh. Good news: `insta 1.47.2` (2026-03-30) does have `assert_binary_snapshot!` (experimental) for byte-vector PNG storage, validating D-06 architecturally — but the comparator must run perceptual diff BEFORE the byte-equality compare, since insta itself does only byte-for-byte.

**Primary recommendation:** Plan in waves so the four success criteria become four CI jobs with deterministic failure signals: (a) renderer snapshot diff > threshold; (b) VT corpus assertion fail / perf probe > budget; (c) `cargo-geiger` forbidden-unsafe match OR `RUST_LOG=debug` grep hit; (d) `gh release view v1.0.0` returns non-zero AND `lipo -info` confirms fat binary. Run all four as gating CI jobs in `ci.yml`; release.yml only needs the README + asset-name + checksum delta.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

**HARDEN-01 — Renderer Snapshot Suite**
- **D-01:** Scene-based snapshot fixtures (not full-frame, not glyph-atlas-only). Author 4–8 curated test scenes that exercise the visible compositing stack end-to-end.
- **D-02:** Initial fixture set covers: (a) plain text with mixed Unicode + emoji, (b) full alt-screen with colors + cursor + selection, (c) reconnect status bar + tab badge (re-uses Phase 9 ReconnectPass), (d) split panes + scrollback. Plan may add up to 4 more scenes during research.
- **D-03:** Comparator is **perceptual** — delta-E or SSIM with threshold ~2.0 to absorb sub-pixel/antialias drift across arm64 and x86_64 CI runners without flapping. Planner picks the exact library (`image-compare`, `imageproc`, or hand-rolled on `image` crate) based on what's lightest in the dep graph.
- **D-04:** Pinned font for all snapshot tests = the existing `crates/vector-app/resources/Fonts/JetBrainsMono-Regular.ttf` (the only bundled font). No system fallback during snapshot tests — failures must reproduce locally.
- **D-05:** Snapshot crate placement: new `crates/vector-render-snapshots/` test-only crate (keeps `vector-render` build-time clean of `image`/diff deps; lets snapshot tests pull in heavy comparators without leaking into the binary).
- **D-06:** `insta` is the runner (already a workspace-wide dev-dep). Goldens committed to git (PNGs are ~4–16 KB each at terminal sizes; no git-lfs needed for ≤16 fixtures).

**HARDEN-02 — VT Conformance Corpus**
- **D-07:** Hand-craft an 8-scenario corpus mapped 1:1 to the ROADMAP success criterion (alt-screen, scroll regions, tab stops, ED/EL, mouse 1006, OSC 52 round-trip, bracketed paste, DECSCUSR). Each test maps to a documented PITFALLS.md item. Lives in `crates/vector-term/tests/vt_conformance/` (one file per scenario or a single `vt_conformance.rs` — planner decides).
- **D-08:** Drive the corpus **against `alacritty_terminal::Term` in unit tests** — feed escape sequences, assert resulting grid/cursor/mode state via Term's API. Fast (<1s for full corpus), zero windowing/GPU infrastructure required.
- **D-09:** Out of scope for v1 corpus: vendoring vttest's full input set; spawning the real `vector` binary end-to-end for VT input (deferred to v2).
- **D-10:** Perf gate metrics for success criterion #2 — Claude's Discretion (see below).

**HARDEN-03 — Hardening (`cargo deny` + token redaction)**
- **D-11:** Token redaction = **heavy audit**: sweep every token-bearing struct; confirm manual `Debug` impls per Pitfall 14; add workspace clippy lint or arch-test that fails the build on `#[derive(Debug)]` for any struct in auth/token modules; add CI step recording a smoke run (login + list codespaces / list tunnels) with `RUST_LOG=debug` and greps the tracing output for `gho_`, `ghp_`, `eyJ`. Zero matches required.
- **D-12:** `cargo deny` — add `[bans] unsafe = "deny"` with an explicit allowlist of crates we accept unsafe in: `objc2`, `objc2-app-kit`, `objc2-foundation`, `wgpu`, `alacritty_terminal`, `crossfont`, `portable-pty`. Any other unsafe-bearing dep added later must be explicitly added to the allowlist with a one-line reason comment.
- **D-13:** Existing `deny.toml` advisories/licenses/bans/sources blocks stay as-is. Only the `unsafe` knob is new for Phase 10.
- **D-14:** Release-profile binding — the `unsafe` policy applies to the full dep graph regardless of profile.

**HARDEN-04 — Tagged Release**
- **D-15:** DMG asset name = `Vector-{version}-universal.dmg` (e.g. `Vector-1.0.0-universal.dmg`).
- **D-16:** Companion artifact: `Vector-{version}-universal.dmg.sha256` checksum file uploaded alongside.
- **D-17:** README install instructions: first content section is `## Install`; copy-paste block contains `xattr -dr com.apple.quarantine /Applications/Vector.app` and `open /Applications/Vector.app`; one-paragraph `### Why the xattr step?` follows.
- **D-18:** Release notes: v1.0.0 ships with hand-written notes ("what's in v1" / "what's out of v1"). Future point releases use `gh release create --generate-notes`.
- **D-19:** Universal-binary assembly happens in `release.yml`: existing arm64 + x86_64 build jobs feed a new `package` job that runs `lipo` → `cargo bundle --release` → `hdiutil create` → `shasum -a 256` → `gh release upload`. Existing two build jobs already work — only the `release` job needs to grow.
- **D-20:** Tag style = `v1.0.0` (matches existing trigger).

**Phase 9 Coupling**
- **D-21:** Phase 10 planning may proceed in parallel with Phase 9 HUMAN-UAT walks. **The v1.0.0 tag itself must wait until PERSIST-04 is signed off.** The plan's final release-cut task is gated on PERSIST-04 = Complete.

### Claude's Discretion
- Perf gate measurement approach (idle CPU < 1%, `cat large.log` at vsync cap).
- Exact perceptual-tolerance library for HARDEN-01 D-03.
- Whether the token-redaction grep gate runs against a recorded tracing-output file in git or freshly-recorded each CI run.
- VT conformance tests as `vt_conformance.rs` with sub-tests vs. one file per scenario.
- Whether `lipo` runs in a new GH Actions job or inside the existing `release` job.

### Deferred Ideas (OUT OF SCOPE)
- Vendoring vttest's full corpus as `#[ignore]`d aspirational tests.
- True end-to-end VT tests that spawn the `vector` binary.
- `scripts/trust-vector.sh` helper for xattr.
- Per-PR snapshot baseline preview comments.
- Auto-generated release notes for v1.0.0.
- Code signing + notarization + Sparkle auto-update (DIST-V2-01/02).
- Public open-source push, contributor docs, CODEOWNERS, issue templates.
- Apple Silicon vs Intel binary size profiling / lipo dead-stripping.

### Conflict Surface (research-discovered — planner MUST resolve with user before tasking)

1. **D-12 mechanism is invalid as written.** `cargo-deny` (current 0.19.7, 2026-05-22) has no `[bans] unsafe` knob. Its `[bans]` table accepts `multiple-versions`, `wildcards`, `deny`, `skip`, `skip-tree`, `allow`, `features` — nothing about unsafe code. The standard tool for the D-12 intent ("block unaudited unsafe in release-profile dependencies") is **`cargo-geiger 0.13.0`** with `--forbid-only` mode + allowlist. See § "Standard Stack" and § "Open Questions" for the resolution options.
2. **D-06 reusable-asset claim is wrong.** `insta` is NOT currently a workspace dev-dep — zero hits in any `Cargo.toml` or in `Cargo.lock`. The plan must add it fresh (and only in `crates/vector-render-snapshots/Cargo.toml` per D-05 to keep production crates clean). This is a tasking implication, not a decision change.

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| **HARDEN-01** | Renderer snapshot test suite runs headless against a pinned font and a perceptual-tolerance comparator; CI gate on regression. | Existing offscreen harness at `crates/vector-render/tests/common/offscreen.rs` (already builds Compositor headlessly via `RenderContext::new_offscreen` + `FontStack::load_bundled`) — directly reusable. `image-compare 0.5.0` provides `rgb_hybrid_compare` returning a 0..1 score → maps cleanly to the D-03 ~2.0 perceptual threshold. `insta 1.47.2` has experimental `assert_binary_snapshot!` for PNG byte storage, but perceptual diff must precede insta's byte-equality compare. |
| **HARDEN-02** | VT conformance corpus runs in CI; perf gate enforces idle CPU <1% and `cat large.log` at vsync cap. | Every D-07 scenario has a working precedent: `alt_screen_1049.rs`, `decstbm_scroll_region.rs`, `ed_el_erase.rs`, `dcs_dispatch.rs` (mouse 1006 + DECSCUSR + bracketed paste already covered), `osc52.rs` (raw + DCS-wrapped round-trip). Term API surface (`feed`/`grid`/`cursor`/`mode`/`damage`) is sufficient. Tab-stops have a precedent in `decstbm_scroll_region.rs::hts_sets_tab_stop_then_cht_jumps_to_it`. |
| **HARDEN-03** | `cargo deny` policy blocks unaudited unsafe in release-profile dependencies; `grep` of `tracing` output shows zero token-shaped strings. | Pitfall 14 is already enforced by `vector-arch-tests/tests/no_token_in_debug_or_log.rs` (regex bans `#[derive(...Debug...)]` near token fields AND bans `tracing::*!(... access_token …)`). The unsafe gate needs the mechanism swap to `cargo-geiger` per Conflict Surface above. The grep gate is a fresh CI job — recommended approach in § Architecture Patterns. |
| **HARDEN-04** | Tagging `v1.0.0` publishes the unsigned Universal `Vector.dmg` to GitHub Releases with install instructions front-and-center. | `release.yml` already does almost everything: arm64+x86_64 build matrix, `cargo xtask dmg --universal`, fat-binary verification via `lipo -info`, `git-cliff --latest`, `gh release create`. Phase 10 delta is small: (1) rename asset to `Vector-{version}-universal.dmg` per D-15 (already named that way in the workflow!), (2) compute and upload `.sha256` sidecar per D-16, (3) replace `git-cliff` with hand-written notes for v1.0.0 only per D-18, (4) restructure README so `## Install` is section 1 per D-17. |

</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **No code signing, no notarization, no Sparkle for v1** (DIST-V2-01/02 deferred).
- **macOS 13 (Ventura) baseline** via `MACOSX_DEPLOYMENT_TARGET=13.0`.
- **Unsigned `.dmg` only.** The xattr ritual is the user-facing install step.
- **~5 Adobe teammates audience.** Don't ship contributor docs, telemetry, crash reporting.
- **Workflow:** commit each logical stage separately; **do not push** — user reviews and pushes asynchronously.
- **Rust 1.88.0 pinned** via `rust-toolchain.toml`.
- **`unsafe_code = "deny"` is a workspace lint** in `Cargo.toml`; only `vector-app` opts back in (AppKit FFI). `vector-mux` also contains `unsafe` (libproc calls). These are first-party code, not dep-tree unsafe — the HARDEN-03 unsafe gate targets the dep tree.
- **`tokio` runs on a separate thread; `winit::EventLoop` owns main** — irrelevant to Phase 10 except for "don't break it in snapshot test infra."

## Standard Stack

### Core (additions for Phase 10)

| Library | Verified Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| **`insta`** | 1.47.2 (2026-03-30) | Snapshot test runner + reviewing tool (`cargo insta review`) | De-facto Rust snapshot library; `assert_binary_snapshot!` (experimental) supports our PNG-byte storage need; pairs with `cargo insta` CLI for golden review. |
| **`image-compare`** | 0.5.0 (2025-08-18) | Perceptual SSIM / RGB hybrid comparator | Returns 0..1 similarity score with `rgb_hybrid_compare` (decomposes RGB into structure + color channels in YUV/YUVA). Smaller dep footprint than `imageproc` (which pulls `nalgebra`). Cross-platform verified on arm64-apple-darwin. |
| **`image`** | 0.25.x (workspace transitive via `image-compare`) | PNG read/write | Required for golden PNG → `image::DynamicImage` round-trip before diffing. |
| **`cargo-geiger`** | 0.13.0 (2025-08-31) | Unsafe-code surface auditor for dep graph | The actual tool for "ban unaudited unsafe in dependencies" (cargo-deny does not have this feature). `--forbid-only` mode is fast enough for CI gates. Allowlist file `cargo-geiger.allowlist.json` at the root crate level. **See Conflict Surface — needs user confirmation to swap from D-12.** |

### Supporting (already in workspace, used by Phase 10)

| Library | Workspace Version | Purpose | Use in Phase 10 |
|---------|---------|---------|-------------|
| `alacritty_terminal` | 0.26 | VT parser + grid + cursor + mode | HARDEN-02 corpus drives `Term` directly. |
| `wgpu` | 29 | GPU rendering | HARDEN-01 offscreen render of test scenes. |
| `vector-fonts::FontStack` | local | Bundled font loader | `FontStack::load_bundled(1.0, 14.0)` already used in `render/tests/common/offscreen.rs`. |
| `vector-render::Compositor` | local | Render pipeline | `Compositor::render_offscreen_with(...)` returns RGBA pixels — direct input to PNG encode + compare. |
| `regex` | 1 | Token-shape regex for grep gate | Match `gho_`, `ghp_`, `eyJ[A-Za-z0-9_-]{10,}`. |

### Already-installed and untouched

| Library | Why It's Already Present | Phase 10 Action |
|---------|--------------------------|-----------------|
| `cargo-deny 0.19.7` | Workspace policy: advisories, licenses, bans, sources | No changes to `deny.toml` for the unsafe knob (the knob doesn't exist); the existing config stays as-is. |
| `cargo-husky`, `convco`, `git-cliff` | Commit hygiene + release notes | `git-cliff` is bypassed for v1.0.0 per D-18 (hand-written notes); resumes for v1.0.1+. |
| `cargo-machete` | Unused-dep check | Already in `ci.yml`. New `vector-render-snapshots` deps must clear it. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `image-compare 0.5.0` | `imageproc::stats::root_mean_squared_error` + hand-rolled SSIM | `imageproc` pulls `nalgebra` (~50 transitive crates); we'd reimplement SSIM. `image-compare` is purpose-built and lighter. |
| `image-compare 0.5.0` | Hand-rolled delta-E on `image` crate | Delta-E (CIE94 / CIEDE2000) is correct for perceptual color, but the threshold ~2.0 in D-03 is ambiguous between Lab delta-E and SSIM score. `image-compare`'s SSIM-based score ∈ [0,1] with threshold 0.98 is the precedent terminal-snapshot projects use (Wezterm doesn't have this; Zed uses screenshot tests via custom Metal capture). |
| `cargo-geiger` | Audit-only `cargo geiger --forbid-only` (advisory, no gate) | Hits the same intent; weaker enforcement. Acceptable fallback if the user rejects cargo-geiger as a build-time gate (it can be slow on cold cache — ~30–60s on this dep tree). |
| `cargo-geiger` | Custom `cargo metadata` parser that walks the dep tree and rejects any crate with `unsafe` blocks not on an allowlist | Reinvents cargo-geiger; one more piece of homegrown CI. Skip. |
| `criterion` for perf gate | Custom probe binary that wall-clocks idle + paste | `criterion` is benchmark-oriented (statistical), not threshold-oriented; flaky on shared GH runners. A custom probe is simpler. |
| Native Linux runner for `cargo-deny` job | Use existing macOS runner | Existing `deny` job already runs on `ubuntu-latest`. cargo-geiger can also run on `ubuntu-latest` — faster cold cache than macOS. |

**Installation (new for Phase 10):**

```bash
# Workspace dev-deps (added in vector-render-snapshots/Cargo.toml only)
cargo add --dev --package vector-render-snapshots insta@1.47 image-compare@0.5 image@0.25

# CI tooling (installed in the new CI jobs, not the workspace)
cargo install cargo-geiger@0.13 --locked
cargo install cargo-insta@1.47 --locked   # for `cargo insta review` ergonomics
```

**Version verification (all checked against crates.io 2026-05-26):**

| Crate | Version Used | Published |
|-------|--------------|-----------|
| insta | 1.47.2 | 2026-03-30 |
| image-compare | 0.5.0 | 2025-08-18 |
| cargo-deny | 0.19.7 (existing CI action) | 2026-05-22 |
| cargo-geiger | 0.13.0 | 2025-08-31 |

## Architecture Patterns

### Recommended Project Structure (deltas only)

```
crates/
├── vector-render-snapshots/          # NEW per D-05 — test-only crate
│   ├── Cargo.toml                    # dev-deps: insta, image-compare, image; deps: vector-render, vector-term, vector-fonts
│   ├── src/lib.rs                    # empty or thin re-export; required for `cargo test -p vector-render-snapshots`
│   └── tests/
│       ├── common/
│       │   └── mod.rs                # render_scene() + perceptual_diff() + load_golden() helpers
│       ├── goldens/                  # PNG fixtures, ~4–16 KB each
│       │   ├── plain_text_unicode_emoji.png
│       │   ├── alt_screen_colors_selection.png
│       │   ├── reconnect_bar_tab_badge.png
│       │   └── split_panes_scrollback.png
│       ├── plain_text_unicode_emoji.rs
│       ├── alt_screen_colors_selection.rs
│       ├── reconnect_bar_tab_badge.rs
│       ├── split_panes_scrollback.rs
│       └── no_tokio_main.rs          # required by existing ci.yml arch-lint
│
crates/vector-term/
└── tests/
    └── vt_conformance/               # NEW per D-07 (planner's call: subdir vs single file — recommend subdir for one-file-per-scenario, easier to read in CI logs)
        ├── alt_screen_1049.rs        # COPIED & extended from existing tests/alt_screen_1049.rs
        ├── scroll_regions.rs         # consolidates existing tests/decstbm_scroll_region.rs
        ├── tab_stops.rs              # extracted from decstbm_scroll_region.rs
        ├── ed_el_erase.rs            # COPIED & extended from existing tests/ed_el_erase.rs
        ├── mouse_1006.rs             # extracted from existing tests/dcs_dispatch.rs::mouse_mode_1006_sgr_sets_state
        ├── osc52_round_trip.rs       # COPIED from existing tests/osc52.rs
        ├── bracketed_paste.rs        # extracted from existing tests/dcs_dispatch.rs::bracketed_paste_mode_2004_sets_state
        └── decscusr.rs               # extracted from existing tests/dcs_dispatch.rs::decscusr_cursor_shape_sets_state

# Workspace root (deltas only)
cargo-geiger.allowlist.json            # NEW — allowlist of crates permitted to use unsafe (D-12 intent)

.github/workflows/
├── ci.yml                             # +3 jobs: snapshot, vt-perf, token-grep (cargo-geiger handled by existing `deny` job swap or new `unsafe-audit` job)
└── release.yml                        # delta: add .sha256 sidecar, swap git-cliff → hand-written notes for v1.0.0 only

README.md                              # restructure: ## Install becomes section 1, ## Why the xattr step? as subsection

deny.toml                              # UNCHANGED for unsafe (cargo-deny lacks the knob); minor advisory cleanup if any new CVEs surface
```

### Pattern 1: Headless Snapshot Render (HARDEN-01)

**What:** Render a curated scene via the existing offscreen harness, encode as PNG, and perceptual-diff against a committed golden.

**When to use:** For each of the 4 (extending to ≤8) scenes in D-02.

**Example** (idiomatic — derived from existing `crates/vector-render/tests/snapshot_singlecell.rs`):

```rust
// Source: crates/vector-render/tests/common/offscreen.rs (existing, reusable)
// crates/vector-render-snapshots/tests/plain_text_unicode_emoji.rs (new)

use image::{ImageBuffer, Rgba};
use image_compare::Algorithm;
use vector_fonts::FontStack;
use vector_render::{Compositor, RenderContext};
use vector_term::Term;

const PERCEPTUAL_THRESHOLD: f64 = 0.98; // SSIM ≥ 0.98 (D-03 ~2.0 maps to score gap ≤ 0.02)

fn render_scene(width: u32, height: u32, scene: impl FnOnce(&mut Term)) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let ctx = RenderContext::new_offscreen(width, height).ok()?;
    let font_stack = FontStack::load_bundled(1.0, 14.0).ok()?;
    let mut comp = Compositor::new_with(&ctx.device, &ctx.queue, ctx.format, ctx.width, ctx.height, font_stack).ok()?;
    let mut term = Term::new(80, 24, 1000);
    scene(&mut term);
    let frame = comp.render_offscreen_with(&ctx.device, &ctx.queue, ctx.width, ctx.height, &mut term, None).ok()?;
    // BGRA → RGBA channel swizzle if needed (frame.format check)
    Some(ImageBuffer::from_raw(frame.width, frame.height, frame.pixels)?)
}

#[test]
fn plain_text_unicode_emoji_scene() {
    let Some(actual) = render_scene(800, 480, |term| {
        term.feed("hello, world\n".as_bytes());
        term.feed("日本語テスト\n".as_bytes());
        term.feed("emoji: 🎉 ✨ 🚀\n".as_bytes());
        term.feed("rust: => let x = 1;\n".as_bytes());
    }) else {
        eprintln!("SKIP: no Metal adapter available (Linux CI host?)");
        return;
    };

    let golden_path = "tests/goldens/plain_text_unicode_emoji.png";
    let expected = image::open(golden_path).expect("golden missing — run `INSTA_UPDATE=auto cargo test`").into_rgba8();
    let actual_rgb = image::DynamicImage::ImageRgba8(actual.clone()).into_rgb8();
    let expected_rgb = image::DynamicImage::ImageRgba8(expected).into_rgb8();

    let result = image_compare::rgb_hybrid_compare(&actual_rgb, &expected_rgb).expect("compare");
    if result.score < PERCEPTUAL_THRESHOLD {
        // Write actual.png next to golden for review
        let diff_path = format!("{}.diff.png", golden_path);
        actual.save(&diff_path).ok();
        panic!("snapshot diff: score={:.4} < threshold={} (see {})", result.score, PERCEPTUAL_THRESHOLD, diff_path);
    }
}
```

**Note on `insta`:** the recommended pattern is to NOT use `assert_binary_snapshot!` directly (it does byte-for-byte). Use insta only for the *metadata* sidecar (scene name, timestamp, threshold used) and run the perceptual diff manually against the committed PNG. This is the same pattern as Servo's WPT image-test harness.

### Pattern 2: VT Conformance Unit Tests (HARDEN-02)

**What:** Feed escape sequences to `vector_term::Term`, assert grid/cursor/mode state.

**Example** (idiomatic — extends `crates/vector-term/tests/dcs_dispatch.rs`):

```rust
// crates/vector-term/tests/vt_conformance/mouse_1006.rs

use alacritty_terminal::term::TermMode;
use vector_term::Term;

#[test]
fn mouse_1006_sgr_mode_sets_state() {
    let mut term = Term::new(80, 24, 1000);
    assert!(!term.mode().contains(TermMode::SGR_MOUSE));
    term.feed(b"\x1b[?1000h\x1b[?1006h");
    assert!(term.mode().contains(TermMode::MOUSE_REPORT_CLICK), "DECSET 1000 must enable click reporting");
    assert!(term.mode().contains(TermMode::SGR_MOUSE), "DECSET 1006 must enable SGR encoding");
    // Pitfall 8 cross-reference: SGR 1006 supports coords beyond col 223
    term.feed(b"\x1b[?1006l\x1b[?1000l");
    assert!(!term.mode().contains(TermMode::SGR_MOUSE));
}
```

### Pattern 3: Perf Gate as a Probe Binary (HARDEN-02 perf gate)

**What:** A tiny `vector-perf-probe` binary that boots a headless `Compositor`, drives a fixed PTY-output workload, and emits a JSON report (`{ idle_cpu_pct: f64, paste_render_fps: f64 }`). CI reads the JSON and fails if `idle_cpu_pct > 1.0` or `paste_render_fps < 55.0` (vsync cap 60, 5fps tolerance for runner jitter).

**Why this over `criterion`:** criterion produces statistical distributions; we need a threshold gate. A custom probe is ~150 lines and produces deterministic numbers. It can live as `crates/vector-render/examples/perf_probe.rs` (no new crate). Sampling: 5s of idle, then 5s of `cat large.log` (a committed 1 MB log fixture).

**Risk note:** GitHub macos-14 and macos-15-intel runners are shared VMs with unpredictable jitter. **Expect occasional FAILs at borderline thresholds; the perf gate must use `continue-on-error: true` for v1** to avoid blocking unrelated PRs. Treat it as an advisory signal, not a merge blocker. Document this trade-off in the plan.

### Pattern 4: Token-Leak Grep Gate (HARDEN-03)

**What:** A CI job that runs a deterministic auth-adjacent smoke command with `RUST_LOG=debug`, captures stderr/stdout, and greps for token-shaped strings. Recommended approach per Claude's Discretion: **freshly recorded each CI run**, NOT a checked-in fixture. Rationale: a checked-in fixture rots silently when new tracing call sites are added; a fresh recording catches regressions the moment they land.

**Example CI step:**

```yaml
- name: Token-leak grep gate
  run: |
    set -euo pipefail
    # Use the existing wiremock-backed unit tests with RUST_LOG=debug;
    # these exercise both GitHub OAuth and Microsoft device-flow paths
    # without touching live credentials (wiremock is already a workspace dev-dep).
    RUST_LOG=debug cargo test --package vector-codespaces --package vector-tunnels \
        --tests auth -- --test-threads=1 2>&1 | tee target/auth-trace.log
    # Token-shape patterns: GitHub PAT prefixes + JWT prefix
    if grep -E '(gho_|ghp_|gha_|ghs_|eyJ[A-Za-z0-9_-]{10,})' target/auth-trace.log; then
      echo "::error::Token-shaped string found in tracing output"
      exit 1
    fi
```

**Why this is sound:** the existing wiremock-backed auth tests in `vector-codespaces/tests/` and `vector-tunnels/tests/` already exercise the full device-flow code paths with synthetic tokens like `gho_FAKE_TOKEN_FOR_TESTING`. If those test fixtures ever land in tracing output (they will look like real tokens — `gho_` prefix), the grep catches it.

**Edge case to confirm in implementation:** the test fixtures themselves must use the `gho_FAKE_...` pattern (so the grep is exercised). Cross-check `vector-codespaces/tests/fixtures/` and confirm token-shaped values exist; if they're masked to `redacted`, change fixtures to `gho_FAKE...` to make the grep gate self-verifying.

### Pattern 5: Unsafe-dep Audit (HARDEN-03 cargo-deny replacement)

**What:** Use `cargo-geiger --forbid-only` with an allowlist JSON to fail CI when any crate uses unsafe outside the allowlist.

**Example CI step:**

```yaml
- name: Unsafe-dep audit (D-12 intent via cargo-geiger)
  run: |
    set -euo pipefail
    cargo install cargo-geiger@0.13 --locked
    # --forbid-only: scan for #![forbid(unsafe_code)] at crate root, fast (~5s on warm cache).
    # Output is JSON when piped to --output-format Json; we parse with jq.
    cargo geiger --forbid-only --output-format Json > target/geiger.json
    # Allowlist comparison: any crate name in target/geiger.json that's not in allowlist fails.
    python3 .github/scripts/check-geiger.py cargo-geiger.allowlist.json target/geiger.json
```

**`cargo-geiger.allowlist.json`** (verified against actual dep tree — `vector-mux` (libproc), `vector-app` (AppKit FFI) are first-party and excluded from this gate; the gate targets external crates):

```json
{
  "allowlist": [
    "objc2", "objc2-app-kit", "objc2-foundation", "objc2-core-foundation",
    "objc2-core-graphics", "objc2-quartz-core",
    "wgpu", "wgpu-core", "wgpu-hal", "wgpu-types",
    "alacritty_terminal", "vte",
    "crossfont", "freetype-rs", "servo-fontconfig",
    "portable-pty",
    "russh", "russh-keys", "russh-cryptovec",
    "bytemuck",
    "tokio",
    "parking_lot", "parking_lot_core",
    "memchr", "smallvec",
    "raw-window-handle",
    "libc", "nix"
  ]
}
```

**The allowlist is larger than CONTEXT D-12 suggests.** The extras are unavoidable — `bytemuck`, `parking_lot`, `tokio`, `libc`, `memchr`, `smallvec` all have unsafe blocks and are universal Rust ecosystem foundations. The planner should explain this to the user when proposing the gate.

### Anti-Patterns to Avoid

- **`assert_binary_snapshot!` for byte-equality PNG diffs.** Two CI runners will produce two pixel-identical-on-purpose but byte-different PNGs (different `image` crate encoder versions, different metadata chunks). Perceptual diff is the gate; insta storage is incidental.
- **System-font fallback during snapshot tests.** Per D-04. Confirm `FontStack::load_bundled` panics or returns a hard error if the bundled JetBrainsMono is missing — silent fallback to Menlo would invalidate every golden.
- **`cargo bench`-based perf gates.** Statistical noise on shared CI runners → flapping → ignored alerts. Use threshold gates with explicit tolerance.
- **Hardcoded `target/dmg/Vector-{HARDCODED_VERSION}-universal.dmg`** in release.yml — already glob-based via `Vector-*-universal.dmg`. Keep it that way.
- **`gh release create --generate-notes` for v1.0.0.** Explicitly rejected by D-18; first release deserves a real story.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Image perceptual diff | Hand-rolled SSIM on `image` crate pixels | `image-compare 0.5.0::rgb_hybrid_compare` | Edge cases: gamma correction, channel ordering (BGRA vs RGBA from wgpu surface), YUV conversion accuracy. |
| Snapshot review workflow | Custom diff-and-bless tool | `cargo insta review` from `insta 1.47` | Battle-tested; engineers already know the UX. |
| Universal binary assembly | Custom lipo wrapper | Existing `cargo xtask dmg --universal` (already shipping) | Phase 1's groundwork — don't rewrite. |
| DMG creation | hdiutil wrapper script | Existing `xtask dmg` (already shipping) | Already produces `Vector-{version}-universal.dmg`. |
| Release notes generation | Custom git-log-to-markdown | `git-cliff` for v1.0.1+; hand-written for v1.0.0 only | Already in release.yml; planner just bypasses for one release. |
| Unsafe-dep policy enforcement | Custom `cargo metadata` walker | `cargo-geiger 0.13 --forbid-only` | Cargo-geiger has 5 years of fixing edge cases (proc macros, build scripts, conditional compilation). |
| Token-shape detection | Custom AST walker | `regex` on tracing-captured stderr/stdout | Pre-existing arch-lint already uses `regex` for the static analysis pass (`no_token_in_debug_or_log.rs`); the dynamic CI gate is the runtime complement. |
| Perceptual diff threshold tuning | Statistical analysis of many runs | Start at SSIM 0.98 (image-compare default-ish), tune down to 0.95 if cross-runner flapping observed | Same threshold many terminal projects use; tunable per-scene if one fixture proves flaky. |

**Key insight:** Phase 10 is overwhelmingly **wiring** work, not invention. The existing offscreen harness + Term API + arch-lint + release pipeline cover ~80% of the implementation; the new code is glue.

## Runtime State Inventory

> Phase 10 is not a rename/refactor/migration phase; this section does not apply. No stored data, live service config, OS-registered state, secrets, or build artifacts carry over with name-dependence. Phase 10 adds CI jobs, a new test crate, and a config file (`cargo-geiger.allowlist.json`).

## Common Pitfalls

### Pitfall A: Snapshot tests flap across arm64 vs x86_64 runners

**What goes wrong:** Two pixel-different PNGs produced on macos-14 (Apple Silicon) and macos-15-intel because subpixel rendering, font hinting, or fragment-shader rounding differs by µε.

**Why it happens:** CoreText's antialiasing rounds at different precisions on different ISAs; Metal's fragment shader runs in fp16 on Apple Silicon by default vs. fp32 on Intel.

**How to avoid:**
- Run snapshot tests ONLY on macos-14 (arm64) in CI. Skip on macos-15-intel — the renderer is identical at the algorithmic level; cross-arch testing belongs to the build/lipo job.
- Use perceptual diff (SSIM) instead of byte-equality. Threshold 0.98 absorbs subpixel jitter.
- Test on the canonical resolution that matches the bundled font's 14pt @ 1.0 DPR (matches existing `FontStack::load_bundled(1.0, 14.0)`).

**Warning signs:** Snapshot test passes locally on M-series Mac, fails in CI; diff PNGs show "everything looks the same but score is 0.96."

### Pitfall B: cargo-geiger false positives on build-script unsafe

**What goes wrong:** cargo-geiger flags a transitive dep (e.g., `serde_derive`'s build script) as unsafe-using, even though the build script never runs at runtime.

**Why it happens:** `--forbid-only` scans for `#![forbid(unsafe_code)]` at the *crate root*; absence is interpreted as "may use unsafe." Many small utility crates don't bother adding the lint even when they have zero unsafe blocks.

**How to avoid:**
- Use cargo-geiger's *full* mode for the first audit, identify the actual unsafe-using crates, and populate the allowlist accordingly.
- Accept that the allowlist will be ~25–40 entries (larger than D-12's 7). Document each entry with a one-line reason.
- Have a script to refresh the allowlist diff for CI inspection: `cargo geiger --output-format Json | jq '.crates[] | select(.unsafety.used.functions > 0) | .package.id'`.

**Warning signs:** CI fails with "crate X uses unsafe" for a crate you've never heard of. Investigate before adding to allowlist; some are legitimate (e.g., `crossbeam-utils`), some indicate a supply-chain concern.

### Pitfall C: Release-pipeline asset-name mismatch (D-15)

**What goes wrong:** CONTEXT D-15 says `Vector-{version}-universal.dmg`. The existing `xtask dmg --universal` produces `Vector-2026.5.10-universal.dmg` (workspace version). When the user tags `v1.0.0`, the asset still says `2026.5.10` because the workspace package version was never bumped.

**Why it happens:** `workspace.package.version = "2026.5.10"` is the CalVer-style placeholder; `git tag v1.0.0` doesn't auto-bump it.

**How to avoid:**
- **Make "bump workspace.package.version to 1.0.0" an explicit task in the plan, gated on PERSIST-04 sign-off per D-21.** Same commit creates the tag.
- Verify in release.yml: `cargo metadata --no-deps --format-version 1 | jq -r '.workspace_packages[0].version'` must equal `${{ steps.tag.outputs.tag }} | sed 's/^v//'` — fail the release if not.

**Warning signs:** Tag is `v1.0.0`, DMG is `Vector-2026.5.10-universal.dmg`; teammates download "the wrong file."

### Pitfall D: Snapshot fixture coverage cliff on Reconnect scene

**What goes wrong:** The D-02 scene (c) "reconnect status bar + tab badge" exercises `ReconnectPass` whose animation is time-dependent (fade-in over `RECONNECT_FADE_IN_MS`). A snapshot at t=0ms and t=200ms produce different alpha values; the test flaps depending on render scheduling.

**Why it happens:** `vector-render::reconnect_pass::alpha_at(now, started_at)` returns a time-varying alpha. A naïve snapshot test that captures "right now" gets nondeterministic output.

**How to avoid:**
- Snapshot at a *fixed virtual time* — extend `Compositor::render_offscreen_with` (or its wrapper) to accept an `Option<Instant>` for "time origin"; pass `started_at + RECONNECT_FADE_IN_MS` so alpha is always 1.0 in the snapshot.
- Alternative: snapshot the *static* end state only (alpha=1.0 plateau), not the fade.

**Warning signs:** The `reconnect_bar_tab_badge.rs` snapshot test passes 9 of 10 runs locally; CI flakes.

### Pitfall E: Bundled font path differs between `cargo test` (workspace) and `cargo test -p vector-render-snapshots`

**What goes wrong:** `FontStack::load_bundled` searches relative to `CARGO_MANIFEST_DIR`. Running from the new crate, the relative path to `crates/vector-app/resources/Fonts/` resolves differently.

**Why it happens:** Inspecting `vector-fonts/src/loader.rs` shows the path-resolution logic checks multiple candidate locations — but the test must run from a known root.

**How to avoid:**
- Verify `FontStack::load_bundled` works from `crates/vector-render-snapshots/`. If not, add a `VECTOR_FONT_PATH` env-var override in `vector-fonts` (small, low-risk delta) and pass it from the test harness via `std::env::var` fallback.
- Add a smoke assertion at the top of the common harness: `font_stack.is_some(), "JetBrainsMono missing — check crates/vector-app/resources/Fonts/"`.

**Warning signs:** First snapshot test panics with "font not loaded."

### Pitfall F: GitHub Actions secrets unavailable for token-grep job

**What goes wrong:** The token-grep CI job tries to exercise live OAuth and silently skips because `VECTOR_OAUTH_TEST_TOKEN` isn't set. The gate is green but tested nothing.

**Why it happens:** GitHub secrets are gated for security; PR forks don't get them.

**How to avoid:**
- The token-grep job MUST use wiremock-backed unit tests (no live credentials needed), not live auth flows. Existing `vector-codespaces/tests/` and `vector-tunnels/tests/` patterns work.
- Make the test fixtures use realistic-looking but fake tokens (`gho_FAKE_TOKEN_FOR_TESTING_xxxxxxxxxxxx`) so the grep regex is exercised; if the grep ever passes a run where the fixture's fake token leaked, that's a real bug.

**Warning signs:** Token-grep job passes on every PR ever; never finds a real leak.

## Code Examples

Verified patterns referenced for task authors:

### Headless Compositor Bootstrap (HARDEN-01)

```rust
// Source: crates/vector-render/tests/common/offscreen.rs (existing, lines 11-26)
use vector_fonts::FontStack;
use vector_render::{Compositor, Offscreen};

pub fn build_compositor(width: u32, height: u32) -> Option<(Compositor, Offscreen)> {
    let ctx = vector_render::RenderContext::new_offscreen(width, height).ok()?;
    let font_stack = FontStack::load_bundled(1.0, 14.0).ok()?;
    let comp = Compositor::new_with(
        &ctx.device, &ctx.queue, ctx.format, ctx.width, ctx.height, font_stack,
    ).ok()?;
    Some((comp, ctx))
}
```

### Feeding VT Sequences and Asserting Mode (HARDEN-02)

```rust
// Source: crates/vector-term/tests/dcs_dispatch.rs (existing, lines 19-36)
use alacritty_terminal::term::TermMode;
use vector_term::Term;

#[test]
fn bracketed_paste_mode_2004_sets_state() {
    let mut term = Term::new(80, 24, 1000);
    assert!(!term.mode().contains(TermMode::BRACKETED_PASTE));
    term.feed(b"\x1b[?2004h");
    assert!(term.mode().contains(TermMode::BRACKETED_PASTE));
    term.feed(b"\x1b[?2004l");
    assert!(!term.mode().contains(TermMode::BRACKETED_PASTE));
}
```

### OSC 52 DCS-Wrapped Round-Trip (HARDEN-02)

```rust
// Source: crates/vector-term/tests/osc52.rs (existing, lines 33-49)
use tokio::sync::mpsc;
use vector_term::{listener::ClipboardEvent, Term};

#[tokio::test(flavor = "current_thread")]
async fn dcs_wrapped_round_trip() {
    let (write_tx, _wrx) = mpsc::channel(16);
    let (clip_tx, mut clip_rx) = mpsc::channel(16);
    let mut t = Term::with_channels(80, 24, 1000, write_tx, clip_tx);
    t.feed(b"\x1bP\x1b]52;c;aGVsbG8=\x07\x1b\\");
    let ev = tokio::time::timeout(std::time::Duration::from_millis(100), clip_rx.recv())
        .await.expect("OSC 52 DCS-wrapped within 100ms").expect("channel closed");
    match ev {
        ClipboardEvent::Store(_, data) => assert_eq!(data, "hello"),
        _ => panic!("expected Store"),
    }
}
```

### Manual Debug for Token-Bearing Struct (HARDEN-03 reference)

```rust
// Source: crates/vector-codespaces/src/auth/device_flow.rs (existing, lines 41-52)
pub struct Tokens {
    pub access: Zeroizing<String>,
    pub refresh: Option<Zeroizing<String>>,
}

impl std::fmt::Debug for Tokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tokens")
            .field("access", &"<redacted>")
            .field("refresh", &self.refresh.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}
```

### Existing Release Workflow Asset Upload (HARDEN-04 — current state)

```yaml
# Source: .github/workflows/release.yml (existing, lines 134-147)
- name: Publish or update GitHub Release
  env:
    GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  run: |
    TAG="${{ steps.tag.outputs.tag }}"
    if gh release view "$TAG" >/dev/null 2>&1; then
      gh release upload "$TAG" target/dmg/Vector-*-universal.dmg --clobber
      gh release edit "$TAG" --title "Vector $TAG" --notes-file RELEASE_NOTES.md
    else
      gh release create "$TAG" \
        --title "Vector $TAG" \
        --notes-file RELEASE_NOTES.md \
        target/dmg/Vector-*-universal.dmg
    fi
```

### Phase 10 Delta — SHA256 Sidecar Upload (HARDEN-04 D-16)

```yaml
# NEW step to add between "Append xattr install footer" and "Publish or update GitHub Release":
- name: Generate SHA256 checksum
  run: |
    set -euo pipefail
    cd target/dmg
    for f in Vector-*-universal.dmg; do
      shasum -a 256 "$f" > "${f}.sha256"
      cat "${f}.sha256"
    done

# And amend the upload step to include the sidecar:
gh release upload "$TAG" target/dmg/Vector-*-universal.dmg target/dmg/Vector-*-universal.dmg.sha256 --clobber
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Hand-rolled VT corpus with `assert_eq!` on grid bytes | Drive `alacritty_terminal::Term` directly + assert via `Term::mode()` / `Term::grid()` / `Term::cursor()` | Phase 2 (alacritty_terminal 0.26) | The corpus is unit-test fast; no PTY, no GPU, no winit needed. |
| `cargo bench` for perf gates | Custom threshold probe binary | n/a (project decision) | Avoids criterion's statistical flapping on shared CI runners. |
| `cargo-deny [bans] unsafe` (mythical) | `cargo-geiger 0.13 --forbid-only` + allowlist | 2024+ (cargo-geiger 0.12 added `--forbid-only`) | The actual existing tool. cargo-deny has never had this knob. |
| `image-compare 0.4 -> 0.5` | `rgb_hybrid_compare` returns `Similarity { score, image }` with score ∈ [0,1] | 2025-08-18 | Score-based threshold (0.98 default for "imperceptible diff") is simpler than delta-E thresholds. |
| `insta::assert_binary_snapshot!` for raw PNG bytes | Use insta for metadata sidecar only; perceptual diff is the gate | n/a | `insta 1.47` does add the macro, but byte-equality is the wrong tool for cross-runner PNG. |

**Deprecated/outdated guidance to ignore:**
- CONTEXT.md "Reusable Assets" says insta is workspace-wide dev-dep. **It is not.** Verified by grep on every `Cargo.toml` and on `Cargo.lock`. Plan must add it.
- CONTEXT.md D-12 names `cargo deny`'s `[bans] unsafe`. **No such knob exists.** Use cargo-geiger.

## Open Questions

1. **Conflict Surface item 1: D-12 cargo-deny mechanism doesn't exist. How to resolve?**
   - **What we know:** cargo-deny 0.19.7 (2026-05-22, current) has no unsafe-banning knob. The intent — block unaudited unsafe in release-profile deps — is real and achievable via `cargo-geiger 0.13.0 --forbid-only` + allowlist JSON.
   - **What's unclear:** does the user want to (a) swap to cargo-geiger and accept the ~25–40-entry allowlist, (b) demote the gate to advisory (run `cargo geiger` for visibility, don't fail CI), or (c) drop the unsafe gate entirely and rely on the workspace `unsafe_code = "deny"` lint that already covers first-party code?
   - **Recommendation:** option (a). cargo-geiger + allowlist preserves D-12's intent; the larger allowlist is a documentation cost, not a security cost. Open with the user before the plan is finalized.

2. **Perf gate thresholds on macos-15-intel: is 1% idle CPU realistic on a shared runner?**
   - **What we know:** macos-14 arm64 hits <1% idle reliably (Pitfall 3 verified in Phase 3). macos-15-intel is x86_64 on a shared VM with unpredictable noisy neighbors.
   - **What's unclear:** does the perf gate run on both runners, or only arm64?
   - **Recommendation:** run only on macos-14 arm64 as a hard gate; macos-15-intel runs the perf probe as advisory output (no fail). The build-arm64 + build-x86_64 split for the Universal binary still verifies x86_64 *correctness*; perf parity on x86_64 is not a v1 acceptance criterion (PROJECT.md acceptance criterion is "60+ fps on Apple Silicon" — explicitly Apple Silicon only).

3. **Should the snapshot tests run on every PR or only on master?**
   - **What we know:** Snapshot reviews are noisy; renderer changes legitimately move pixels.
   - **What's unclear:** is `cargo insta review` ergonomic enough for the user to absorb the noise on every PR?
   - **Recommendation:** run on every PR by default; document the `INSTA_UPDATE=auto` workflow in the plan. If noise becomes painful in practice, downgrade to "master only" — but the cost of catching a regression in main is higher than the cost of a few extra review clicks.

4. **Does PERSIST-04 (Phase 9) actually block v1.0.0, given the joint UAT debt?**
   - **What we know:** D-21 makes the tag gated on PERSIST-04 sign-off. PERSIST-04 status in REQUIREMENTS.md is Pending due to joint debt with 09-05/09-06 UATs blocked by DevTunnelsActor main.rs wiring.
   - **What's unclear:** is the user planning a follow-up plan to wire DevTunnelsActor before Phase 10, or in parallel?
   - **Recommendation:** raise this as the FIRST item the planner addresses with the user. If DevTunnelsActor wiring needs to happen, it's a Phase 9 task, not Phase 10 — but Phase 10 plans should be authored with full awareness that the tag step is gated.

5. **Hand-written v1.0.0 release notes location: in-repo vs in-the-release-only?**
   - **What we know:** D-18 says hand-written for v1.0.0; existing pipeline uses `git-cliff --latest > RELEASE_NOTES.md`.
   - **What's unclear:** does the user want the v1.0.0 notes also committed to the repo (e.g., `CHANGELOG.md` or `.planning/release-notes/v1.0.0.md`) or only in the GitHub Release body?
   - **Recommendation:** commit to `CHANGELOG.md` (one section per release) AND use it as the source for the release body. Keeps history searchable without leaving the repo.

## Environment Availability

Phase 10 depends on tools that are either already in CI (no install delta) or installed inside CI jobs at use-time. Local dev only needs the workspace toolchain.

| Dependency | Required By | Available in CI | Version | Fallback |
|------------|------------|-----------------|---------|----------|
| `cargo-bundle` | release.yml (existing) | ✓ (installed in `release` job) | 0.10.0 | — |
| `create-dmg`, `librsvg` | release.yml (existing) | ✓ (`brew install`) | latest | — |
| `git-cliff` | release.yml (existing) | ✓ (`brew install`) | latest | hand-written notes per D-18 |
| `lipo` | release.yml (existing) | ✓ (Xcode CLI tools, on all macos runners) | bundled | — |
| `hdiutil` | release.yml via `xtask dmg` | ✓ (macOS system tool) | bundled | — |
| `shasum -a 256` | NEW for D-16 | ✓ (macOS system tool, BSD shasum) | bundled | `openssl dgst -sha256` |
| `cargo-geiger` | NEW for HARDEN-03 unsafe gate | ✗ (must `cargo install` in CI job) | 0.13.0 | demote to advisory if install flaky |
| `cargo-insta` | NEW for HARDEN-01 review ergonomics | ✗ (only needed locally for `cargo insta review`; CI runs `cargo test`) | 1.47.2 | — (not needed in CI) |
| `cargo` (stable 1.88.0) | All jobs | ✓ (existing `dtolnay/rust-toolchain@1.88.0`) | 1.88.0 | — |
| Apple Silicon GPU (Metal) for HARDEN-01 | snapshot CI job | ✓ on macos-14 (M-series); ✗ on macos-15-intel (Intel GPU still has Metal but different driver) | — | skip x86_64 for snapshot tests (Pitfall A) |
| GitHub repository secrets `VECTOR_E2E_*` | Existing persist-e2e job | conditional | — | already `continue-on-error` |

**Missing dependencies with no fallback:** none.

**Missing dependencies with fallback:** `cargo-geiger` install — if `cargo install` is flaky in CI (rare but possible), demote the gate to advisory mode (`continue-on-error: true`) until the install settles.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (workspace) + `insta 1.47.2` snapshot review + `cargo-geiger 0.13.0` unsafe audit |
| Config file | none global; per-crate `Cargo.toml` `[dev-dependencies]`; `cargo-geiger.allowlist.json` at workspace root |
| Quick run command | `cargo test --workspace --tests` (~30s warm) |
| Full suite command | `cargo test --workspace --all-targets` + `cargo geiger --forbid-only` + the four new CI jobs |
| Snapshot review | `cargo insta review` (local), `INSTA_UPDATE=auto cargo test` (regenerate) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| **HARDEN-01** | Plain text + Unicode + emoji renders pixel-stable | snapshot (perceptual) | `cargo test -p vector-render-snapshots --test plain_text_unicode_emoji` | ❌ Wave 0 |
| HARDEN-01 | Alt-screen with colors + cursor + selection | snapshot | `cargo test -p vector-render-snapshots --test alt_screen_colors_selection` | ❌ Wave 0 |
| HARDEN-01 | Reconnect status bar + tab badge | snapshot | `cargo test -p vector-render-snapshots --test reconnect_bar_tab_badge` | ❌ Wave 0 |
| HARDEN-01 | Split panes + scrollback | snapshot | `cargo test -p vector-render-snapshots --test split_panes_scrollback` | ❌ Wave 0 |
| HARDEN-01 | CI gate blocks regression | CI gate | `.github/workflows/ci.yml::snapshot` job → exit 1 on `panic!("snapshot diff: …")` | ❌ Wave 0 |
| **HARDEN-02** | DECSET 1049 alt-screen | unit | `cargo test -p vector-term --test vt_conformance alt_screen_1049` | partial (exists in `tests/alt_screen_1049.rs`; relocate per D-07) |
| HARDEN-02 | DECSTBM scroll regions | unit | `cargo test -p vector-term --test vt_conformance scroll_regions` | partial (exists in `tests/decstbm_scroll_region.rs`) |
| HARDEN-02 | Tab stops (HTS/CHT/TBC) | unit | `cargo test -p vector-term --test vt_conformance tab_stops` | partial (in `decstbm_scroll_region.rs`; extract) |
| HARDEN-02 | ED / EL erase semantics | unit | `cargo test -p vector-term --test vt_conformance ed_el_erase` | partial (exists in `tests/ed_el_erase.rs`) |
| HARDEN-02 | Mouse mode 1006 (SGR) | unit | `cargo test -p vector-term --test vt_conformance mouse_1006` | partial (in `dcs_dispatch.rs::mouse_mode_1006_sgr_sets_state`) |
| HARDEN-02 | OSC 52 round-trip (raw + DCS-wrapped) | unit | `cargo test -p vector-term --test vt_conformance osc52_round_trip` | partial (exists in `tests/osc52.rs`) |
| HARDEN-02 | Bracketed paste (mode 2004) | unit | `cargo test -p vector-term --test vt_conformance bracketed_paste` | partial (in `dcs_dispatch.rs::bracketed_paste_mode_2004_sets_state`) |
| HARDEN-02 | DECSCUSR cursor shape | unit | `cargo test -p vector-term --test vt_conformance decscusr` | partial (in `dcs_dispatch.rs::decscusr_cursor_shape_sets_state`) |
| HARDEN-02 | Idle CPU <1% perf gate | integration probe (advisory on x86_64) | `cargo run -p vector-render --example perf_probe -- --mode idle --duration 5s` | ❌ Wave 0 |
| HARDEN-02 | `cat large.log` at vsync cap | integration probe | `cargo run -p vector-render --example perf_probe -- --mode paste --fixture large.log --duration 5s` | ❌ Wave 0 |
| **HARDEN-03** | No `#[derive(Debug)]` on token-bearing structs (static) | unit (arch-lint) | `cargo test -p vector-arch-tests --test no_token_in_debug_or_log` | ✅ (exists; passes) |
| HARDEN-03 | No `tracing::*!` macro logs token field by name (static) | unit (arch-lint) | (same file, second `#[test]`) | ✅ (exists; passes) |
| HARDEN-03 | Runtime grep of RUST_LOG=debug auth trace finds no token-shaped string | CI gate | `.github/workflows/ci.yml::token-grep` job runs auth tests with `RUST_LOG=debug`, greps stderr | ❌ Wave 0 |
| HARDEN-03 | Unsafe blocks in dep tree only in allowlisted crates | CI gate | `.github/workflows/ci.yml::unsafe-audit` job runs `cargo geiger --forbid-only` + allowlist check | ❌ Wave 0 |
| **HARDEN-04** | `Vector-{version}-universal.dmg` exists in release | CI / manual | `gh release view v1.0.0 --json assets --jq '.assets[].name' \| grep -E '^Vector-.*-universal\.dmg$'` | ❌ tag-time only |
| HARDEN-04 | `Vector-{version}-universal.dmg.sha256` exists | CI / manual | `gh release view v1.0.0 --json assets --jq '.assets[].name' \| grep -E '\.sha256$'` | ❌ tag-time only |
| HARDEN-04 | DMG is a fat binary | CI gate (existing) | `lipo -info <bundled binary>` shows both architectures (existing in `release.yml`) | ✅ existing |
| HARDEN-04 | README section 1 is `## Install` with the xattr block | manual (README inspection) | `head -30 README.md \| grep -E '^## Install'` and `grep 'xattr -dr com.apple.quarantine' README.md` | ❌ Wave 0 (README restructure) |
| HARDEN-04 | Release notes are hand-written for v1.0.0 (not git-cliff output) | manual UAT | review `gh release view v1.0.0 --json body` text | n/a (tag-time) |

### Sampling Rate

- **Per task commit:** `cargo test --workspace --tests` (existing project default) — fast path catches arch-lint regressions and unit-test breakage.
- **Per wave merge:** `cargo test --workspace --all-targets` + `cargo test -p vector-render-snapshots` (the snapshot suite has Metal-adapter dependency; runs cleanly on dev Macs).
- **Phase gate before `/gsd:verify-work`:** full suite green; manual run of `cargo geiger --forbid-only` against allowlist; manual `gh release create v1.0.0-rc1 --draft` dry-run to validate the release pipeline before the real tag.

### Wave 0 Gaps

- [ ] `crates/vector-render-snapshots/Cargo.toml` — new test-only crate with insta, image-compare, image dev-deps + vector-render, vector-term, vector-fonts deps. Workspace `members` add.
- [ ] `crates/vector-render-snapshots/src/lib.rs` — empty (or thin re-export shim — must compile to enable `cargo test`).
- [ ] `crates/vector-render-snapshots/tests/common/mod.rs` — shared render-scene + diff helpers.
- [ ] `crates/vector-render-snapshots/tests/no_tokio_main.rs` — required by existing ci.yml arch-lint (`crates_count == tests_count`).
- [ ] `crates/vector-render-snapshots/tests/goldens/*.png` — 4 PNGs, ~4–16 KB each, generated via first run of `INSTA_UPDATE=auto cargo test`.
- [ ] `crates/vector-render-snapshots/tests/{plain_text_unicode_emoji,alt_screen_colors_selection,reconnect_bar_tab_badge,split_panes_scrollback}.rs` — 4 test files.
- [ ] `crates/vector-term/tests/vt_conformance/` — 8 test files (extracting + extending existing tests per D-07).
- [ ] `crates/vector-render/examples/perf_probe.rs` — threshold-based perf measurement binary + `crates/vector-render/examples/fixtures/large.log` (1 MB synthetic log).
- [ ] `cargo-geiger.allowlist.json` — root-level allowlist with ~25–40 entries (full list from first `cargo geiger` run).
- [ ] `.github/scripts/check-geiger.py` (or `.sh`) — parses geiger JSON, fails if any unsafe-using crate not in allowlist.
- [ ] `.github/workflows/ci.yml` — three new jobs: `snapshot`, `vt-perf`, `token-grep`. Maybe a fourth `unsafe-audit` (or fold into existing `deny`).
- [ ] `.github/workflows/release.yml` — delta steps: `shasum -a 256`, sidecar upload, hand-written notes path (skip git-cliff for v1.0.0).
- [ ] `README.md` — restructure: `## Install` becomes section 1; `### Why the xattr step?` subsection.
- [ ] `CHANGELOG.md` — new file; first entry is hand-written v1.0.0 notes.
- [ ] Workspace version bump from `2026.5.10` → `1.0.0` (one task, gated on PERSIST-04).

## Sources

### Primary (HIGH confidence)

- `/Users/ashutosh/personal/vector/.planning/phases/10-hardening-release/10-CONTEXT.md` — user decisions D-01..D-21
- `/Users/ashutosh/personal/vector/.planning/REQUIREMENTS.md` — HARDEN-01..04 acceptance criteria
- `/Users/ashutosh/personal/vector/.planning/ROADMAP.md` — Phase 10 success criteria verbatim
- `/Users/ashutosh/personal/vector/CLAUDE.md` — pinned tech stack, version compatibility matrix
- `/Users/ashutosh/personal/vector/.planning/research/PITFALLS.md` — full "looks done but isn't" checklist + Pitfalls 1–22
- `/Users/ashutosh/personal/vector/.github/workflows/release.yml` — current release pipeline (read in full)
- `/Users/ashutosh/personal/vector/.github/workflows/ci.yml` — current CI (read in full)
- `/Users/ashutosh/personal/vector/deny.toml` — current cargo-deny config
- `/Users/ashutosh/personal/vector/Cargo.toml` — workspace members, dependencies, lints (`unsafe_code = "deny"`)
- `/Users/ashutosh/personal/vector/crates/vector-render/tests/common/offscreen.rs` — existing offscreen harness (reusable verbatim)
- `/Users/ashutosh/personal/vector/crates/vector-render/tests/snapshot_*.rs` — existing snapshot test patterns
- `/Users/ashutosh/personal/vector/crates/vector-term/tests/{alt_screen_1049,decstbm_scroll_region,ed_el_erase,dcs_dispatch,osc52}.rs` — existing VT test precedents (1:1 with D-07 corpus)
- `/Users/ashutosh/personal/vector/crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs` — existing Pitfall-14 enforcement
- `/Users/ashutosh/personal/vector/crates/vector-codespaces/src/auth/device_flow.rs` — existing manual Debug pattern (`Tokens` struct)
- `/Users/ashutosh/personal/vector/crates/vector-tunnels/src/{auth/*,api.rs,model.rs}` — existing manual Debug coverage (TunnelRecord, MicrosoftAuth, etc.)
- `/Users/ashutosh/personal/vector/crates/vector-secrets/src/lib.rs` — Secrets API + manual Debug
- [crates.io API for cargo-deny 0.19.7](https://crates.io/api/v1/crates/cargo-deny) — verified 2026-05-22
- [crates.io API for cargo-geiger 0.13.0](https://crates.io/api/v1/crates/cargo-geiger) — verified 2025-08-31
- [crates.io API for image-compare 0.5.0](https://crates.io/api/v1/crates/image-compare) — verified 2025-08-18
- [crates.io API for insta 1.47.2](https://crates.io/api/v1/crates/insta) — verified 2026-03-30
- [insta snapshot-types docs](https://insta.rs/docs/snapshot-types/) — confirms `assert_binary_snapshot!` (experimental)
- [docs.rs/insta](https://docs.rs/insta) — macro list verified
- [docs.rs/image-compare](https://docs.rs/image-compare/latest/image_compare/) — `rgb_hybrid_compare` API

### Secondary (MEDIUM confidence)

- [cargo-deny bans cfg docs](https://embarkstudios.github.io/cargo-deny/checks/bans/cfg.html) — verified no `unsafe` knob exists
- [cargo-geiger README](https://github.com/geiger-rs/cargo-geiger) — verified `--forbid-only` flag and allowlist concept
- [GeekWala "Rust Vulnerability Scanning"](https://www.geekwala.com/blog/securing-rust-dependencies-2026) — cross-reference for "cargo-deny + cargo-geiger as complementary tools" guidance

### Tertiary (LOW confidence — flagged for validation)

- macOS shared CI runner perf variance — cited from PROJECT context, not measured fresh for Phase 10; should be observed empirically when perf probe lands.
- The "~25–40 entries" allowlist size estimate — order-of-magnitude only; first `cargo geiger` run on the full workspace dep tree will give the true number.

## Metadata

**Confidence breakdown:**
- Standard stack: **HIGH** — every crate version verified against crates.io 2026-05-26; primary stack additions (insta, image-compare) are well-trod ground in Rust testing.
- Architecture: **HIGH** — patterns are extracted directly from existing in-tree code; ~80% of the implementation is gluing existing pieces.
- Pitfalls: **HIGH** for renderer/VT/release pipeline (existing project-specific PITFALLS.md + verified test precedents); **MEDIUM** for the cargo-geiger allowlist accuracy (true content only known after first run).
- Open questions: **MEDIUM** — the cargo-deny mechanism gap (Open Q 1) and PERSIST-04 gating (Open Q 4) are blockers the planner should raise with the user before tasking.

**Research date:** 2026-05-26
**Valid until:** 2026-06-25 (30 days; tooling versions stable, no protocol or platform dependency for Phase 10).
