//! B1 (Plan 05-10 Task 1) — D-78 OSC 8 Cmd-click dispatch + scheme-reject toast.
//!
//! Two responsibilities:
//! 1. Pure-Rust scheme routing (testable without AppKit): `dispatch_cmd_click(url, toasts)`.
//! 2. AppKit FFI: `open_with_nsworkspace(url)` — invoked by App when DispatchAction::OpenUrl arrives.
//!
//! UI-SPEC §6.1 toast string is LOCKED here. Do NOT paraphrase.

use crate::toast::{ToastBanner, ToastStack};
use vector_term::hyperlink::is_allowed_scheme;

/// UI-SPEC §6.1 EXACT string for disallowed-scheme toast.
pub const DISALLOWED_SCHEME_TOAST: &str = "vector only opens http and https links";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchAction {
    OpenUrl(String),
    None,
}

/// Route a Cmd-click on an OSC 8 cell into an action. Honors D-78 scheme allowlist;
/// pushes the UI-SPEC §6.1 toast on rejection.
pub fn dispatch_cmd_click(url: &str, toasts: &mut ToastStack) -> DispatchAction {
    if is_allowed_scheme(url) {
        DispatchAction::OpenUrl(url.to_owned())
    } else {
        tracing::info!(url = %url, "OSC 8 Cmd-click scheme not in allowlist; toasting");
        toasts.show(ToastBanner::info(DISALLOWED_SCHEME_TOAST));
        DispatchAction::None
    }
}

/// AppKit FFI — `NSWorkspace.sharedWorkspace().open(NSURL.URLWithString(url))`.
#[cfg(not(test))]
pub fn open_with_nsworkspace(url: &str) {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSString, NSURL};
    let ns_url_str = NSString::from_str(url);
    if let Some(ns_url) = NSURL::URLWithString(&ns_url_str) {
        let ws = NSWorkspace::sharedWorkspace();
        let _ = ws.openURL(&ns_url);
        tracing::info!(url = %url, "NSWorkspace.openURL dispatched");
    } else {
        tracing::warn!(url = %url, "NSURL::URLWithString returned nil — invalid URL");
    }
}

#[cfg(test)]
pub fn open_with_nsworkspace(_url: &str) { /* no-op in unit tests */
}
