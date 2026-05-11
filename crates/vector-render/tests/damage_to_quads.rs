//! Feed a red 'A', offscreen render, assert red-dominant pixels appear in the top-row strip.

#[path = "common/offscreen.rs"]
mod offscreen;

use offscreen::channel_indices;

#[test]
#[allow(clippy::many_single_char_names)]
fn red_a_cell_paints_red_pixels() {
    let Some((mut comp, ctx)) = offscreen::build_compositor(300, 150) else {
        return;
    };
    let mut term = vector_term::Term::new(20, 6, 100);
    term.feed(b"\x1b[31mA\x1b[0m");
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
    let stride = 4 * frame.width;
    let mut red_dominant = 0u32;
    let y_start = 0u32;
    let y_end = ch.min(frame.height);
    let x_start = 0u32;
    let x_end = (cw * 4).min(frame.width);
    for y in y_start..y_end {
        for x in x_start..x_end {
            let off = (y * stride + x * 4) as usize;
            let r = frame.pixels[off + r_idx];
            let g = frame.pixels[off + g_idx];
            let b = frame.pixels[off + b_idx];
            if r > 150 && g < 80 && b < 80 {
                red_dominant += 1;
            }
        }
    }
    assert!(
        red_dominant > 20,
        "expected ≥20 red-dominant pixels for red 'A', got {red_dominant}",
    );
}
