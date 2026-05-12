//! D-66: offscreen pixel snapshot showing the active-pane border on viewport edges.
//! Plan 04-04: cell.wgsl's `border_color` uniform paints pixels within `border_width_px`
//! of any viewport edge.

#![allow(clippy::many_single_char_names)]

#[path = "common/offscreen.rs"]
mod offscreen;

use offscreen::channel_indices;

#[test]
fn border_color_some_renders_red_border_on_edges() {
    let Some((mut comp, ctx)) = offscreen::build_compositor(200, 120) else {
        // No Metal adapter available (e.g. headless Linux); skip gracefully.
        return;
    };
    comp.set_border_color(&ctx.queue, [1.0, 0.0, 0.0, 1.0]);
    // Term must be sized so its cells cover the whole surface; the border test
    // runs in the cell fragment shader, not for areas outside any cell.
    let cell_w = comp.cell_width_px() as usize;
    let cell_h = comp.cell_height_px() as usize;
    let cols = u16::try_from(ctx.width as usize / cell_w.max(1) + 1).unwrap_or(80);
    let rows = u16::try_from(ctx.height as usize / cell_h.max(1) + 1).unwrap_or(24);
    let mut term = vector_term::Term::new(cols, rows, 100);
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
    let w = frame.width as usize;
    let h = frame.height as usize;

    // Sample the top edge (row 0): every pixel should be red-dominant.
    let mut red_top = 0u32;
    for col in 0..w {
        let off = col * 4;
        let r = frame.pixels[off + r_idx];
        let g = frame.pixels[off + g_idx];
        let b = frame.pixels[off + b_idx];
        if r > 200 && g < 50 && b < 50 {
            red_top += 1;
        }
    }
    assert!(
        red_top as usize > w * 9 / 10,
        "top edge should be mostly red; got {red_top}/{w}"
    );

    // Sample row 50 (interior, ~middle): pixels should NOT be red-dominant.
    let interior_row = 50.min(h - 1);
    let mut red_interior = 0u32;
    for col in 4..w.saturating_sub(4) {
        let off = (interior_row * w + col) * 4;
        let r = frame.pixels[off + r_idx];
        let g = frame.pixels[off + g_idx];
        let b = frame.pixels[off + b_idx];
        if r > 200 && g < 50 && b < 50 {
            red_interior += 1;
        }
    }
    // Interior should be the dark background; allow a tiny budget for any noise.
    assert!(
        red_interior < 4,
        "interior row {interior_row} should have no red border pixels; got {red_interior}"
    );
}

#[test]
fn border_color_alpha_zero_renders_no_border() {
    let Some((mut comp, ctx)) = offscreen::build_compositor(200, 120) else {
        return;
    };
    // Default border_color is [0,0,0,0]; explicit set to confirm.
    comp.set_border_color(&ctx.queue, [1.0, 0.0, 0.0, 0.0]);
    let cell_w = comp.cell_width_px() as usize;
    let cell_h = comp.cell_height_px() as usize;
    let cols = u16::try_from(ctx.width as usize / cell_w.max(1) + 1).unwrap_or(80);
    let rows = u16::try_from(ctx.height as usize / cell_h.max(1) + 1).unwrap_or(24);
    let mut term = vector_term::Term::new(cols, rows, 100);
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
    let w = frame.width as usize;
    let mut red_top = 0u32;
    for col in 0..w {
        let off = col * 4;
        let r = frame.pixels[off + r_idx];
        let g = frame.pixels[off + g_idx];
        let b = frame.pixels[off + b_idx];
        if r > 200 && g < 50 && b < 50 {
            red_top += 1;
        }
    }
    assert_eq!(red_top, 0, "border disabled (alpha=0) → no red pixels");
}
