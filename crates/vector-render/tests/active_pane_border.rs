//! D-66: offscreen pixel snapshot showing 1-px border on viewport edge.
//! Plan 04-04 un-ignores and fills.

#[test]
#[ignore = "Wave-0 stub: Plan 04-04"]
fn border_color_some_renders_one_px_border() {
    // Plan 04-04: offscreen Compositor::new_with(viewport_offset_px=[0,0], size=[800,600])
    // + render_offscreen_with(term, selection=None, border_color=Some([0.4, 0.6, 1.0, 1.0]))
    // -> read pixels along viewport edge -> assert majority of edge-pixels match border_color
    // within tolerance; interior cells are bg-color.
    panic!("Wave-0 stub — implemented by Plan 04-04");
}
