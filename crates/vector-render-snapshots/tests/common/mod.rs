//! Shared helpers for HARDEN-01 snapshot scenes.
//!
//! Wraps the existing offscreen harness from
//! `crates/vector-render/tests/common/offscreen.rs` (D-27) and adds perceptual
//! diff + golden load. Goldens live under `tests/goldens/` as PNG.
//!
//! Set `INSTA_UPDATE=auto` (or delete a golden file) to (re)generate goldens.

#![allow(dead_code)]

use image::{ImageBuffer, Rgba};
use std::path::PathBuf;
use vector_fonts::FontStack;
use vector_render::{Compositor, RenderContext};
use vector_term::Term;

/// D-03 — SSIM ≥ 0.98 (~delta-E 2.0) absorbs sub-pixel/antialias drift.
pub const PERCEPTUAL_THRESHOLD: f64 = 0.98;

/// Render a deterministic scene to RGBA pixels. Returns `None` on Metal-less
/// hosts (Linux dev shells / CI runners without GPU) — caller should SKIP.
pub fn render_scene(
    width: u32,
    height: u32,
    cols: u16,
    rows: u16,
    scene: impl FnOnce(&mut Term),
) -> Option<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    let ctx = RenderContext::new_offscreen(width, height).ok()?;
    let font_stack = FontStack::load_bundled(1.0, 14.0).ok()?;
    let mut comp = Compositor::new_with(
        &ctx.device,
        &ctx.queue,
        ctx.format,
        ctx.width,
        ctx.height,
        font_stack,
    )
    .ok()?;
    let mut term = Term::new(cols, rows, 1000);
    scene(&mut term);
    let frame = comp
        .render_offscreen_with(
            &ctx.device,
            &ctx.queue,
            ctx.width,
            ctx.height,
            &mut term,
            None,
        )
        .ok()?;

    // BGRA → RGBA swizzle so all goldens are stored in one canonical layout.
    let mut pixels = frame.pixels.clone();
    let needs_swap = matches!(
        frame.format,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    );
    if needs_swap {
        for chunk in pixels.chunks_mut(4) {
            chunk.swap(0, 2);
        }
    }
    ImageBuffer::from_raw(frame.width, frame.height, pixels)
}

pub fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens")
        .join(format!("{name}.png"))
}

/// Perceptual diff against the committed golden. On `INSTA_UPDATE=auto` or
/// missing golden, writes the golden and returns. On regression, writes a
/// `.diff.png` artifact next to the golden and panics with the SSIM score.
pub fn diff_or_panic(actual: &ImageBuffer<Rgba<u8>, Vec<u8>>, name: &str) {
    let gp = golden_path(name);
    let update = std::env::var("INSTA_UPDATE").as_deref() == Ok("auto");
    if update || !gp.exists() {
        if let Some(parent) = gp.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        actual.save(&gp).expect("write golden");
        eprintln!("WROTE golden: {}", gp.display());
        return;
    }

    let expected = image::open(&gp)
        .unwrap_or_else(|e| panic!("golden {} missing or unreadable: {e}", gp.display()))
        .into_rgba8();
    if expected.dimensions() != actual.dimensions() {
        let diff = gp.with_extension("diff.png");
        actual.save(&diff).ok();
        panic!(
            "snapshot diff: dimensions changed expected={:?} actual={:?} (see {})",
            expected.dimensions(),
            actual.dimensions(),
            diff.display()
        );
    }
    let a = image::DynamicImage::ImageRgba8(actual.clone()).into_rgb8();
    let b = image::DynamicImage::ImageRgba8(expected).into_rgb8();
    let r = image_compare::rgb_hybrid_compare(&a, &b).expect("compare");
    if r.score < PERCEPTUAL_THRESHOLD {
        let diff = gp.with_extension("diff.png");
        actual.save(&diff).ok();
        panic!(
            "snapshot diff: score={:.4} < {} (see {})",
            r.score,
            PERCEPTUAL_THRESHOLD,
            diff.display()
        );
    }
}
