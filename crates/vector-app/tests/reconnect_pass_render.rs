//! Plan 09-04 — UI-side unit tests for the ReconnectPass data contract.
//! Pixel-perfect render is a manual UAT item (UI-SPEC §Manual-Only Verifications);
//! these tests lock the constants + text formatter + truncation rules so Plan 09-05
//! can wire the render hook against a stable shape.

use vector_render::{
    format_reconnect_text, RECONNECT_BAR_HEIGHT_PX, RECONNECT_DEBOUNCE_MS, RECONNECT_FADE_IN_MS,
    RECONNECT_FADE_OUT_MS,
};

#[test]
fn reconnect_pass_renders_status_line() {
    assert_eq!(RECONNECT_BAR_HEIGHT_PX, 24);
    assert_eq!(RECONNECT_FADE_IN_MS, 120);
    assert_eq!(RECONNECT_FADE_OUT_MS, 200);
    assert_eq!(RECONNECT_DEBOUNCE_MS, 250);
}

#[test]
fn reconnect_overlay_text_format_default() {
    let s = format_reconnect_text("corp-dev-box-42", 3, 80).unwrap();
    assert_eq!(s, "Reconnecting to corp-dev-box-42\u{2026} (attempt 3)");
}

#[test]
fn reconnect_overlay_text_format_narrow_truncation() {
    let s = format_reconnect_text("corp-dev-box-42-very-long", 3, 30).unwrap();
    assert!(s.starts_with("Reconnecting to "), "got: {s}");
    assert!(s.contains('\u{2026}'), "expected U+2026 ellipsis; got: {s}");
    assert!(s.ends_with(" (attempt 3)"), "got: {s}");
}

#[test]
fn reconnect_overlay_attempt_counter_caps_at_9_plus() {
    assert!(format_reconnect_text("p", 9, 80)
        .unwrap()
        .ends_with("(attempt 9)"));
    assert!(format_reconnect_text("p", 10, 80)
        .unwrap()
        .ends_with("(attempt 9+)"));
    assert!(format_reconnect_text("p", 15, 80)
        .unwrap()
        .ends_with("(attempt 9+)"));
    assert!(format_reconnect_text("p", 9999, 80)
        .unwrap()
        .ends_with("(attempt 9+)"));
}

#[test]
fn reconnect_overlay_text_format_default_no_profile_under_28() {
    let s = format_reconnect_text("anything", 3, 20).unwrap();
    assert_eq!(s, "Reconnecting\u{2026} (attempt 3)");
}

#[test]
fn reconnect_overlay_impossibly_narrow_returns_none() {
    assert_eq!(format_reconnect_text("anything", 3, 12), None);
    assert_eq!(format_reconnect_text("anything", 3, 17), None);
    assert!(format_reconnect_text("anything", 3, 18).is_some());
}

#[test]
#[ignore = "Wave 0 — implemented in Plan 09-05"]
fn input_locked_in_reconnecting_state() {
    unimplemented!()
}

#[test]
#[ignore = "Wave 0 — implemented in Plan 09-05"]
fn tab_badge_during_reconnect() {
    unimplemented!()
}
