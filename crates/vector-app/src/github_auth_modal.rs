//! Phase 8 / Plan 08-05 Task 2 / UI-SPEC §S2 — GitHub Device-Flow modal.
//!
//! Mirrors `crates/vector-app/src/auth_modal.rs` shape against GitHub-specific
//! labels per UI-SPEC §Copywriting Contract / Modal copy. 480 × 280 px.
//!
//! Buttons fire via an ObjC responder class that captures the
//! `tokio_util::sync::CancellationToken` from `GitHubDeviceFlowStarted`.
//! Pitfall 14: only `user_code` (8-char pairing code) ever reaches this module —
//! NEVER the access/refresh token.

use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSBackingStoreType, NSBezelStyle, NSButton, NSColor, NSFont, NSPanel, NSPasteboard,
    NSPasteboardTypeString, NSTextAlignment, NSTextField, NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString, NSURL};
use tokio_util::sync::CancellationToken;

pub use responder::GitHubAuthModalResponder;

/// UI-SPEC §Spacing Scale — modal frame size.
pub const PANEL_W: f64 = 480.0;
pub const PANEL_H: f64 = 280.0;

/// Modal context handed in by App at construction.
pub struct GitHubAuthModalCtx {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: Duration,
    pub cancel: CancellationToken,
}

pub struct GitHubAuthDeviceFlowModal {
    panel: Retained<NSPanel>,
    countdown_field: Retained<NSTextField>,
    started_at: Instant,
    expires_in: Duration,
    user_code: String,
    verification_uri: String,
    cancel: CancellationToken,
    _responder: Retained<GitHubAuthModalResponder>,
}

impl GitHubAuthDeviceFlowModal {
    pub fn show(mtm: MainThreadMarker, ctx: GitHubAuthModalCtx) -> Self {
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(PANEL_W, PANEL_H));
        let style = NSWindowStyleMask::Titled | NSWindowStyleMask::Closable;
        let panel: Retained<NSPanel> = NSPanel::initWithContentRect_styleMask_backing_defer(
            mtm.alloc::<NSPanel>(),
            frame,
            style,
            NSBackingStoreType::Buffered,
            false,
        );
        panel.setTitle(&NSString::from_str("Sign in with GitHub"));
        panel.setLevel(objc2_app_kit::NSFloatingWindowLevel);
        panel.center();

        let responder = GitHubAuthModalResponder::new(
            mtm,
            ctx.user_code.clone(),
            ctx.verification_uri.clone(),
            ctx.cancel.clone(),
        );

        let prompt_text = format!(
            "Open {} in your browser and enter this code:",
            ctx.verification_uri
        );
        let prompt = make_label(
            mtm,
            &prompt_text,
            NSRect::new(NSPoint::new(20.0, 220.0), NSSize::new(440.0, 24.0)),
            14.0,
            false,
        );
        let code_field = make_code_field(
            mtm,
            &ctx.user_code,
            NSRect::new(NSPoint::new(60.0, 140.0), NSSize::new(360.0, 60.0)),
        );
        let countdown_field = make_label(
            mtm,
            "Expires in --:--",
            NSRect::new(NSPoint::new(20.0, 100.0), NSSize::new(440.0, 22.0)),
            11.0,
            true,
        );
        let secondary_btn = make_button(
            mtm,
            "Cancel sign-in",
            NSRect::new(NSPoint::new(40.0, 20.0), NSSize::new(150.0, 28.0)),
        );

        unsafe {
            use objc2::sel;
            let responder_obj: &objc2::runtime::AnyObject = (*responder).as_ref();
            secondary_btn.setTarget(Some(responder_obj));
            secondary_btn.setAction(Some(sel!(cancelClicked:)));
        }

        {
            let content = panel.contentView().expect("content view");
            content.addSubview(&prompt);
            content.addSubview(&code_field);
            content.addSubview(&countdown_field);
            content.addSubview(&secondary_btn);
            panel.makeKeyAndOrderFront(None);
        }

        // Best-effort: open the verification URI for the user.
        open_url(&ctx.verification_uri);
        // Best-effort: copy user code so they can paste it on the GitHub page.
        write_clipboard(&ctx.user_code);

        Self {
            panel,
            countdown_field,
            started_at: Instant::now(),
            expires_in: ctx.expires_in,
            user_code: ctx.user_code,
            verification_uri: ctx.verification_uri,
            cancel: ctx.cancel,
            _responder: responder,
        }
    }

    /// 1 Hz tick. Returns `true` if the device code expired.
    pub fn tick(&self) -> bool {
        let elapsed = self.started_at.elapsed();
        if elapsed >= self.expires_in {
            return true;
        }
        let remaining = self.expires_in - elapsed;
        let secs = remaining.as_secs();
        let min = secs / 60;
        let sec = secs % 60;
        let text = format!("Expires in {min}:{sec:02}");
        self.countdown_field
            .setStringValue(&NSString::from_str(&text));
        false
    }

    pub fn dismiss(&self) {
        self.cancel.cancel();
        self.panel.orderOut(None);
    }

    #[must_use]
    pub fn user_code(&self) -> &str {
        &self.user_code
    }

    #[must_use]
    pub fn verification_uri(&self) -> &str {
        &self.verification_uri
    }
}

fn make_label(
    mtm: MainThreadMarker,
    text: &str,
    frame: NSRect,
    font_size: f64,
    muted: bool,
) -> Retained<NSTextField> {
    let f = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    f.setFrame(frame);
    f.setBezeled(false);
    f.setEditable(false);
    f.setBackgroundColor(Some(&NSColor::clearColor()));
    f.setAlignment(NSTextAlignment::Center);
    let font = NSFont::systemFontOfSize(font_size);
    f.setFont(Some(&font));
    if muted {
        f.setTextColor(Some(&NSColor::secondaryLabelColor()));
    }
    f
}

fn make_code_field(mtm: MainThreadMarker, code: &str, frame: NSRect) -> Retained<NSTextField> {
    let f = NSTextField::labelWithString(&NSString::from_str(code), mtm);
    f.setFrame(frame);
    f.setBezeled(true);
    f.setEditable(false);
    f.setSelectable(true);
    f.setAlignment(NSTextAlignment::Center);
    // 32pt mono semibold (UI-SPEC §Typography: Display row).
    let font = NSFont::monospacedSystemFontOfSize_weight(32.0, 0.6_f64);
    f.setFont(Some(&font));
    f
}

fn make_button(mtm: MainThreadMarker, title: &str, frame: NSRect) -> Retained<NSButton> {
    let b = unsafe {
        NSButton::buttonWithTitle_target_action(&NSString::from_str(title), None, None, mtm)
    };
    b.setFrame(frame);
    b.setBezelStyle(NSBezelStyle::Push);
    b
}

fn open_url(url: &str) {
    let ns_url = NSURL::URLWithString(&NSString::from_str(url));
    if let Some(u) = ns_url {
        let workspace = NSWorkspace::sharedWorkspace();
        let _ = workspace.openURL(&u);
    }
}

fn write_clipboard(value: &str) {
    let pb = NSPasteboard::generalPasteboard();
    pb.clearContents();
    unsafe {
        pb.setString_forType(&NSString::from_str(value), NSPasteboardTypeString);
    }
}

mod responder {
    use objc2::define_class;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{msg_send, DefinedClass, MainThreadOnly};
    use objc2_app_kit::NSResponder;
    use parking_lot::Mutex;
    use tokio_util::sync::CancellationToken;

    pub struct Ivars {
        #[allow(dead_code)]
        user_code: String,
        #[allow(dead_code)]
        verification_uri: String,
        cancel: CancellationToken,
    }

    const _: fn() = || {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Mutex<Ivars>>();
    };

    define_class!(
        #[unsafe(super(NSResponder))]
        #[name = "VectorGitHubAuthModalResponder"]
        #[ivars = Mutex<Ivars>]
        pub struct GitHubAuthModalResponder;

        impl GitHubAuthModalResponder {
            #[unsafe(method(cancelClicked:))]
            fn cancel_clicked(&self, _sender: &AnyObject) {
                let g = self.ivars().lock();
                g.cancel.cancel();
            }
        }
    );

    impl GitHubAuthModalResponder {
        pub fn new(
            mtm: objc2::MainThreadMarker,
            user_code: String,
            verification_uri: String,
            cancel: CancellationToken,
        ) -> Retained<Self> {
            let ivars = Ivars {
                user_code,
                verification_uri,
                cancel,
            };
            let this = Self::alloc(mtm).set_ivars(Mutex::new(ivars));
            unsafe { msg_send![super(this), init] }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_frame_constants_lock_480x280() {
        assert!((PANEL_W - 480.0).abs() < f64::EPSILON);
        assert!((PANEL_H - 280.0).abs() < f64::EPSILON);
    }
}
