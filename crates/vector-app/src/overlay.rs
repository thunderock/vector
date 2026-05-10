//! NSTextField version overlay anchored to the window's bottom-right.

use std::ffi::c_void;
use std::ptr::NonNull;

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSColor, NSFont, NSFontWeightRegular, NSTextField, NSView,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

const OVERLAY_W: f64 = 260.0;
const OVERLAY_H: f64 = 22.0;
const MARGIN: f64 = 16.0;

pub struct Overlay {
    // Retained so the text field outlives `install`. autoresizingMask handles
    // bottom-right anchoring; `relayout` is a no-op kept for the resize hook.
    _label: Retained<NSTextField>,
}

impl Overlay {
    /// Resize handler. AutoresizingMask keeps the label anchored to
    /// bottom-right — nothing to do manually.
    #[allow(clippy::unused_self)]
    pub fn relayout(&mut self) {}
}

/// SAFETY: must be called on the macOS main thread.
pub unsafe fn install(window: &Window) -> Overlay {
    let mtm = MainThreadMarker::new().expect("must be called on main thread");

    let handle = window.window_handle().expect("window_handle").as_raw();
    let ns_view_ptr: NonNull<c_void> = match handle {
        RawWindowHandle::AppKit(h) => h.ns_view,
        _ => panic!("expected AppKit window handle"),
    };
    let ns_view: &NSView = ns_view_ptr.cast::<NSView>().as_ref();
    let ns_window = ns_view.window().expect("NSView has no window");
    let content_view = ns_window
        .contentView()
        .expect("NSWindow has no contentView");

    let text = format!(
        "Vector v{} (build {})",
        env!("CARGO_PKG_VERSION"),
        env!("VECTOR_BUILD_SHA")
    );
    let label = NSTextField::labelWithString(&NSString::from_str(&text), mtm);

    // 11pt monospaced (SF Mono on macOS 13+).
    let font = NSFont::monospacedSystemFontOfSize_weight(11.0, NSFontWeightRegular);
    label.setFont(Some(&font));

    // #9A9A9A text on #2A2A2A plate.
    let text_color = NSColor::colorWithSRGBRed_green_blue_alpha(0.604, 0.604, 0.604, 1.0);
    label.setTextColor(Some(&text_color));
    label.setBezeled(false);
    label.setBordered(false);
    label.setEditable(false);
    label.setSelectable(false);
    label.setDrawsBackground(false);

    // Position bottom-right with MARGIN px inset from each edge.
    let bounds = content_view.bounds();
    let frame = NSRect::new(
        NSPoint::new(bounds.size.width - OVERLAY_W - MARGIN, MARGIN),
        NSSize::new(OVERLAY_W, OVERLAY_H),
    );
    label.setFrame(frame);
    label.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewMinXMargin | NSAutoresizingMaskOptions::ViewMaxYMargin,
    );

    // Rounded plate background via CALayer.
    label.setWantsLayer(true);
    if let Some(layer) = label.layer() {
        let plate = NSColor::colorWithSRGBRed_green_blue_alpha(0.165, 0.165, 0.165, 1.0);
        layer.setBackgroundColor(Some(&plate.CGColor()));
        layer.setCornerRadius(4.0);
    }

    content_view.addSubview(&label);

    Overlay { _label: label }
}
