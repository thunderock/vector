//! Plan 05-12 — ClipboardRouter policy dispatch tests.
//!
//! Validates that ClipboardEvent::Store routes to the correct ClipboardAction
//! depending on the resolved profile policy. The wiring of router.handle into
//! App.user_event is covered indirectly: this test pins the policy semantics
//! so the App arm only needs to match-and-dispatch.

use vector_app::clipboard_router::{ClipboardAction, ClipboardRouter};
use vector_app::toast::ToastMode;
use vector_config::ClipboardPolicy;
use vector_term::{ClipboardEvent, ClipboardType};

fn make_router(policy: Option<ClipboardPolicy>) -> ClipboardRouter {
    ClipboardRouter {
        active_profile: "default".into(),
        policy,
    }
}

#[test]
fn allow_policy_routes_to_write_pasteboard() {
    let router = make_router(Some(ClipboardPolicy::Allow));
    let event = ClipboardEvent::Store(ClipboardType::Clipboard, "payload".into());
    match router.handle(event, "zsh") {
        ClipboardAction::WritePasteboard(s) => assert_eq!(s, "payload"),
        other => panic!("expected WritePasteboard, got {other:?}"),
    }
}

#[test]
fn no_policy_routes_to_action_toast_with_three_buttons() {
    let router = make_router(None);
    let event = ClipboardEvent::Store(ClipboardType::Clipboard, "secret".into());
    match router.handle(event, "zsh") {
        ClipboardAction::ShowPrompt(toast) => match &toast.mode {
            ToastMode::Action { buttons } => assert_eq!(
                buttons,
                &vec![
                    "allow once".to_owned(),
                    "always".to_owned(),
                    "block".to_owned()
                ]
            ),
            other => panic!("expected Action toast, got {other:?}"),
        },
        other => panic!("expected ShowPrompt, got {other:?}"),
    }
}

#[test]
fn block_policy_routes_to_info_toast() {
    let router = make_router(Some(ClipboardPolicy::Block));
    let event = ClipboardEvent::Store(ClipboardType::Clipboard, "payload".into());
    match router.handle(event, "zsh") {
        ClipboardAction::ShowPrompt(toast) => {
            assert!(matches!(toast.mode, ToastMode::Info));
            assert_eq!(toast.text, "clipboard write from default blocked");
        }
        other => panic!("expected ShowPrompt, got {other:?}"),
    }
}
