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
fn input_locked_in_reconnecting_state() {
    // Plan 09-05: the App-level keystroke gate consults `reconnecting_panes`.
    // Test the extracted helper directly so we don't have to stand up a winit
    // event loop / Mux singleton just to assert this branch.
    use std::collections::HashMap;
    use vector_app::app::{pane_input_locked, ReconnectingState};
    use vector_mux::PaneId;

    let pane = PaneId(42);
    let other = PaneId(99);
    let mut map: HashMap<PaneId, ReconnectingState> = HashMap::new();
    // Not reconnecting: input flows.
    assert!(!pane_input_locked(&map, pane));
    // Insert pane into reconnecting state: input must drop.
    map.insert(
        pane,
        ReconnectingState {
            profile_label: "corp-dev-box-42".to_string(),
            attempt: 1,
            started_at: std::time::Instant::now(),
            fade_in_started_at: None,
        },
    );
    assert!(pane_input_locked(&map, pane));
    // Unrelated pane is unaffected (per-pane lock).
    assert!(!pane_input_locked(&map, other));
    // After PaneReconnected the entry is removed; lock releases.
    map.remove(&pane);
    assert!(!pane_input_locked(&map, pane));
}

#[test]
fn tab_badge_during_reconnect() {
    // UI-SPEC §S4 — tab title transitions [remote] → [reconnecting] → [remote].
    use vector_mux::{format_tab_title, PaneUiState, TransportKind};

    // Active: badge is `[remote]`.
    let active = format_tab_title("zsh", None, TransportKind::DevTunnel, PaneUiState::Active);
    assert!(active.ends_with(" [remote]"), "got: {active}");
    assert!(!active.contains("[reconnecting]"), "got: {active}");

    // Reconnecting: badge flips to `[reconnecting]`.
    let reconn = format_tab_title(
        "zsh",
        None,
        TransportKind::DevTunnel,
        PaneUiState::Reconnecting,
    );
    assert!(reconn.ends_with(" [reconnecting]"), "got: {reconn}");
    assert!(!reconn.contains("[remote]"), "got: {reconn}");

    // Local panes never flip — they cannot reconnect.
    let local_reconn =
        format_tab_title("zsh", None, TransportKind::Local, PaneUiState::Reconnecting);
    assert!(
        !local_reconn.contains("[reconnecting]"),
        "got: {local_reconn}"
    );
    assert!(!local_reconn.contains("[remote]"), "got: {local_reconn}");
}
