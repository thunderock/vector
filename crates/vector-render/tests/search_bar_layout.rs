//! M1 (Plan 05-10 Task 2) — SearchBarPass layout geometry tests (UI-SPEC §5.2).

#![allow(clippy::float_cmp)]

use vector_render::{search_bar_layout, SEARCH_BAR_HEIGHT_PX};

#[test]
fn search_bar_geometry() {
    let layout = search_bar_layout(1200, false);
    assert_eq!(layout.height_px, SEARCH_BAR_HEIGHT_PX);
    assert_eq!(layout.height_px, 32, "UI-SPEC §5.2 — 32 px");
    let sum = layout.query_field.w
        + layout.smart_case_indicator.w
        + layout.prev_arrow.w
        + layout.next_arrow.w
        + layout.counter.w
        + layout.close_btn.w;
    assert!(sum > 0.0 && sum < 1200.0);
    assert!((layout.smart_case_indicator.w - 24.0).abs() < f32::EPSILON);
    assert!((layout.prev_arrow.w - 24.0).abs() < f32::EPSILON);
    assert!((layout.next_arrow.w - 24.0).abs() < f32::EPSILON);
    assert!((layout.close_btn.w - 24.0).abs() < f32::EPSILON);
}

#[test]
fn search_bar_no_match_tint() {
    let normal = search_bar_layout(1200, false);
    let tinted = search_bar_layout(1200, true);
    assert_ne!(
        normal.bg_rgba, tinted.bg_rgba,
        "M1 + UI-SPEC §5.2: no_match must tint bg toward color.warning α 0.20"
    );
    assert!(tinted.bg_rgba[0] > normal.bg_rgba[0]);
}
