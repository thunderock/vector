//! POLISH-05 / D-70 — clipboard router.
//! Consumes ClipboardEvent::Store from Plan 05-05's ForwardingListener;
//! per-origin policy decides write-through-to-NSPasteboard vs prompt-via-toast.
//!
//! Empirical (Plan 05-06): alacritty 0.26 base64-DECODES the OSC 52 payload
//! before dispatching `Event::ClipboardStore`, so `data` is plaintext.

use crate::toast::ToastBanner;
use vector_config::ClipboardPolicy;
use vector_term::ClipboardEvent;

pub struct ClipboardRouter {
    pub active_profile: String,
    /// Resolved per-profile from config; `None` = ask the user.
    pub policy: Option<ClipboardPolicy>,
}

/// Caller-side action requested by the router.
#[derive(Debug, Clone)]
pub enum ClipboardAction {
    WritePasteboard(String),
    ShowPrompt(ToastBanner),
    DenyRead,
}

/// Plan 05-12: keep `alacritty_terminal::term::ClipboardType` out of `app.rs`
/// by translating the App-side `bool` discriminator into the real enum here.
#[must_use]
pub fn make_store_event(kind_is_selection: bool, data: String) -> ClipboardEvent {
    use vector_term::ClipboardType;
    ClipboardEvent::Store(
        if kind_is_selection {
            ClipboardType::Selection
        } else {
            ClipboardType::Clipboard
        },
        data,
    )
}

impl ClipboardRouter {
    #[must_use]
    pub fn handle(&self, event: ClipboardEvent, foreground_process: &str) -> ClipboardAction {
        match event {
            ClipboardEvent::Store(_kind, data) => match self.policy {
                Some(ClipboardPolicy::Allow) => ClipboardAction::WritePasteboard(data),
                Some(ClipboardPolicy::Block) => ClipboardAction::ShowPrompt(ToastBanner::info(
                    format!("clipboard write from {} blocked", self.active_profile),
                )),
                None => ClipboardAction::ShowPrompt(ToastBanner::action(
                    format!(
                        "allow \"{} : {}\" to write to your clipboard?",
                        self.active_profile, foreground_process
                    ),
                    vec![
                        "allow once".to_owned(),
                        "always".to_owned(),
                        "block".to_owned(),
                    ],
                )),
            },
            ClipboardEvent::LoadDenied => ClipboardAction::DenyRead,
        }
    }
}
