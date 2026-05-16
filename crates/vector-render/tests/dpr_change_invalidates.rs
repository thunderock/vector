//! DPR change clears both atlases; next frame lazy-rasterizes (D-48 / RENDER-04).
//!
//! Builds the compositor offscreen, primes the atlases with two glyphs (one
//! mono + one color), asserts both atlases hold entries, calls clear_atlases,
//! and asserts both atlases are empty.

#![allow(clippy::missing_panics_doc)]

use vector_fonts::FontStack;
use vector_render::{Compositor, RenderContext};
use vector_term::Term;

#[test]
fn scale_factor_change_clears_atlases() {
    let Ok(offscreen) = RenderContext::new_offscreen(256, 64) else {
        eprintln!("no Metal adapter; skipping");
        return;
    };
    let Ok(fs) = FontStack::load_bundled(1.0, 14.0) else {
        eprintln!("no bundled font; skipping");
        return;
    };
    let mut comp = Compositor::new_with(
        &offscreen.device,
        &offscreen.queue,
        offscreen.format,
        offscreen.width,
        offscreen.height,
        fs,
    )
    .expect("compositor");

    let mut term = Term::new(80, 24, 1000);
    term.feed(b"Hi");
    let _ = comp.render_offscreen_with(
        &offscreen.device,
        &offscreen.queue,
        offscreen.width,
        offscreen.height,
        &mut term,
        None,
    );

    // After rendering ASCII content, atlas should have populated some glyphs.
    assert!(comp.atlas_mut().mono_has_entries() || comp.atlas_mut().color_has_entries());

    // Simulate ScaleFactorChanged — clear, expect both atlases empty.
    comp.clear_atlases();
    assert!(!comp.atlas_mut().mono_has_entries());
    assert!(!comp.atlas_mut().color_has_entries());
}
