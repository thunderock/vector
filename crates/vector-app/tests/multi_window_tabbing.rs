//! D-56: set_tabbing_identifier invoked on every Cmd-T window.
//! Plan 04-04 un-ignores and fills.

#[test]
#[ignore = "Wave-0 stub: Plan 04-04"]
fn set_tabbing_identifier_called_on_cmd_t() {
    // Plan 04-04: mock or trait-route winit::Window::set_tabbing_identifier;
    // assert the App's Cmd-T handler invokes set_tabbing_identifier(&"com.vector.terminal")
    // on the newly-created window. Visual NSWindowTabbingMode behavior is manual-only
    // (smoke matrix #1).
    panic!("Wave-0 stub — implemented by Plan 04-04");
}
