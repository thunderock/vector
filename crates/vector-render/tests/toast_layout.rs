//! M2 (Plan 05-10 Task 2) — Toast layout + alpha math (UI-SPEC §5.4).

use vector_render::{alpha_at, toast_layout, ToastModeKind};

#[test]
fn toast_info_height_36() {
    assert_eq!(toast_layout(ToastModeKind::Info).height_px, 36);
}

#[test]
fn toast_action_height_56() {
    assert_eq!(toast_layout(ToastModeKind::Action).height_px, 56);
}

#[test]
fn toast_fade_durations() {
    let l = toast_layout(ToastModeKind::Info);
    assert_eq!(l.fade_in_ms, 120, "UI-SPEC §5.4: 120 ms ease-out fade in");
    assert_eq!(l.fade_out_ms, 200, "UI-SPEC §5.4: 200 ms fade out");
}

#[test]
fn toast_alpha_fades() {
    let total = 5000u32;
    let reduce = false;
    assert!(alpha_at(0, total, reduce) < 0.1);
    assert!((alpha_at(120, total, reduce) - 1.0).abs() < 0.01);
    assert!((alpha_at(total - 200, total, reduce) - 1.0).abs() < 0.01);
    assert!(alpha_at(total, total, reduce) < 0.01);
    assert!((alpha_at(0, total, true) - 1.0).abs() < f32::EPSILON);
    assert!(alpha_at(total, total, true) < f32::EPSILON);
}
