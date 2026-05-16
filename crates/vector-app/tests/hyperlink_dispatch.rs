//! B1 (Plan 05-10 Task 1) — D-78 OSC 8 Cmd-click dispatch + UI-SPEC §6.1 toast.

use vector_app::hyperlink_dispatch::{dispatch_cmd_click, DispatchAction, DISALLOWED_SCHEME_TOAST};
use vector_app::toast::ToastStack;

#[test]
fn cmd_click_allowed_scheme_opens() {
    let mut toasts = ToastStack::default();
    let action = dispatch_cmd_click("https://example.com", &mut toasts);
    assert_eq!(
        action,
        DispatchAction::OpenUrl("https://example.com".to_owned())
    );
    assert!(toasts.current().is_none(), "no toast for allowed scheme");
}

#[test]
fn cmd_click_disallowed_scheme_toasts() {
    let mut toasts = ToastStack::default();
    let action = dispatch_cmd_click("javascript:alert(1)", &mut toasts);
    assert_eq!(action, DispatchAction::None);
    let t = toasts
        .current()
        .expect("toast must be pushed on disallowed scheme");
    assert_eq!(
        t.text, DISALLOWED_SCHEME_TOAST,
        "B1 + M5: UI-SPEC §6.1 EXACT string required; got {:?}",
        t.text
    );
}

#[test]
fn gopher_scheme_rejected_with_toast() {
    let mut toasts = ToastStack::default();
    let action = dispatch_cmd_click("gopher://example.com", &mut toasts);
    assert_eq!(action, DispatchAction::None);
    assert_eq!(
        toasts.current().unwrap().text,
        "vector only opens http and https links"
    );
}

#[test]
fn file_scheme_allowed() {
    let mut toasts = ToastStack::default();
    let action = dispatch_cmd_click("file:///etc/hosts", &mut toasts);
    assert!(matches!(action, DispatchAction::OpenUrl(_)));
}

#[test]
fn mailto_scheme_allowed() {
    let mut toasts = ToastStack::default();
    let action = dispatch_cmd_click("mailto:user@host.com", &mut toasts);
    assert!(matches!(action, DispatchAction::OpenUrl(_)));
}
