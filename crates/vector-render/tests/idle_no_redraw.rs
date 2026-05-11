//! Render-on-dirty: empty drain → no redraw (RENDER-03 / D-44).
//! Mirror of the request_redraw gate in vector-app.

fn should_redraw(empty_drain: bool, input_event: bool) -> bool {
    !empty_drain || input_event
}

#[test]
fn empty_drain_without_input_no_redraw() {
    assert!(!should_redraw(true, false));
}

#[test]
fn empty_drain_with_input_redraws() {
    assert!(should_redraw(true, true));
}

#[test]
fn non_empty_drain_redraws() {
    assert!(should_redraw(false, false));
}
