//! Mux command dispatch + Cmd-T window-creation helper. Plan 04-04 (D-56/59/60/61/62).
//!
//! `create_tabbed_winit_window` is the single call site that pairs
//! `event_loop.create_window(attrs)` with
//! `winit::platform::macos::WindowExtMacOS::set_tabbing_identifier`. The
//! identifier ensures AppKit groups the new NSWindow into the existing tab
//! group (D-56). Test-time mocking goes through [`WindowFactory`] so
//! `multi_window_tabbing.rs` can assert the call without spinning up a winit
//! event loop.

use std::sync::Arc;

use anyhow::Result;
use vector_input::MuxCommand;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

/// Stable identifier used by AppKit's `NSWindowTabbingMode.preferred` to group
/// all of Vector's tab-bearing NSWindows.
pub const VECTOR_TABBING_IDENTIFIER: &str = "com.vector.terminal";

/// Trait-routed factory for creating + grouping a winit Window. Production
/// callers use [`WinitWindowFactory`]; tests provide a mock to assert the
/// `set_tabbing_identifier` call without an event loop.
pub trait WindowFactory {
    fn create_tabbed(
        &self,
        attrs: WindowAttributes,
        tabbing_identifier: &str,
    ) -> Result<Arc<Window>>;
}

/// Production factory: drives `event_loop.create_window` + the AppKit tab grouping.
pub struct WinitWindowFactory<'a> {
    pub event_loop: &'a ActiveEventLoop,
}

impl WindowFactory for WinitWindowFactory<'_> {
    fn create_tabbed(
        &self,
        attrs: WindowAttributes,
        tabbing_identifier: &str,
    ) -> Result<Arc<Window>> {
        let win = self.event_loop.create_window(attrs)?;
        apply_tabbing_identifier(&win, tabbing_identifier);
        Ok(Arc::new(win))
    }
}

impl WinitWindowFactory<'_> {
    /// D-82 Cmd-N: create a fresh winit Window OUTSIDE any tab group.
    /// MEDIUM-1 (05-REVIEWS.md): implemented via setTabbingMode:NSWindowTabbingModeDisallowed on
    /// the underlying NSWindow — deterministic, no uuid dep, no counter.
    pub fn create_ungrouped(
        &self,
        attrs: WindowAttributes,
    ) -> Result<Arc<Window>, winit::error::OsError> {
        let win = self.event_loop.create_window(attrs)?;
        // SAFETY: window_event/SpawnNewWindow runs on main thread per winit contract.
        unsafe {
            apply_tabbing_mode_disallowed(&win);
        }
        Ok(Arc::new(win))
    }
}

/// Apply the AppKit tabbing identifier on a freshly-created winit Window. Plan
/// 04-04 uses both winit's `WindowExtMacOS::set_tabbing_identifier` and an
/// explicit objc2-app-kit `setTabbingMode(NSWindowTabbingModePreferred)` call
/// to mitigate winit#2238 (single-window may launch as a separate NSWindow
/// instead of joining the tab group on first invocation).
#[cfg(target_os = "macos")]
fn apply_tabbing_identifier(win: &Window, identifier: &str) {
    use winit::platform::macos::WindowExtMacOS;
    win.set_tabbing_identifier(identifier);
    // Belt-and-braces (D-56 / winit#2238): explicitly set tabbing mode to
    // Preferred via objc2-app-kit. Best-effort — if the AppKit handle is
    // unavailable we keep the winit-applied identifier as the sole signal.
    set_tabbing_mode_preferred(win);
}

#[cfg(not(target_os = "macos"))]
fn apply_tabbing_identifier(_win: &Window, _identifier: &str) {
    // Non-mac targets have no tab group concept.
}

/// MEDIUM-1 (05-REVIEWS.md): set NSWindowTabbingModeDisallowed so AppKit will
/// NEVER group this window into another window's tab bar.
/// Modeled on the existing `set_tabbing_mode_preferred` helper.
/// SAFETY: must be called on the main thread; caller verifies via winit event-loop contract.
#[cfg(target_os = "macos")]
unsafe fn apply_tabbing_mode_disallowed(win: &Window) {
    use std::ffi::c_void;
    use std::ptr::NonNull;

    use objc2_app_kit::{NSView, NSWindowTabbingMode};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = win.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(h) = handle.as_raw() else {
        return;
    };
    let ns_view_ptr: NonNull<c_void> = h.ns_view;
    let ns_view = ns_view_ptr.cast::<NSView>().as_ref();
    let Some(ns_window) = ns_view.window() else {
        return;
    };
    // NSWindowTabbingModeDisallowed = 2 — prevents AppKit from grouping this window.
    ns_window.setTabbingMode(NSWindowTabbingMode::Disallowed);
}

#[cfg(not(target_os = "macos"))]
unsafe fn apply_tabbing_mode_disallowed(_win: &Window) {}

#[cfg(target_os = "macos")]
fn set_tabbing_mode_preferred(win: &Window) {
    use std::ffi::c_void;
    use std::ptr::NonNull;

    use objc2_app_kit::{NSView, NSWindowTabbingMode};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = win.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(h) = handle.as_raw() else {
        return;
    };
    let ns_view_ptr: NonNull<c_void> = h.ns_view;
    // SAFETY: AppKit window handle from winit; pointer is non-null and points to NSView.
    let ns_view = unsafe { ns_view_ptr.cast::<NSView>().as_ref() };
    let Some(ns_window) = ns_view.window() else {
        return;
    };
    ns_window.setTabbingMode(NSWindowTabbingMode::Preferred);
}

/// Mux command dispatch (Plan 04-04 placeholder). The full path lands in
/// `app.rs` after this helper resolves the active TabWindow via WindowId.
///
/// Plan 04-04 ships the routing seam; Plan 04-05 polish ties in the per-pane
/// Compositor map + close cascade UI side-effects.
pub fn describe(cmd: MuxCommand) -> &'static str {
    match cmd {
        MuxCommand::NewTab => "Cmd-T (new tab)",
        MuxCommand::SplitHorizontal => "Cmd-D (split horizontal)",
        MuxCommand::SplitVertical => "Cmd-Shift-D (split vertical)",
        MuxCommand::ClosePane => "Cmd-W (close pane)",
        MuxCommand::CycleTabNext => "Cmd-Shift-] (next tab)",
        MuxCommand::CycleTabPrev => "Cmd-Shift-[ (prev tab)",
        MuxCommand::FocusDir(_) => "Cmd-Opt-Arrow (focus direction)",
        MuxCommand::NudgeSplit(_) => "Cmd-Shift-Arrow (nudge split)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Mock factory used by `multi_window_tabbing.rs` to assert the tabbing
    /// identifier call without an event loop.
    pub struct MockFactory {
        pub calls: RefCell<Vec<String>>,
    }

    impl MockFactory {
        pub fn new() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl WindowFactory for MockFactory {
        fn create_tabbed(
            &self,
            _attrs: WindowAttributes,
            tabbing_identifier: &str,
        ) -> Result<Arc<Window>> {
            self.calls.borrow_mut().push(tabbing_identifier.to_string());
            // Mock fails the actual window creation — tests assert on `calls`.
            Err(anyhow::anyhow!("mock: no real window"))
        }
    }

    #[test]
    fn mock_factory_records_tabbing_identifier() {
        let factory = MockFactory::new();
        let _ = factory.create_tabbed(WindowAttributes::default(), VECTOR_TABBING_IDENTIFIER);
        assert_eq!(
            factory.calls.borrow().as_slice(),
            &[VECTOR_TABBING_IDENTIFIER.to_string()]
        );
    }

    #[test]
    fn describe_maps_each_variant() {
        use vector_mux::Direction;
        for cmd in [
            MuxCommand::NewTab,
            MuxCommand::SplitHorizontal,
            MuxCommand::SplitVertical,
            MuxCommand::ClosePane,
            MuxCommand::CycleTabNext,
            MuxCommand::CycleTabPrev,
            MuxCommand::FocusDir(Direction::Left),
            MuxCommand::NudgeSplit(Direction::Right),
        ] {
            assert!(!describe(cmd).is_empty());
        }
    }
}
