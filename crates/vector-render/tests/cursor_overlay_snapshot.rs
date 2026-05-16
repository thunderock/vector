//! Cursor at (0,0) on a clean grid paints a light block in cell 0.

#[path = "common/offscreen.rs"]
mod offscreen;

use offscreen::channel_indices;

#[test]
#[allow(clippy::many_single_char_names)]
fn cursor_paints_light_block_in_cursor_cell() {
    let Some((mut comp, ctx)) = offscreen::build_compositor(300, 150) else {
        return;
    };
    let mut term = vector_term::Term::new(20, 6, 100);
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
    let cw = comp.cell_width_px();
    let ch = comp.cell_height_px();
    let x = (cw / 2).min(frame.width - 1);
    let y = (ch / 2).min(frame.height - 1);
    let stride = 4 * frame.width;
    let off = (y * stride + x * 4) as usize;
    let r = frame.pixels[off + r_idx];
    let g = frame.pixels[off + g_idx];
    let b = frame.pixels[off + b_idx];
    assert!(
        r > 150 && g > 150 && b > 150,
        "cursor cell center should be near light-gray, got ({r},{g},{b})",
    );
}
