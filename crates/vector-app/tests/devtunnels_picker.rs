//! Phase 8 / Plan 08-05 Task 2 — DevTunnels picker pure-Rust helpers.
//!
//! Cross-checks UI-SPEC §Picker footer copy, §Row copy template, §Spacing Scale
//! constants. AppKit modal surface is covered by Task 2's manual smoke matrix.

use vector_app::devtunnels_actor::TunnelView;
use vector_app::devtunnels_modal::{
    footer_copy, format_row, status_dot, FooterState, StatusColor, FOOTER_FONT_SIZE, FOOTER_H,
    FOOTER_W, FOOTER_X, FOOTER_Y, PANEL_H, PANEL_W, ROWS_H, ROWS_W, ROWS_X, ROWS_Y, ROW_FONT_SIZE,
    ROW_HEIGHT,
};

#[test]
fn panel_frame_lock_640x480() {
    assert!((PANEL_W - 640.0).abs() < f64::EPSILON);
    assert!((PANEL_H - 480.0).abs() < f64::EPSILON);
    assert!((ROWS_X - 8.0).abs() < f64::EPSILON);
    assert!((ROWS_Y - 32.0).abs() < f64::EPSILON);
    assert!((ROWS_W - 624.0).abs() < f64::EPSILON);
    assert!((ROWS_H - 416.0).abs() < f64::EPSILON);
    assert!((FOOTER_X - 8.0).abs() < f64::EPSILON);
    assert!((FOOTER_Y - 4.0).abs() < f64::EPSILON);
    assert!((FOOTER_W - 624.0).abs() < f64::EPSILON);
    assert!((FOOTER_H - 24.0).abs() < f64::EPSILON);
    assert!((ROW_HEIGHT - 22.0).abs() < f64::EPSILON);
    assert!((ROW_FONT_SIZE - 13.0).abs() < f64::EPSILON);
    assert!((FOOTER_FONT_SIZE - 11.0).abs() < f64::EPSILON);
}

// UI-SPEC §Picker footer copy table — verbatim, character-for-character.
const LOADING: &str = "Loading Dev Tunnels\u{2026}"; // … = U+2026
const EMPTY_SIGNED_IN: &str =
    "No Vector-agent tunnels yet. Install vector-tunnel-agent on a remote machine and run it.";
const NOT_SIGNED_IN: &str = "Sign in with GitHub or Microsoft to list Dev Tunnels.";

#[test]
fn footer_copy_loading_matches_ui_spec_verbatim() {
    assert_eq!(footer_copy(&FooterState::Loading), LOADING);
}

#[test]
fn footer_copy_empty_signed_in_matches_ui_spec_verbatim() {
    assert_eq!(footer_copy(&FooterState::EmptySignedIn), EMPTY_SIGNED_IN);
}

#[test]
fn footer_copy_not_signed_in_matches_ui_spec_verbatim() {
    assert_eq!(footer_copy(&FooterState::NotSignedIn), NOT_SIGNED_IN);
}

#[test]
fn footer_copy_api_error_press_r_to_retry() {
    let s = footer_copy(&FooterState::ApiError {
        reason: "503".into(),
    });
    assert!(s.contains("Press R to retry"));
    assert_eq!(s, "Could not load tunnels: 503. Press R to retry.");
}

#[test]
fn footer_copy_loaded_n_of_m() {
    assert_eq!(
        footer_copy(&FooterState::Loaded { shown: 3, total: 5 }),
        "3 of 5 tunnels."
    );
}

#[test]
fn footer_copy_signed_in_other_provider() {
    assert_eq!(
        footer_copy(&FooterState::SignedInOtherProvider {
            provider: "Microsoft".into(),
        }),
        "No tunnels under your Microsoft account. Switch providers or register one."
    );
}

#[test]
fn row_format_template_and_no_vector_prefix() {
    let v = TunnelView {
        tunnel_id: "tid1".into(),
        display_name: "alpha".into(),
        host: "alpha.example.com".into(),
        last_seen_secs_ago: Some(45),
    };
    let line = format_row(&v);
    assert!(line.starts_with('\u{25CF}'));
    assert!(line.contains("alpha"));
    assert!(line.contains("alpha.example.com"));
    assert!(line.contains(" · "));
    assert!(!line.contains("vector-"));
}

#[test]
fn row_format_handles_unknown_last_seen() {
    let v = TunnelView {
        tunnel_id: "tid2".into(),
        display_name: "beta".into(),
        host: "beta.example.com".into(),
        last_seen_secs_ago: None,
    };
    let line = format_row(&v);
    assert!(line.contains("never"));
}

#[test]
fn status_dot_buckets_align_with_color_categories() {
    assert_eq!(status_dot(Some(0)).1, StatusColor::Live);
    assert_eq!(status_dot(Some(4 * 60)).1, StatusColor::Live);
    assert_eq!(status_dot(Some(5 * 60)).1, StatusColor::Stale);
    assert_eq!(status_dot(Some(60 * 60)).1, StatusColor::Stale);
    assert_eq!(status_dot(Some(48 * 60 * 60)).1, StatusColor::Unreachable);
    assert_eq!(status_dot(None).1, StatusColor::Unreachable);
}
