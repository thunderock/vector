//! Plan 03-04 Task 2: selection tint lights up the selected cells. Tracks RENDER-05.
//!
//! Builds an offscreen compositor over an empty Term, renders with selection=Some(((0,0),(3,0))),
//! reads back pixels, asserts the blue channel in the selected region is meaningfully higher
//! than in an adjacent unselected region — the selection_tint is xterm-ish blue (D-54).

#[path = "common/offscreen.rs"]
mod offscreen;

use offscreen::channel_indices;

#[test]
fn selection_tint_visible_in_selected_cells() {
    let Some((mut comp, ctx)) = offscreen::build_compositor(300, 150) else {
        // No Metal adapter (e.g. Linux dev shell) — skip cleanly.
        return;
    };
    let mut term = vector_term::Term::new(20, 6, 100);
    // Selection covers columns 0..=3 on row 0.
    let frame = comp
        .render_offscreen_with(
            &ctx.device,
            &ctx.queue,
            ctx.width,
            ctx.height,
            &mut term,
            Some(((0, 0), (3, 0))),
        )
        .expect("render_offscreen_with");
    let (r_idx, _, b_idx) = channel_indices(frame.format);
    let cw = comp.cell_width_px();
    let ch = comp.cell_height_px();
    let stride = 4 * frame.width;

    // Sample the center of cells (1, 0) — selected — and (10, 0) — unselected.
    let sel_x = cw + cw / 2;
    let sel_y = ch / 2;
    let unsel_x = 10 * cw + cw / 2;
    let unsel_y = ch / 2;
    let sel_off = (sel_y * stride + sel_x * 4) as usize;
    let unsel_off = (unsel_y * stride + unsel_x * 4) as usize;
    let sel_b = frame.pixels[sel_off + b_idx];
    let unsel_b = frame.pixels[unsel_off + b_idx];
    let sel_r = frame.pixels[sel_off + r_idx];

    // The tint is blue-dominant (selection_tint = [0.27, 0.48, 0.78, 0.40]); the selected pixel's
    // blue channel must be noticeably higher than the unselected default-bg pixel's blue channel.
    assert!(
        sel_b > unsel_b + 20,
        "expected selected cell blue ({sel_b}) > unselected ({unsel_b}) + 20",
    );
    // And blue should dominate red in the selected cell.
    assert!(
        sel_b > sel_r,
        "expected selected cell blue ({sel_b}) > red ({sel_r}) due to selection tint",
    );
}
