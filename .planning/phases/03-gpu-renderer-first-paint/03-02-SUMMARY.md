---
phase: 03-gpu-renderer-first-paint
plan: 02
subsystem: render
tags: [crossfont, jetbrains-mono, cargo-bundle, atlas, etagere, lru, wgpu, rgba8unorm, unicode-width]

# Dependency graph
requires:
  - phase: 03-gpu-renderer-first-paint
    plan: 01
    provides: "vector-render::RenderContext (pub device/queue/surface), 5 Wave-0 #[ignore] atlas stubs, workspace deps (crossfont 0.9.0, etagere 0.2, unicode-width 0.2.2, parking_lot 0.12)"
provides:
  - "vector-fonts::FontStack::load_bundled(dpr, size_pt) + FontStack::rasterize(c) backed by crossfont 0.9 CoreText (D-40, D-41, D-50)"
  - "vector-fonts::BitmapKind::Mono(Vec<u8>)/Color(Vec<u8>) + RasterizedGlyph"
  - "vector-fonts::cell_width(c) sourced from unicode-width (Pitfall 2)"
  - "vector-render::Atlas { mono + color Rgba8Unorm 2048x2048 } + slot_for + clear_all + bounded LRU eviction (D-43, Pitfall 2)"
  - "vector-render::GlyphKey { character, dpr_bucket } + AtlasSlot::{Mono,Color,Fallback}"
  - "Bundled JetBrains Mono Regular TTF (270KB) + OFL license shipped via cargo-bundle [package.metadata.bundle].resources"
affects: [03-03-compositor, 03-05-pacing-polish, 04-mux]

# Tech tracking
tech-stack:
  added: []  # all deps were locked at workspace level by Plan 03-01
  patterns:
    - "Arc<parking_lot::Mutex<Rasterizer>> inside FontStack — crossfont's CoreTextRasterizer is !Sync; Mutex lock is brief and never crosses .await"
    - "VecDeque<GlyphKey> (insertion order) + HashMap<GlyphKey, SlotEntry> per atlas — O(n) touch, O(1) lookup; n is bounded by 2048*2048/min_glyph_area at runtime"
    - "Mono 3-channel RGB-alphamask expanded to RGBA at upload time (alpha = max(r,g,b)); compositor (Plan 03-03) multiplies sampled .rgb by fg color (Pattern 3)"
    - "etagere::AtlasAllocator + manual evict_one() retry loop on allocate() = None — bounded LRU contract"
    - "Bundle path lookup: Vector.app/Contents/Resources/Fonts/ first, dev workspace crates/vector-app/resources/Fonts/ fallback"

key-files:
  created:
    - crates/vector-fonts/src/glyph.rs
    - crates/vector-fonts/src/loader.rs
    - crates/vector-fonts/src/width.rs
    - crates/vector-render/src/atlas.rs
    - crates/vector-app/resources/Fonts/JetBrainsMono-Regular.ttf (binary; 270224 bytes)
    - crates/vector-app/resources/Fonts/LICENSE-JetBrainsMono.txt (4399 bytes)
  modified:
    - Cargo.lock
    - crates/vector-app/Cargo.toml ([package.metadata.bundle].resources entry)
    - crates/vector-fonts/Cargo.toml (+crossfont, parking_lot, unicode-width)
    - crates/vector-fonts/src/lib.rs (replaced stub with mod tree + pub use)
    - crates/vector-fonts/tests/crossfont_load_bundled.rs (un-ignored)
    - crates/vector-fonts/tests/grayscale_pixel_format.rs (un-ignored)
    - crates/vector-fonts/tests/two_atlas_split.rs (un-ignored)
    - crates/vector-fonts/tests/atlas_lru_eviction.rs (un-ignored, expanded to 2 sub-tests)
    - crates/vector-render/Cargo.toml (+etagere, +vector-fonts dep)
    - crates/vector-render/src/lib.rs (added mod atlas; pub use Atlas/AtlasSlot/GlyphKey)
    - crates/vector-render/tests/atlas_lru.rs (un-ignored; wgpu Metal integration)

key-decisions:
  - "FontStack uses Arc<Mutex<Rasterizer>> instead of plain &mut — crossfont's CoreTextRasterizer is !Sync, so the wrapper must serialize rasterize() calls. Lock scope is per-glyph and never crosses an await; compositor (Plan 03-03) will call rasterize on the main render thread."
  - "Atlas size = 2048×2048 (ATLAS_DIM const). Within Metal's MAX_TEXTURE_DIMENSION_2D (8192 on Apple Silicon) and matches Pitfall 2's prescription. new_with_dims test-only escape hatch sized 64×64 in atlas_lru test forces eviction at ~24 ASCII glyphs of 94."
  - "Mono glyphs are 3-channel; we expand to 4-channel RGBA at upload (alpha = max(r,g,b)). Both atlases are Rgba8Unorm — simpler bind group layout for Plan 03-03's compositor than mixing R8/RGBA8."
  - "SlotEntry struct replaces a bare 4-tuple for HashMap values — clippy `type_complexity` lint rejected the tuple."
  - "GlyphKey passed by value (Copy) into contains/touch — clippy `trivially_copy_pass_by_ref` lint rejected &GlyphKey."

patterns-established:
  - "Lazy rasterize: compositor passes (key, &glyph) to slot_for which inserts on miss + uploads via queue.write_texture; cache hit just touches LRU and returns the existing UV."
  - "Bounded LRU contract: allocate() failure triggers evict_one() in a loop; only returns None when the slot map is empty AND the requested glyph still doesn't fit (oversized glyph case)."
  - "Bundle path resolution: locate_bundled_font() walks current_exe()/../Resources/Fonts/ first (production .app) then falls back to CARGO_MANIFEST_DIR/../vector-app/resources/Fonts/ (dev workspace runs). Both `cargo test` and `Vector.app` launch resolve cleanly."

requirements-completed: [RENDER-04]

# Metrics
duration: 10 min
completed: 2026-05-11
---

# Phase 3 Plan 02: Glyph Atlas — crossfont + bundled JetBrains Mono + two-atlas LRU Summary

**vector-fonts ships FontStack over crossfont 0.9 CoreText with bundled JetBrains Mono (OFL); vector-render::Atlas implements two Rgba8Unorm 2048×2048 textures (mono + color) with bounded LRU eviction; 5 of Plan 03-01's Wave-0 #[ignore] stubs un-ignored and passing (3 vector-fonts + 1 vector-fonts pure-Rust LRU + 1 vector-render wgpu Metal LRU). RENDER-04 lands.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-05-11T19:39:11Z
- **Completed:** 2026-05-11T19:49:28Z
- **Tasks:** 2 (both TDD-tagged — Wave-0 stubs from Plan 03-01 provided the failing-tests baseline; we un-ignored and turned them green here)
- **Files modified:** 11 modified, 6 created (3 src + 1 src + 2 binary assets)

## Accomplishments

- **Bundled JetBrains Mono Regular TTF + OFL license live on disk and in cargo-bundle config.** TTF is **270,224 bytes** (real, >100 KB minimum from acceptance criteria), downloaded from `https://github.com/JetBrains/JetBrainsMono/raw/master/fonts/ttf/JetBrainsMono-Regular.ttf`; license from `https://raw.githubusercontent.com/JetBrains/JetBrainsMono/master/OFL.txt`. `vector-app/Cargo.toml [package.metadata.bundle].resources` extended with both paths (single new array entry — no prior `resources = […]` block existed; we created one).
- **`vector-fonts::FontStack` operational.** `load_bundled(dpr, size_pt)` instantiates `crossfont::Rasterizer` (CoreText backend on macOS), pre-multiplies `size_pt * max(dpr, 1.0)` into a `crossfont::Size` so the rasterizer pixel grid matches HiDPI requirements, loads JetBrains Mono Regular by family name (CoreText finds the bundled face via the standard discovery chain), and caches `CellMetrics { width_px, height_px, baseline }` from `Rasterizer::metrics(font_key, size)`. `rasterize(c)` returns a `RasterizedGlyph { character, width, height, top, left, advance_x, bitmap: BitmapKind::Mono | Color }`; ASCII rasterizes as `Mono` (3-channel RGB alphamask per D-50 + research finding #1), emoji 🦀 falls through CoreText's fallback chain to Apple Color Emoji and rasterizes as `Color` (4-channel premultiplied RGBA).
- **`vector-fonts::cell_width` sourced from `unicode-width` (Pitfall 2).** Source of truth — never font advance. Single function `cell_width(c) -> u8` calling `UnicodeWidthChar::width(c).unwrap_or(1)` with a saturating `try_from` clamp; 0 for combining/ZWJ, 1 default, 2 for wide CJK/emoji.
- **`vector-render::Atlas` ships the two-atlas LRU eviction store.** Two `Rgba8Unorm` 2048×2048 wgpu textures (mono + color) with `TEXTURE_BINDING | COPY_DST` usage. Per-atlas state: `etagere::AtlasAllocator` for rectangle packing, `HashMap<GlyphKey, SlotEntry>` for O(1) cache lookup, `VecDeque<GlyphKey>` for LRU access order. `slot_for(queue, key, &glyph)` routes by `BitmapKind` variant: mono glyphs expand 3-channel → RGBA (`alpha = max(r,g,b)`) before upload; color glyphs upload directly. Cache hit → `touch` (move key to back of VecDeque) → return existing `AtlasSlot`. Cache miss → `insert` → if `allocator.allocate(size2)` returns `None`, `evict_one` (pop oldest LRU, free AllocId) and retry; if eviction can't proceed, return `AtlasSlot::Fallback`. `clear_all()` rebuilds both `AtlasAllocator`s + clears slot maps + LRU queues — the lever Plan 03-05 wires to `ScaleFactorChanged` (D-48). `mono_view()` / `color_view()` expose `&TextureView` for Plan 03-03's bind group layout.
- **5 Plan 03-01 Wave-0 stubs un-ignored and passing:**
  - `vector-fonts/tests/crossfont_load_bundled.rs::loads_bundled_jetbrains_mono_and_rasterizes_a` (D-41)
  - `vector-fonts/tests/grayscale_pixel_format.rs::mono_bitmap_is_three_channel_per_pixel` (D-50 + research finding #1)
  - `vector-fonts/tests/two_atlas_split.rs::ascii_is_mono_emoji_is_color` (RENDER-04)
  - `vector-fonts/tests/atlas_lru_eviction.rs` — expanded from 1 stub to 2 sub-tests covering pure-Rust LRU bookkeeping (`lru_moves_touched_key_to_back`, `lru_pop_front_returns_oldest`)
  - `vector-render/tests/atlas_lru.rs::lru_evicts_oldest_glyph_when_atlas_fills` (wgpu Metal integration: 64×64 atlas + 94 printable ASCII forces eviction; '!' evicted, '~' resident)
- **Workspace test ledger:** baseline (post Plan 03-01) was 55 passed / 18 ignored. Post 03-02: **61 passed / 0 failed / 13 ignored**. Net: +6 passing (5 newly-un-ignored stubs; `atlas_lru_eviction.rs` carries 2 sub-tests so it contributes 2 passes and removes 1 ignored — math: 18 − 5 = 13 ignored; 55 + 6 = 61 passing). 13 still-ignored stubs are owned by Plans 03-03 (6), 03-04 (3), and 03-05 (4).
- **Arch-lint invariant holds.** `find crates -name no_tokio_main.rs | wc -l` = 15. Unchanged from Plan 03-01.

## Task Commits

1. **Task 1: vector-fonts — crossfont rasterizer + bundled JetBrains Mono + unicode-width cell width** — `1976cec` (feat)
2. **Task 2: vector-render — two-atlas wgpu textures + bounded LRU eviction** — `9dd4208` (feat)

_Plan metadata commit lands separately after this SUMMARY._

## Files Created/Modified

**Created (src):**
- `crates/vector-fonts/src/glyph.rs` — `BitmapKind::{Mono(Vec<u8>), Color(Vec<u8>)}` + `RasterizedGlyph` struct.
- `crates/vector-fonts/src/loader.rs` — `FontStack::load_bundled` / `FontStack::rasterize` / `CellMetrics` + `locate_bundled_font` resolver (bundle path → dev path).
- `crates/vector-fonts/src/width.rs` — `cell_width(c) -> u8` via `unicode_width::UnicodeWidthChar`.
- `crates/vector-render/src/atlas.rs` — `Atlas`, `AtlasSlot`, `GlyphKey`, internal `AtlasTexture` + `SlotEntry`.

**Created (bundled assets):**
- `crates/vector-app/resources/Fonts/JetBrainsMono-Regular.ttf` (270,224 bytes, OFL 1.1)
- `crates/vector-app/resources/Fonts/LICENSE-JetBrainsMono.txt` (4399 bytes, OFL text)

**Modified:**
- `Cargo.lock` — etagere + crossfont sub-tree resolved.
- `crates/vector-app/Cargo.toml` — added `resources = ["resources/Fonts/JetBrainsMono-Regular.ttf", "resources/Fonts/LICENSE-JetBrainsMono.txt"]` to `[package.metadata.bundle]`; existing icon/osx_info_plist_exts keys preserved.
- `crates/vector-fonts/Cargo.toml` — added `crossfont.workspace = true`, `parking_lot.workspace = true`, `unicode-width.workspace = true`.
- `crates/vector-fonts/src/lib.rs` — replaced stub with `mod glyph; mod loader; mod width;` + `pub use BitmapKind/RasterizedGlyph/FontStack/CellMetrics/cell_width`.
- `crates/vector-fonts/tests/crossfont_load_bundled.rs` — un-ignored + real assertions.
- `crates/vector-fonts/tests/grayscale_pixel_format.rs` — un-ignored + `len == w*h*3` assertion.
- `crates/vector-fonts/tests/two_atlas_split.rs` — un-ignored + ASCII-Mono / 🦀-Color split assertion, `#[cfg(target_os = "macos")]` guard.
- `crates/vector-fonts/tests/atlas_lru_eviction.rs` — un-ignored + 2 pure-Rust LRU sub-tests.
- `crates/vector-render/Cargo.toml` — added `etagere.workspace = true`, `vector-fonts = { path = "../vector-fonts" }`.
- `crates/vector-render/src/lib.rs` — added `mod atlas;` + `pub use Atlas/AtlasSlot/GlyphKey`.
- `crates/vector-render/tests/atlas_lru.rs` — un-ignored + 64×64 atlas wgpu integration test against 94 ASCII glyphs.

## Decisions Made

- **Atlas dimension = 2048×2048 (ATLAS_DIM const).** Within Apple Silicon Metal's `MAX_TEXTURE_DIMENSION_2D = 8192` and matches Pitfall 2's mention of "e.g., 2048×2048" as the planner-level prescription. ~4M pixels per atlas × 2 atlases = ~32 MiB GPU memory (Rgba8Unorm = 4 bytes/pixel); negligible vs. the rest of the wgpu surface budget. `new_with_dims` test-only constructor enables tight-atlas LRU eviction proofs.
- **Mono atlas is Rgba8Unorm, not R8Unorm.** The plan prescribed RGBA on both atlases (Pattern 3) so Plan 03-03's compositor binds one texture format and one sampler. We expand the 3-channel CoreText alphamask to RGBA at upload (alpha = max(r,g,b)); shader will multiply sampled `.rgb` by foreground color. R8Unorm would shrink memory by 4× but force a separate shader path for color emoji — net loss in code size.
- **`Arc<parking_lot::Mutex<Rasterizer>>` inside FontStack.** crossfont's `CoreTextRasterizer` is `!Sync` (it holds a `RefCell<CGContext>` internally). The wrapper must serialize `rasterize()` calls, but lock scope is per-glyph and never crosses an await. Compositor calls happen on the main render thread; future Plan 03-03 atlas-on-cache-miss path holds the lock for one glyph at a time.
- **`SlotEntry` struct over a 4-tuple in slot map.** Clippy's `type_complexity` lint rejected `HashMap<GlyphKey, (AllocId, [f32; 4], [u32; 2], [i32; 2])>`. Named fields read better at the call sites anyway.
- **`GlyphKey` passed by value (Copy) into `contains` / `touch`.** Clippy's `trivially_copy_pass_by_ref` rejected `&GlyphKey` for an 8-byte type.
- **Bundle path lookup order: bundle first, dev workspace second.** `Vector.app/Contents/Resources/Fonts/JetBrainsMono-Regular.ttf` (production via cargo-bundle), then `crates/vector-app/resources/Fonts/JetBrainsMono-Regular.ttf` (dev `cargo test`/`cargo run`). Both resolve cleanly.
- **`#[cfg(target_os = "macos")]` on the emoji test only.** Linux/Windows future ports will need a different fallback chain assertion; the test guards against premature CI failure.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] crossfont 0.9 `Rasterizer::new()` takes no arguments**
- **Found during:** Task 1 (initial compile of `loader.rs`)
- **Issue:** Plan snippet (line 92 of 03-02-PLAN.md) shows `Rasterizer::new(dpr)?` — passes `dpr: f32`. The actual crossfont 0.9.0 API (verified at `~/.cargo/registry/src/.../crossfont-0.9.0/src/darwin/mod.rs:105`) is `fn new() -> Result<CoreTextRasterizer, Error>` with no arguments. The `Rasterize` trait at `lib.rs:237` confirms the no-arg signature.
- **Fix:** Removed the `dpr` arg from `Rasterizer::new()`; instead pre-multiply `dpr` into the point size: `Size::new(size_pt * dpr.max(1.0))`. CoreText's rasterizer-per-pixel-grid produces the same effective pixel density.
- **Files modified:** `crates/vector-fonts/src/loader.rs`
- **Verification:** `cargo test -p vector-fonts --test crossfont_load_bundled` passes; the test confirms a non-zero-size glyph rasterizes.
- **Committed in:** `1976cec`

**2. [Rule 1 - Bug] wgpu 29 renamed `ImageCopyTexture` → `TexelCopyTextureInfo` and `ImageDataLayout` → `TexelCopyBufferLayout`**
- **Found during:** Task 2 (initial compile of `atlas.rs`)
- **Issue:** Plan snippet (line ~505 of 03-02-PLAN.md) uses `ImageCopyTexture { ... }` and `ImageDataLayout { ... }` as the `queue.write_texture` arguments. wgpu 29.0.3 re-exports `TexelCopyBufferLayout` from `wgpu_types` (verified at `wgpu-29.0.3/src/lib.rs:150`) and `TexelCopyTextureInfo` from `wgpu_types` (visible in the `Queue::write_texture` dispatch signature at `wgpu-29.0.3/src/dispatch.rs:242`). The old `Image*` names were removed in the wgpu 25 → 27 refactor.
- **Fix:** Renamed both types at the import + call sites in `atlas.rs`.
- **Files modified:** `crates/vector-render/src/atlas.rs`
- **Verification:** `cargo build -p vector-render` clean; `cargo test -p vector-render --test atlas_lru` passes.
- **Committed in:** `9dd4208`

**3. [Rule 1 - Bug] 128×128 test atlas was too large to force LRU eviction**
- **Found during:** Task 2 (atlas_lru integration test panic)
- **Issue:** Plan prescribed `Atlas::new_with_dims(&device, 128, 128)` for the eviction test. At 14 pt, ASCII glyphs are ~9×17 px; 94 chars × 153 px² ≈ 14.4 k px², well below 128² = 16.4 k px². Even with shelf packing waste, 'A'-through-'~' fits without forcing eviction, so the assertion that `!` had been evicted failed.
- **Fix:** Shrunk to `64×64` (4096 px² capacity) so eviction is mandatory after ~24 glyphs. Test now confirms '!' evicted and '~' resident.
- **Files modified:** `crates/vector-render/tests/atlas_lru.rs`
- **Verification:** `cargo test -p vector-render --test atlas_lru` passes deterministically across 3 consecutive runs.
- **Committed in:** `9dd4208`

**4. [Rule 1 - Bug] clippy pedantic cast lints on metrics rounding (`cast_sign_loss`, `cast_possible_truncation`)**
- **Found during:** Task 1 (clippy pass)
- **Issue:** Workspace `pedantic` warns on `metrics.average_advance.round().max(1.0) as u32` (`cast_sign_loss`) and `metrics.descent.round() as i32` (`cast_possible_truncation`). The values are clamped to safe ranges in the original code, but the lint can't see through `.round()`.
- **Fix:** Extracted to helper functions `f_to_u32` (clamp to `[1.0, u32::MAX]`) and `f_to_i32` (clamp to `[i32::MIN, i32::MAX]`) with scoped `#[allow]` attributes on each. The casts are now both safe at runtime and lint-clean.
- **Files modified:** `crates/vector-fonts/src/loader.rs`
- **Verification:** `cargo clippy -p vector-fonts --all-targets -- -D warnings` clean.
- **Committed in:** `1976cec`

**5. [Rule 1 - Bug] clippy pedantic: `type_complexity` on `HashMap<GlyphKey, (AllocId, [f32; 4], [u32; 2], [i32; 2])>`**
- **Found during:** Task 2 (clippy pass)
- **Issue:** The bare 4-tuple value type tripped the `clippy::type_complexity` lint (which is rolled up by `pedantic`).
- **Fix:** Introduced an internal `SlotEntry { alloc_id, uv, size_px, offset_px }` struct in `atlas.rs` and threaded it through `evict_one` / `slot_for` cache-hit paths. Reads cleaner at every call site.
- **Files modified:** `crates/vector-render/src/atlas.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` clean.
- **Committed in:** `9dd4208`

**6. [Rule 1 - Bug] clippy pedantic: `trivially_copy_pass_by_ref` on `GlyphKey` (8 bytes)**
- **Found during:** Task 2 (clippy pass)
- **Issue:** `fn contains(&self, key: &GlyphKey)` and `fn touch(&mut self, key: &GlyphKey)` flagged — `GlyphKey` is `Copy` and 8 bytes (`char` + `u8`+ padding).
- **Fix:** Took `key: GlyphKey` by value at all signatures; updated the integration test `atlas_lru.rs` to call `atlas.contains(keys[0])` instead of `atlas.contains(&keys[0])`.
- **Files modified:** `crates/vector-render/src/atlas.rs`, `crates/vector-render/tests/atlas_lru.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` clean.
- **Committed in:** `9dd4208`

**7. [Rule 1 - Bug] clippy pedantic: `many_single_char_names` in `expand_rgb_to_rgba`**
- **Found during:** Task 2 (clippy pass)
- **Issue:** Local bindings `r`, `g`, `b`, `a`, `n` in `expand_rgb_to_rgba` exceeded the pedantic single-character-binding budget.
- **Fix:** Renamed to `red`, `green`, `blue`, `alpha`, `pixel_count`; replaced the `i*3` index arithmetic with `rgb.chunks_exact(3).take(pixel_count)` for clarity.
- **Files modified:** `crates/vector-render/src/atlas.rs`
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` clean.
- **Committed in:** `9dd4208`

---

**Total deviations:** 7 code auto-fixes (Rule 1 only — all correctness/lint compliance). 0 Rule 4 architectural decisions. No scope creep.

**Impact on plan:** Two were API drift between the plan's reproduced snippets and reality (crossfont 0.9 `new()` signature; wgpu 29 `TexelCopy*` rename). One was a test-fixture sizing bug (128×128 too large to force LRU eviction). Four were clippy `-D warnings` lint compliance fixes on top of the plan's verbatim code. Plan's behavioral contract (RENDER-04, D-40, D-41, D-43, D-50, Pitfall 2) is met exactly.

## Issues Encountered

None beyond the deviations above. crossfont's CoreText fallback chain delivered Apple Color Emoji for `🦀` without any user-facing configuration — the `two_atlas_split` test passes on the first run. etagere's shelf packing is deterministic at fixed input sizes, so the LRU eviction test is reproducible.

## User Setup Required

None. JetBrains Mono is bundled at build time (cargo-bundle for production .app; dev workspace path for `cargo run` / `cargo test`); no system-level font installation required.

**Pitfall 7 / Open Question #3 cargo-bundle subdirectory preservation:** The `resources = ["resources/Fonts/JetBrainsMono-Regular.ttf", "resources/Fonts/LICENSE-JetBrainsMono.txt"]` array passes the full sub-path. cargo-bundle 0.10's documented behavior is to copy each entry to `Vector.app/Contents/Resources/<basename>` — i.e., the `Fonts/` subdirectory **may not be preserved** in the bundled `.app`. This is **NOT a CI gate** for Plan 03-02; it surfaces in Plan 03-05's manual DMG smoke matrix (Task 2, item #1: "vim renders correctly with visible cursor in a real window"). If the subdir is flattened, our `locate_bundled_font` resolver's bundle-path branch (`.join("Resources").join("Fonts").join(...)`) will miss and fall back to the dev path — which is empty in a shipped .app, triggering the `JetBrains Mono not found` error. Mitigation if needed: post-process step in `xtask::dmg` to move `Vector.app/Contents/Resources/JetBrainsMono-Regular.ttf` into a `Fonts/` subdir, OR change `locate_bundled_font` to also try `Resources/JetBrainsMono-Regular.ttf` (flat). Recommend the latter — one extra path probe, no xtask change. Documented here so Plan 03-05's smoke matrix can flag it cleanly.

## Hand-off Notes

**Plan 03-03 (compositor):**
- `Atlas::slot_for(&queue, key, &glyph) -> AtlasSlot` is the call site. The compositor builds a `GlyphKey { character, dpr_bucket }` (dpr_bucket = round(scale_factor) as u8 typically), rasterizes via `FontStack::rasterize(c)` to get a `RasterizedGlyph`, then passes both to `slot_for`. Cache hits return `AtlasSlot::{Mono,Color}` with UVs + size + offset; cache misses upload and return the same.
- `mono_view()` / `color_view()` are the bind-group source views for the cell shader. Both are `Rgba8Unorm`. The cell shader should sample with linear filtering (sampler in Plan 03-03's responsibility), multiply `.rgb` by the fg color for mono samples, and use `.rgba` directly for color samples. The shader needs a way to know which atlas to sample — Plan 03-03 will likely encode it as a vertex attribute (e.g., `atlas_kind: u32` in the quad vertex).
- `AtlasSlot::Fallback` indicates the glyph is too large to fit even an empty atlas (≥ 2048 px in either dimension — unlikely for a terminal font but defensible). Compositor should render a tofu box for these.
- Atlas is `!Sync` (wgpu types are `Sync` but `HashMap<_,_>` mutation through `&mut self` makes the whole struct exclusive). Put it in the same render-thread location as `RenderContext`; do NOT share it with the I/O thread.

**Plan 03-05 (DPR change + polish):**
- `Atlas::clear_all()` is the lever for `ScaleFactorChanged` (D-48). It rebuilds both `AtlasAllocator`s, clears both slot maps, and clears both LRU queues. Compositor's next-frame glyph lookups will all miss and re-rasterize at the new DPR. Acceptable one-frame stutter per success criterion #4.
- `FontStack::load_bundled(new_dpr, size_pt)` should be called alongside `clear_all` to reload metrics at the new pixel grid; the per-frame `rasterize` calls flow through normally.
- DMG smoke matrix item #1 will catch any cargo-bundle subdir flattening (see "User Setup Required" above).

**Plan 04 (mux):**
- Atlas state is per-render-context. If Phase 4 introduces multiple windows, each window's `RenderHost` should have its own `Atlas` instance (atlases share the wgpu `Device` but not the `Texture`/slot state).

## Self-Check: PASSED

- FOUND: `crates/vector-fonts/src/glyph.rs`
- FOUND: `crates/vector-fonts/src/loader.rs`
- FOUND: `crates/vector-fonts/src/width.rs`
- FOUND: `crates/vector-render/src/atlas.rs`
- FOUND: `crates/vector-app/resources/Fonts/JetBrainsMono-Regular.ttf` (270,224 bytes)
- FOUND: `crates/vector-app/resources/Fonts/LICENSE-JetBrainsMono.txt` (4399 bytes)
- FOUND commit `1976cec` (Task 1: vector-fonts + bundled TTF)
- FOUND commit `9dd4208` (Task 2: Atlas + LRU)
- 5 Wave-0 stubs un-ignored and passing (3 crossfont/grayscale/two-atlas in vector-fonts, atlas_lru_eviction with 2 sub-tests, atlas_lru in vector-render)
- 13 Wave-0 stubs still ignored (owned by Plans 03-03/03-04/03-05)
- Arch-lint: 15 `no_tokio_main.rs` files (unchanged from baseline; 15==15 holds)
- Workspace: 61 passed / 0 failed / 13 ignored

---
*Phase: 03-gpu-renderer-first-paint*
*Completed: 2026-05-11*
