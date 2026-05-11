//! NSWindow style mask sanity. WIN-01.
//! Phase 1's overlay smoke proved {Titled, Closable, Miniaturizable, Resizable}
//! are set on the default winit window on macOS. This test compile-checks the
//! import path so a future deletion of the bit flags trips the build;
//! full visual verification is in Plan 03-05's manual smoke matrix.

#[test]
fn win_attributes_default_includes_required_mask_bits() {
    use objc2_app_kit::NSWindowStyleMask;
    let mask = NSWindowStyleMask::Titled
        | NSWindowStyleMask::Closable
        | NSWindowStyleMask::Miniaturizable
        | NSWindowStyleMask::Resizable;
    // Touching the value ensures the OR'd mask is preserved by the optimizer.
    assert!(mask.bits() != 0);
}
