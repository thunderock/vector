//! Headless offscreen render of an empty grid: every pixel should match the bg color.

#[path = "common/offscreen.rs"]
mod offscreen;

use offscreen::channel_indices;

#[test]
fn empty_grid_paints_bg_color() {
    let Some((mut comp, ctx)) = offscreen::build_compositor(200, 100) else {
        return;
    };
    let mut term = vector_term::Term::new(10, 5, 100);
    let frame = comp
        .render_offscreen_with(
            &ctx.device,
            &ctx.queue,
            ctx.width,
            ctx.height,
            &mut term,
            None,
        )
        .expect("render_offscreen_with");
    let (r_idx, g_idx, b_idx) = channel_indices(frame.format);
    // bg = [0.06, 0.06, 0.06] in linear. Allow some headroom for the cursor cell at (0,0).
    let mut bright_pixels = 0u32;
    for px in frame.pixels.chunks_exact(4) {
        let r = px[r_idx];
        let g = px[g_idx];
        let b = px[b_idx];
        if r > 64 || g > 64 || b > 64 {
            bright_pixels += 1;
        }
    }
    let total = frame.width * frame.height;
    // Cursor cell is bright; the cursor is at most ~cell_w * cell_h pixels.
    let cursor_budget = comp.cell_width_px() * comp.cell_height_px() * 2;
    assert!(
        bright_pixels < cursor_budget,
        "expected mostly-dark frame, got {bright_pixels} bright of {total} (cursor budget {cursor_budget})",
    );
}
