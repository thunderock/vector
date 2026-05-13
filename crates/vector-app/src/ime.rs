//! Plan 05-15: declare_class! NSTextInputClient subclass implemented in `appkit_impl` submodule.
//!
//! Five-selector minimum (D-81): setMarkedText, insertText, unmarkText,
//! hasMarkedText, markedRange. No candidate window in v1 — full IME with
//! native candidate UI is v2 TERM-V2-01.
//!
//! Pitfall 9: preedit text MUST NEVER enter the PTY byte stream. Only
//! `insertText:` (mapped to [`ImeState::commit`]) writes to the channel.

use objc2_foundation::NSRange;
use tokio::sync::mpsc;

/// Pure-Rust IME state. Decoupled from AppKit so unit tests can drive the
/// state machine without an NSView. The `declare_class!` wrapper lives in the
/// `appkit_impl` submodule (cfg-gated off macOS / tests).
pub struct ImeState {
    preedit: String,
    selected_offset: usize,
    active: bool,
    pub write_tx: mpsc::Sender<Vec<u8>>,
}

impl ImeState {
    pub fn new(write_tx: mpsc::Sender<Vec<u8>>) -> Self {
        Self {
            preedit: String::new(),
            selected_offset: 0,
            active: false,
            write_tx,
        }
    }

    /// `setMarkedText:` handler — preedit display only.
    ///
    /// Pitfall 9: NEVER writes to PTY. The preedit string is held until either
    /// `commit` (insertText) flushes it or `clear` (unmarkText) drops it.
    pub fn set_preedit(&mut self, text: &str, selected_offset: usize) {
        text.clone_into(&mut self.preedit);
        self.selected_offset = selected_offset;
        self.active = !text.is_empty();
    }

    /// `insertText:` handler — commit. Writes the UTF-8 bytes to PTY and clears
    /// the preedit buffer.
    pub fn commit(&mut self, text: &str) -> bool {
        let sent = self.write_tx.try_send(text.as_bytes().to_vec()).is_ok();
        self.preedit.clear();
        self.active = false;
        self.selected_offset = 0;
        sent
    }

    /// `unmarkText` handler — drop preedit without committing.
    pub fn clear(&mut self) {
        self.preedit.clear();
        self.active = false;
        self.selected_offset = 0;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn preedit(&self) -> &str {
        &self.preedit
    }

    pub fn selected_offset(&self) -> usize {
        self.selected_offset
    }

    /// `markedRange` handler. Returns `{NSNotFound, 0}` when inactive (signaled
    /// here via `location = usize::MAX`).
    pub fn marked_range(&self) -> NSRange {
        if self.active {
            NSRange {
                location: 0,
                length: self.preedit.chars().count(),
            }
        } else {
            NSRange {
                location: usize::MAX,
                length: 0,
            }
        }
    }
}

// Plan 05-15: NSTextInputClient subclass via objc2 0.6.4 `define_class!` macro.
// Gated behind cfg(all(target_os = "macos", not(feature = "test-hooks"))) so
// that `cargo test` (no AppKit runtime) can still drive the pure-Rust ImeState
// layer through the data-only methods above.

#[cfg(all(target_os = "macos", not(feature = "test-hooks")))]
pub mod appkit_impl {
    use super::ImeState;
    // NSAttributedString required: AppKit may deliver attributed strings to
    // insertText: and setMarkedText:; we extract the plain string via `-string`
    // which both NSString and NSAttributedString respond to (HIGH-1 import).
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly};
    use objc2_app_kit::NSView;
    use objc2_foundation::{NSAttributedString, NSRange, NSString};
    use parking_lot::Mutex;

    // HIGH-1 Send+Sync compile-time guarantee: define_class! Ivars must be
    // Send+Sync. ImeState fields: String, usize, bool, mpsc::Sender<Vec<u8>>
    // — all Send. Mutex<T>: Send+Sync when T: Send.
    const _: fn() = || {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Mutex<ImeState>>();
        // Ensure NSAttributedString import is used (documents the runtime
        // reason; coercion is via AnyObject). HIGH-1 reviewer requirement.
        let _: fn(*const NSAttributedString) = |_| {};
    };

    define_class!(
        // SAFETY: NSView subclassing requirements are upheld; we do not implement
        // Drop and do not retain self across selector bodies.
        #[unsafe(super(NSView))]
        #[name = "VectorInputView"]
        #[ivars = Mutex<ImeState>]
        pub struct VectorInputView;

        impl VectorInputView {
            // 1. insertText:replacementRange: — D-81 commit path.
            // text may be NSString or NSAttributedString; both respond to -string.
            #[unsafe(method(insertText:replacementRange:))]
            fn insert_text(&self, text: &AnyObject, _range: NSRange) {
                // Cast via downcast_ref: try NSString first, fall back to NSAttributedString.
                let s = if let Some(ns_str) = text.downcast_ref::<NSString>() {
                    ns_str.to_string()
                } else if let Some(attr_str) = text.downcast_ref::<NSAttributedString>() {
                    attr_str.string().to_string()
                } else {
                    // Fallback: send -string message to the raw object.
                    let ns_str: Retained<NSString> = unsafe { msg_send![text, string] };
                    ns_str.to_string()
                };
                let _ = self.ivars().lock().commit(&s);
            }

            // 2. setMarkedText:selectedRange:replacementRange: — D-81 preedit path.
            // Pitfall 9: NEVER writes to PTY.
            #[unsafe(method(setMarkedText:selectedRange:replacementRange:))]
            fn set_marked_text(
                &self,
                text: &AnyObject,
                selected: NSRange,
                _replacement: NSRange,
            ) {
                let s = if let Some(ns_str) = text.downcast_ref::<NSString>() {
                    ns_str.to_string()
                } else if let Some(attr_str) = text.downcast_ref::<NSAttributedString>() {
                    attr_str.string().to_string()
                } else {
                    let ns_str: Retained<NSString> = unsafe { msg_send![text, string] };
                    ns_str.to_string()
                };
                self.ivars().lock().set_preedit(&s, selected.location);
            }

            // 3. unmarkText
            #[unsafe(method(unmarkText))]
            fn unmark_text(&self) {
                self.ivars().lock().clear();
            }

            // 4. markedRange
            #[unsafe(method(markedRange))]
            fn marked_range(&self) -> NSRange {
                self.ivars().lock().marked_range()
            }

            // 5. selectedRange — return NSNotFound (no in-document selection in v1)
            #[unsafe(method(selectedRange))]
            fn selected_range(&self) -> NSRange {
                NSRange { location: usize::MAX, length: 0 }
            }

            // hasMarkedText — return YES when preedit is active.
            #[unsafe(method(hasMarkedText))]
            fn has_marked_text(&self) -> bool {
                self.ivars().lock().is_active()
            }
        }
    );

    impl VectorInputView {
        /// Construct a new `VectorInputView` on the main thread, wrapping `state`.
        pub fn new_with_state(
            mtm: objc2::MainThreadMarker,
            state: ImeState,
        ) -> Retained<Self> {
            let this = Self::alloc(mtm).set_ivars(Mutex::new(state));
            unsafe { msg_send![super(this), init] }
        }
    }
}

#[cfg(all(target_os = "macos", not(feature = "test-hooks")))]
pub use appkit_impl::VectorInputView;
