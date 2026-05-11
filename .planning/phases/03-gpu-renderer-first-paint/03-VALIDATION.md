---
phase: 03
slug: gpu-renderer-first-paint
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-11
---

# Phase 03 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Bootstrapped from `03-RESEARCH.md §Validation Architecture`. Planner will populate the Per-Task Verification Map once plans exist; this file ships as a skeleton and is consumed by the gsd-planner agent to ensure every task has a verification path.

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
- GPU snapshot tests via `wgpu::TextureView` readback to committed PNG fixtures under `crates/vector-render/tests/snapshots/` (offscreen render — no display required, matches Phase 1 CI on `macos-14` runners)

---

## Sampling Rate

- **After every task commit:** Run `cargo test --workspace --tests -q` (quick)
- **After every plan wave:** Run quick + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo fmt --all -- --check`
- **Before `/gsd:verify-work`:** Full suite (`--release`) must be green plus the manual smoke matrix below
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

*Populated by gsd-planner once PLAN.md files exist. Each task in each plan must map to a row here OR be classified as Wave 0 (test scaffolding) OR appear in the Manual-Only Verifications table below.*

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| _TBD by planner_ | | | | | | | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Test scaffolding that must land before later waves run (matches Phase 2's Plan 02-01 model):

- [ ] `crates/vector-render/tests/snapshot_clearcolor.rs` — `#[ignore]` stub (filled by Plan 03-01 or 03-03)
- [ ] `crates/vector-render/tests/snapshot_singlecell.rs` — `#[ignore]` stub
- [ ] `crates/vector-render/tests/snapshot_truecolor.rs` — `#[ignore]` stub (RENDER-04)
- [ ] `crates/vector-render/tests/atlas_lru.rs` — `#[ignore]` stub (Plan 03-02; Pitfall 2 prescription)
- [ ] `crates/vector-render/tests/dpr_change_invalidates.rs` — `#[ignore]` stub (Plan 03-05; success criterion #4)
- [ ] `crates/vector-fonts/tests/crossfont_load_bundled.rs` — `#[ignore]` stub (Plan 03-02; D-41)
- [ ] `crates/vector-fonts/tests/grayscale_pixel_format.rs` — `#[ignore]` stub (Plan 03-02; D-50)
- [ ] `crates/vector-app/tests/xterm_key_table.rs` — `#[ignore]` stub (Plan 03-04; D-52)
- [ ] `crates/vector-app/tests/bracketed_paste.rs` — `#[ignore]` stub (Plan 03-04; D-53)
- [ ] `crates/vector-app/tests/selection_render.rs` — `#[ignore]` stub (Plan 03-04 + 03-03; D-54 + success criterion #5)
- [ ] `crates/vector-app/tests/frame_pacing.rs` — `#[ignore]` stub (Plan 03-05; D-44..47, RENDER-02 + RENDER-03)
- [ ] JetBrains Mono Regular `.ttf` shipped at `crates/vector-app/resources/fonts/JetBrainsMono-Regular.ttf` (D-41); cargo-bundle picks it up via the existing Phase 1 bundle config (Plan 03-02 adds the resource entry)
- [ ] Workspace `Cargo.toml` adds: `wgpu = "29"`, `crossfont = "0.9"`, `unicode-width = "0.2"`, `bytemuck = "1"` (verify against current crates.io; researcher confirmed versions)

If a Wave 0 stub later turns out unnecessary, the executor deletes it in the plan that owns the matching test path — never leave orphaned `#[ignore]` files.

---

## Manual-Only Verifications

Behaviors that automated tests cannot fully verify — these require human eyes-on-glyphs or a real display.

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `vim` renders correctly with visible cursor in a real window | Success criterion #1, RENDER-01, WIN-01 | Real NSWindow + Metal surface; offscreen wgpu can verify pipeline but not user-visible composition | Open `Vector.app` from `target/debug/`, run `vim /tmp/foo`, type `ihello<Esc>:wq`. Confirm: cursor visible (block), syntax color present, status bar bottom-right correct, no glyph corruption. |
| `cat large.log` sustains 60+ fps on Apple Silicon at 1080p (and 120 fps on ProMotion) | Success criterion #2, RENDER-02 | Frame-rate ceiling depends on real display + GPU + LPM state; CI runners have no display | On Apple Silicon Mac, run `Vector.app`, then in-shell `yes | head -n 1000000 > /tmp/big.log && cat /tmp/big.log`. Watch Activity Monitor → GPU History; should sustain target fps. ProMotion check: same on a 14"/16" M1 Pro/Max MBP. |
| Idle CPU < 1% with no dirty rows | Success criterion #3, RENDER-03 | Activity Monitor sampling over 60s; not script-checkable cleanly | Open `Vector.app`, do nothing for 60s. Activity Monitor → CPU column for vector-app process should sit below 1%. |
| Retina ↔ non-Retina monitor swap keeps glyphs correct, no visible stutter beyond 1 frame | Success criterion #4, RENDER-04 | Requires physical display swap | Plug external non-Retina monitor; drag `Vector.app` window between built-in Retina and external. Verify glyphs sharp on both, no broken cells, single-frame stutter at most on the swap. |
| Selection rectangle composites over live grid without flicker; arrow-key cursor moves cleanly under selection | Success criterion #5, RENDER-05 | Visual stability requires real display | Run `top` or `htop`. Click-drag to select a region of live-updating output; arrow-key the cursor while selection persists; verify no flicker, no z-fighting, selection visible against both dark and light theme. |
| `Cmd-V` paste round-trip via bracketed paste in `vim` insert mode | D-53 | Pasteboard ↔ PTY round-trip is system-level | Copy `hello world` to clipboard (Cmd-C in any app), open vim in Vector, press `i` then Cmd-V; verify "hello world" inserted; verify vim doesn't think the paste is typed input (bracketed paste mode active). |
| ProMotion 120Hz honored on supported hardware | Success criterion #2 | Display capability detection | Same as 60fps test, on a ProMotion device. Confirm frame rate via Quartz Debug or visual smoothness. |
| Low Power Mode caps to ~30 fps with `tracing` log emitted | D-46 | NSProcessInfo state change is a system signal | Enable Low Power Mode (Settings → Battery), launch Vector, run `cat /tmp/big.log`. Verify reduced fps + tracing log entry recorded. Restore normal mode, verify fps returns. |

These items will be migrated to `03-HUMAN-UAT.md` automatically by the verifier in `/gsd:execute-phase 3`'s `verify_phase_goal` step.

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies (planner ensures this in PLAN.md frontmatter)
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references listed above
- [ ] No watch-mode flags (`cargo test` is single-shot)
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter once planner + plan-checker agree map is complete

**Approval:** pending
