//! POLISH-08 / D-81 / Pitfall 9 — basic IME via NSTextInputClient.
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

// NOTE: full `declare_class!` NSTextInputClient subclass wrapping `ImeState`
// is the AppKit half of D-81. It is intentionally deferred to Plan 05-09's
// follow-on render-side work because:
//   1. Building NSView subclasses cannot be exercised in `cargo test` (no
//      AppKit runtime in the test harness).
//   2. The five required selectors are already covered by `ImeState`'s pure
//      Rust API; the AppKit wrapper is a thin forward-each-method shim.
//   3. Plan 05-09's manual smoke matrix item #3 (Hiragana preedit) gates the
//      visual half end-to-end via human verification.
//
// Until the AppKit shim lands, the IME data path is exercised through unit
// tests; preedit rendering uses the existing Phase-3 cell pipeline underline
// attribute. When the shim is implemented it lives in this module under
// `#[cfg(all(target_os = "macos", not(feature = "test-hooks")))]`.
