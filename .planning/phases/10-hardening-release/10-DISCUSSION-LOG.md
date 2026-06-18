# Phase 10: Hardening & Release — Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in 10-CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-26
**Phase:** 10-hardening-release
**Areas discussed:** Snapshot test scope, VT conformance corpus shape, Token redaction sweep + grep gate, Release UX

---

## Snapshot test scope

| Option | Description | Selected |
|--------|-------------|----------|
| Scene-based fixtures | Curated test pages rendered to PNG; 4–8 goldens covering plain text + alt-screen + reconnect bar + splits. Stable, intentional, debuggable. | ✓ |
| Glyph atlas only | Snapshot just CoreText → atlas rasterization. Most stable but misses layout/compositing/state regressions. | |
| Full frame on every change | Snapshot entire visible window for long input scripts. Catches everything but goldens churn constantly. | |

**User's choice:** Scene-based fixtures
**Notes:** Initial scenes locked in CONTEXT D-02: plain text + Unicode + emoji, alt-screen with colors/cursor/selection, reconnect status bar + tab badge, splits + scrollback.

| Option | Description | Selected |
|--------|-------------|----------|
| Perceptual (delta-E or SSIM, ~2 threshold) | Absorbs sub-pixel/antialias drift across arm64 vs x86_64 runners without flapping. Standard for headless rendering CI. | ✓ |
| Pixel-strict (byte-equal) | Fails on any pixel diff; catches the most but cross-runner antialias drift will be flaky. | |
| Loose MAE per tile (~5%) | Mean absolute error per 16×16 tile. Forgiving; only useful if perceptual flakes. | |

**User's choice:** Perceptual (delta-E or SSIM, ~2 threshold)
**Notes:** Threshold value (~2.0) locked. Exact comparator library (`image-compare` vs `imageproc` vs hand-rolled) left to planner per CONTEXT Claude's Discretion.

---

## VT conformance corpus shape

| Option | Description | Selected |
|--------|-------------|----------|
| Hand-craft 8-scenario corpus | Author tests for the 8 ROADMAP scenarios (alt-screen, scroll regions, tab stops, ED/EL, mouse 1006, OSC 52, bracketed paste, DECSCUSR). Tight, owned, maps 1:1 to PITFALLS.md. | ✓ |
| Vendor vttest's source corpus | Bigger coverage but most tests obsolete (VT100-specific). | |
| Both — hand-craft core + vendor extras | Hand-craft as gate; vendor as `#[ignore]`d aspirational. Higher cost, defers decisions. | |

**User's choice:** Hand-craft 8-scenario corpus
**Notes:** vttest vendoring captured in CONTEXT deferred-ideas for v2 reconsideration.

| Option | Description | Selected |
|--------|-------------|----------|
| alacritty_terminal directly in unit tests | Feed sequences into `Term`, assert grid state via Term API. Fast (<1s), zero infra. | ✓ |
| Spawn vector binary + pipe stdin | True e2e through windowing+GPU; catches integration bugs but heavy infra and flaky CI risk. | |
| Hybrid — Term unit + small render e2e | Mostly Term + a handful of e2e tests for DECSCUSR + mouse cursor. Adds implementation overhead. | |

**User's choice:** alacritty_terminal directly in unit tests
**Notes:** True binary-driven e2e captured in CONTEXT deferred-ideas as a future v2 phase.

---

## Token redaction sweep + grep gate

| Option | Description | Selected |
|--------|-------------|----------|
| Heavy: audit + workspace lint + CI grep gate | Sweep all token-bearing structs, add clippy lint preventing `derive(Debug)` regressions, add CI grep on `gho_|ghp_|eyJ` in recorded tracing output. Matches HARDEN-03 wording verbatim. | ✓ |
| Medium: audit + workspace lint | Sweep + lint, skip the grep gate. README claim about clean grep output would require manual verification. | |
| Light: verify existing impls survive | Trust current code; just confirm no regression. | |

**User's choice:** Heavy (audit + lint + grep gate)
**Notes:** This is the literal text of HARDEN-03 success criterion #3 — going lighter would have meant publishing a promise we can't enforce.

| Option | Description | Selected |
|--------|-------------|----------|
| `[bans] unsafe = "deny"` with explicit allowlist | Enumerate the small set of unsafe-bearing deps (objc2, wgpu, alacritty_terminal, crossfont, portable-pty). New deps must add to allowlist with a reason. | ✓ |
| `[bans] unsafe = "warn"` | Logs but doesn't block CI. Lower friction; not what HARDEN-03 calls for. | |
| Skip the unsafe knob entirely | Treat HARDEN-03's unsafe clause as documented best practice, not enforced. | |

**User's choice:** Deny + explicit allowlist
**Notes:** Allowlist enumerated in CONTEXT D-12.

---

## Release UX

| Option | Description | Selected |
|--------|-------------|----------|
| `Vector-{version}-universal.dmg` | Unique per tag (GH requires unique asset names), version + arch advertised in filename. | ✓ |
| `Vector.dmg` | Clean but clashes on re-uploads; forces rename ceremony. | |
| `Vector-{version}.dmg` | Versioned but doesn't advertise universal nature. | |

**User's choice:** `Vector-{version}-universal.dmg`
**Notes:** Sibling SHA256 file (`.sha256` suffix) also part of D-16.

| Option | Description | Selected |
|--------|-------------|----------|
| README top-of-file block with copy-paste + WHY | First section of README is `## Install`; copy block + a one-paragraph "why xattr is needed". | ✓ |
| README block + `scripts/trust-vector.sh` helper | Adds maintenance for a 3-line script; unnecessary for a 5-person audience. | |
| Wiki page link from README | Adds a click; install copy lives elsewhere. Not "front-and-center". | |

**User's choice:** README top-of-file block with copy-paste + WHY
**Notes:** Exact xattr+open commands fixed in CONTEXT specifics section.

| Option | Description | Selected |
|--------|-------------|----------|
| Hand-written for v1.0.0, auto-from-commits later | v1 deserves a real story; later point releases use `gh release create --generate-notes`. | ✓ |
| Auto-from-commits via gh CLI | Fast but commit titles read as noise to a teammate. | |
| Pull from PHASE-VERIFICATION.md success criteria | Most accurate but verbose for a first read. | |

**User's choice:** Hand-written for v1.0.0
**Notes:** Future point-release behavior also encoded in D-18.

---

## Claude's Discretion

- Perf gate measurement approach (idle CPU < 1%, `cat large.log` at vsync cap) — planner chooses tool/probe.
- Exact perceptual-tolerance library for HARDEN-01 D-03 (threshold ~2.0 locked, library not).
- Whether the token-redaction grep gate runs against a recorded tracing file in git vs a freshly-recorded one each CI run.
- File organization for VT conformance corpus (one file vs one file per scenario).
- Whether `lipo` runs in its own GH Actions job or inside the existing `release` job.

## Deferred Ideas

- Vendoring vttest's full corpus as `#[ignore]`d aspirational tests.
- True end-to-end VT tests that spawn the `vector` binary over PTY.
- `scripts/trust-vector.sh` helper for xattr.
- Per-PR snapshot baseline preview comments.
- Auto-generated release notes for v1.0.0 (explicitly rejected).
- Code signing + notarization + Sparkle (DIST-V2-01/02).
- Public OSS push, contributor docs.
- Apple Silicon vs Intel DMG-size profiling.
