---
phase: 03
slug: gpu-renderer-first-paint
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-05-11
updated: 2026-05-11
---

# Phase 03 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Bootstrapped from `03-RESEARCH.md §Validation Architecture`. Test paths reconciled with 03-01..03-05 PLAN.md files (rev: 2026-05-11).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust workspace; per-crate integration tests under `crates/<name>/tests/`) |
| **Config file** | `Cargo.toml` (workspace) + per-crate `Cargo.toml` |
| **Quick run command** | `cargo test --workspace --tests -q` |
| **Full suite command** | `cargo test --workspace --tests --release` |
| **Estimated runtime** | ~3–8s (debug) per current Phase 2 baseline (53 tests in <1s); Phase 3 adds renderer/atlas/input tests — budget under 30s total. |

**Additional automated gates Phase 3 introduces:**
- `cargo clippy --workspace --all-targets -- -D warnings` (workspace-wide; `await_holding_lock = "deny"` from Phase 1 D-11 is the renderer-specific guard)
- `cargo fmt --all -- --check`
- Per-crate `tests/no_tokio_main.rs` arch-lint (Phase 1 D-08; must stay 15==15)
- GPU snapshot tests via `wgpu::TextureView` readback under `crates/vector-render/tests/` (offscreen render — no display required, matches Phase 1 CI on `macos-14` runners)

---

## Sampling Rate

- **After every task commit:** Run `cargo test --workspace --tests -q` (quick)
- **After every plan wave:** Run quick + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo fmt --all -- --check`
- **Before `/gsd:verify-work`:** Full suite (`--release`) must be green plus the manual smoke matrix below
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

Per-task verification is canonical in each plan's `must_haves.truths` and `<verify><automated>` blocks — see the 5 PLAN.md files for the authoritative map. Summary table below for cross-reference.

| Plan | Wave | Owns | Test Files | Automated Command |
|------|------|------|------------|-------------------|
| 03-01 | 1 | Crate scaffolding + winit + wgpu surface + no_tokio_main arch-lint | `crates/vector-{render,fonts,input,app}/tests/no_tokio_main.rs`, `crates/vector-render/tests/pipeline_init.rs` | `cargo test --workspace --tests -q` |
| 03-02 | 2 | Glyph atlas + crossfont + JBM bundle | `crates/vector-render/tests/atlas_lru.rs`, `crates/vector-fonts/tests/crossfont_load_bundled.rs`, `crates/vector-fonts/tests/grayscale_pixel_format.rs` | `cargo test -p vector-render -p vector-fonts --tests` |
| 03-03 | 3 | Cell + cursor pipelines + truecolor + Compositor::render (selection arg from day one, callers pass None) | `crates/vector-render/tests/{damage_to_quads,snapshot_clearcolor,snapshot_singlecell,snapshot_truecolor,cursor_overlay_snapshot}.rs` | `cargo test -p vector-render --tests` |
| 03-04 | 4 | xterm keymap + bracketed paste + selection state + Compositor selection wiring | `crates/vector-input/tests/{xterm_key_table,bracketed_paste_wrap}.rs`, `crates/vector-render/tests/selection_overlay_snapshot.rs`, `crates/vector-app/tests/selection_render.rs` | `cargo test --workspace --tests -q` |
| 03-05 | 5 | Frame pacing + LPM + DPR atlas clear + first-paint gate + manual smoke | `crates/vector-app/tests/frame_pacing.rs`, `crates/vector-render/tests/{pty_coalesce,idle_no_redraw,dpr_change_invalidates}.rs` | `cargo test --workspace --tests -q` |

*Status: every task in every plan ships either an `<automated>` command or a `checkpoint:human-verify` block — Nyquist contract satisfied.*

---

## Wave 0 Requirements

Test scaffolding that must land before later waves run (matches Phase 2's Plan 02-01 model). Paths reconciled with plan `files_modified` lists (rev: 2026-05-11).

- [ ] `crates/vector-render/tests/snapshot_clearcolor.rs` — `#[ignore]` stub (filled by Plan 03-03)
- [ ] `crates/vector-render/tests/snapshot_singlecell.rs` — `#[ignore]` stub (filled by Plan 03-03)
- [ ] `crates/vector-render/tests/snapshot_truecolor.rs` — `#[ignore]` stub (filled by Plan 03-03; RENDER-04)
- [ ] `crates/vector-render/tests/atlas_lru.rs` — `#[ignore]` stub (filled by Plan 03-02; Pitfall 2)
- [ ] `crates/vector-render/tests/dpr_change_invalidates.rs` — `#[ignore]` stub (filled by Plan 03-05; success criterion #4)
- [ ] `crates/vector-render/tests/cursor_overlay_snapshot.rs` — `#[ignore]` stub (filled by Plan 03-03; RENDER-05)
- [ ] `crates/vector-render/tests/damage_to_quads.rs` — `#[ignore]` stub (filled by Plan 03-03)
- [ ] `crates/vector-render/tests/selection_overlay_snapshot.rs` — `#[ignore]` stub (filled by Plan 03-04; RENDER-05 + D-54)
- [ ] `crates/vector-render/tests/pty_coalesce.rs` — `#[ignore]` stub (filled by Plan 03-05; D-47)
- [ ] `crates/vector-render/tests/idle_no_redraw.rs` — `#[ignore]` stub (filled by Plan 03-05; RENDER-03)
- [ ] `crates/vector-fonts/tests/crossfont_load_bundled.rs` — `#[ignore]` stub (filled by Plan 03-02; D-41)
- [ ] `crates/vector-fonts/tests/grayscale_pixel_format.rs` — `#[ignore]` stub (filled by Plan 03-02; D-50)
- [ ] `crates/vector-input/tests/xterm_key_table.rs` — `#[ignore]` stub (filled by Plan 03-04; D-52)
- [ ] `crates/vector-input/tests/bracketed_paste_wrap.rs` — `#[ignore]` stub (filled by Plan 03-04; D-53)
- [ ] `crates/vector-app/tests/selection_render.rs` — `#[ignore]` stub (filled by Plan 03-04; D-54 + success criterion #5)
- [ ] `crates/vector-app/tests/frame_pacing.rs` — `#[ignore]` stub (filled by Plan 03-05; D-44..47, RENDER-02 + RENDER-03)
- [ ] JetBrains Mono Regular `.ttf` shipped at `crates/vector-app/resources/fonts/JetBrainsMono-Regular.ttf` (D-41); cargo-bundle picks it up via the existing Phase 1 bundle config (Plan 03-02 adds the resource entry)
- [ ] Workspace `Cargo.toml` adds: `wgpu = "29"`, `crossfont = "0.9"`, `unicode-width = "0.2"`, `bytemuck = "1"`, `bytes = "1"` (verify against current crates.io; researcher confirmed versions)

If a Wave 0 stub later turns out unnecessary, the executor deletes it in the plan that owns the matching test path — never leave orphaned `#[ignore]` files.

---

## Manual-Only Verifications

Behaviors that automated tests cannot fully verify — these require human eyes-on-glyphs or a real display. **9 items total**; Plan 03-05 Task 2 (`checkpoint:human-verify`) walks this matrix verbatim.

| # | Behavior | Requirement | Why Manual | Test Instructions |
|---|----------|-------------|------------|-------------------|
| 1 | `vim` renders correctly with visible cursor in a real window | Success criterion #1, RENDER-01, WIN-01 | Real NSWindow + Metal surface; offscreen wgpu can verify pipeline but not user-visible composition | Open `Vector.app` from `target/debug/`, run `vim /tmp/foo`, type `ihello<Esc>:wq`. Confirm: cursor visible (block), syntax color present, status bar bottom-right correct, no glyph corruption. |
| 2 | `cat large.log` sustains 60+ fps on Apple Silicon at 1080p | Success criterion #2, RENDER-02 | Frame-rate ceiling depends on real display + GPU + LPM state; CI runners have no display | On Apple Silicon Mac, run `Vector.app`, then in-shell `yes \| head -n 1000000 > /tmp/big.log && cat /tmp/big.log`. Watch Activity Monitor → GPU History; should sustain ≥ 60 fps. |
| 3 | Idle CPU < 1% with no dirty rows | Success criterion #3, RENDER-03 | Activity Monitor sampling over 60s; not script-checkable cleanly | Open `Vector.app`, do nothing for 60s. Activity Monitor → CPU column for vector-app process should sit below 1%. |
| 4 | Retina ↔ non-Retina monitor swap keeps glyphs correct, no visible stutter beyond 1 frame | Success criterion #4, RENDER-04, D-48 | Requires physical display swap | Plug external non-Retina monitor; drag `Vector.app` window between built-in Retina and external. Verify glyphs sharp on both, no broken cells, single-frame stutter at most on the swap. |
| 5 | Selection rectangle composites over live grid without flicker against the default (dark) theme | Success criterion #5, RENDER-05, D-54 | Visual stability requires real display | Run `top` or `htop`. Click-drag to select a region of live-updating output; arrow-key the cursor while selection persists; verify no flicker, no z-fighting, selection visible against dark theme. (Light-theme variant deferred to v2 — D-40 ships a single default.) |
| 6 | `Cmd-V` paste round-trip via bracketed paste in `vim` insert mode | D-53 | Pasteboard ↔ PTY round-trip is system-level | Copy `hello world` to clipboard (Cmd-C in any app), open vim in Vector, press `i` then Cmd-V; verify "hello world" inserted; verify vim doesn't think the paste is typed input (bracketed paste mode active). |
| 7 | ProMotion 120Hz honored on supported hardware | Success criterion #2, D-45 | Display capability detection | On a ProMotion device (M1 Pro/Max MBP 14"/16"), repeat the `cat large.log` test. Confirm 120Hz feel via Quartz Debug or visual smoothness. |
| 8 | Low Power Mode caps to ~30 fps with `tracing` log emitted | D-46 | NSProcessInfo state change is a system signal | Enable Low Power Mode (Settings → Battery), launch Vector, run `cat /tmp/big.log`. Verify reduced fps + `tracing` log entry recorded. Restore normal mode, verify fps returns. |
| 9 | `Cmd-Ctrl-F` fullscreen toggles cleanly | WIN-01, Success criterion #1 | NSWindow fullscreen + menu/traffic-light hide is a system behavior | In Vector, press Cmd-Ctrl-F. Window enters fullscreen, traffic-light buttons hide, menu bar auto-hides until mouse-to-top. Press again to exit cleanly. |

These items migrate to `03-HUMAN-UAT.md` automatically via the verifier in `/gsd:execute-phase 3`'s `verify_phase_goal` step.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (canonical map lives in each PLAN.md's frontmatter + `<verify>` blocks)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references listed above
- [x] No watch-mode flags (`cargo test` is single-shot)
- [x] Feedback latency < 30s
- [x] Per-task map deferred to plan frontmatters (per the rev: 2026-05-11 reconciliation) — `nyquist_compliant: true`

**Approval:** validation contract complete; Wave 0 stubs land in Plan 03-01.
