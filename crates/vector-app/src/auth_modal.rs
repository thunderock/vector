//! Phase 6 / AUTH-01 / UI-SPEC §5.1 — Device-Flow modal NSPanel.
//!
//! 440 × 280 px, Titled+Closable (Pitfall 3: never modalPanel — that strands
//! key-window status). NSFloatingWindowLevel so it stays above the main
//! NSWindow but does not steal Cmd-Q. Clipboard is captured on mount and
//! restored on dismiss (Pitfall 7).
//!
//! Buttons are wired to ObjC actions via an `AuthModalResponder` class that
//! captures a `EventLoopProxy<UserEvent>` in its ivars. Primary click does
//! its own side-effect inline (re-copy code + open URL); Cancel sends a
//! `UserEvent::AuthFailed { reason: "cancelled" }` so the App's user_event
//! handler can dismiss the modal and restore the clipboard centrally.
//!
//! Pitfall 14: the OAuth token NEVER reaches this module. Only the 8-char
//! user-code (device-pairing, not account-bearing) is rendered.

use std::time::SystemTime;

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSBackingStoreType, NSBezelStyle, NSButton, NSColor, NSFont, NSPanel, NSPasteboard,
    NSPasteboardTypeString, NSTextAlignment, NSTextField, NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString, NSURL};
use winit::event_loop::EventLoopProxy;

use crate::auth_actor::AuthCancellation;
use crate::UserEvent;

pub use responder::AuthModalResponder;

/// Owns the NSPanel + countdown updater + saved clipboard. Held by App until
/// dismissed; `Drop` is implicit because Retained<NSPanel> releases on drop
/// (the panel itself disappears once `orderOut` runs in `dismiss`).
pub struct AuthDeviceFlowModal {
    panel: Retained<NSPanel>,
    countdown_field: Retained<NSTextField>,
    saved_clipboard: Option<String>,
    expires_at: SystemTime,
    user_code: String,
    verification_uri: String,
    cancellation: AuthCancellation,
    // Keep the responder alive for the lifetime of the modal — NSButton holds
    // a weak target reference, so dropping the Retained early would null
    // out the action.
    _responder: Retained<AuthModalResponder>,
}

impl AuthDeviceFlowModal {
    pub fn show(
        mtm: MainThreadMarker,
        user_code: String,
        verification_uri: String,
        expires_at: SystemTime,
        cancellation: AuthCancellation,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Self {
        let saved_clipboard = capture_clipboard();
        write_clipboard(&user_code);

        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(440.0, 280.0));
        let style = NSWindowStyleMask::Titled | NSWindowStyleMask::Closable;
        let panel: Retained<NSPanel> = NSPanel::initWithContentRect_styleMask_backing_defer(
            mtm.alloc::<NSPanel>(),
            frame,
            style,
            NSBackingStoreType::Buffered,
            false,
        );
        panel.setTitle(&NSString::from_str("Sign in to GitHub"));
        // NSFloatingWindowLevel = 3 (objc2_app_kit::NSFloatingWindowLevel).
        // NSWindowLevel is an `NSInteger` type alias, not a newtype.
        panel.setLevel(objc2_app_kit::NSFloatingWindowLevel);
        panel.center();

        // Build the responder before the buttons so we can wire targets/actions.
        let responder = AuthModalResponder::new(
            mtm,
            user_code.clone(),
            verification_uri.clone(),
            cancellation.clone(),
            proxy,
        );

        let prompt = make_label(
            mtm,
            "Enter this code at github.com/device:",
            NSRect::new(NSPoint::new(20.0, 220.0), NSSize::new(400.0, 24.0)),
            13.0,
            false,
        );
        let code_field = make_code_field(
            mtm,
            &user_code,
            NSRect::new(NSPoint::new(60.0, 150.0), NSSize::new(320.0, 60.0)),
        );
        let countdown_field = make_label(
            mtm,
            "Code copied to clipboard. Expires in --:--.",
            NSRect::new(NSPoint::new(20.0, 110.0), NSSize::new(400.0, 22.0)),
            11.0,
            true,
        );
        let primary_btn = make_button(
            mtm,
            "Copy code and open github.com/device",
            NSRect::new(NSPoint::new(40.0, 60.0), NSSize::new(360.0, 36.0)),
        );
        let secondary_btn = make_button(
            mtm,
            "Cancel sign-in",
            NSRect::new(NSPoint::new(40.0, 20.0), NSSize::new(150.0, 28.0)),
        );

        // Wire the ObjC target/action plumbing.
        unsafe {
            use objc2::sel;
            let responder_obj: &objc2::runtime::AnyObject = (*responder).as_ref();
            primary_btn.setTarget(Some(responder_obj));
            primary_btn.setAction(Some(sel!(primaryClicked:)));
            secondary_btn.setTarget(Some(responder_obj));
            secondary_btn.setAction(Some(sel!(cancelClicked:)));
        }

        {
            let content = panel.contentView().expect("content view");
            content.addSubview(&prompt);
            content.addSubview(&code_field);
            content.addSubview(&countdown_field);
            content.addSubview(&primary_btn);
            content.addSubview(&secondary_btn);
            panel.makeKeyAndOrderFront(None);
        }

        AuthDeviceFlowModal {
            panel,
            countdown_field,
            saved_clipboard,
            expires_at,
            user_code,
            verification_uri,
            cancellation,
            _responder: responder,
        }
    }

    /// 1 Hz tick from the frame-tick loop. Returns `true` if the modal should
    /// auto-close because the device code expired (UI-SPEC §5.1 last row).
    pub fn tick(&self) -> bool {
        let now = SystemTime::now();
        let remaining = self
            .expires_at
            .duration_since(now)
            .unwrap_or_default()
            .as_secs();
        if remaining == 0 {
            return true;
        }
        let min = remaining / 60;
        let sec = remaining % 60;
        let text = format!("Code copied to clipboard. Expires in {min:02}:{sec:02}.");
        self.countdown_field
            .setStringValue(&NSString::from_str(&text));
        false
    }

    /// Cancel the in-flight oauth poll + dismiss the modal. Idempotent; safe
    /// to call from either the Cancel button or the AuthFailed handler.
    pub fn cancel(&self, mtm: MainThreadMarker) {
        self.cancellation.cancel();
        self.dismiss(mtm);
    }

    /// Dismiss: restore clipboard + orderOut. Called by every terminal path
    /// (success, cancel, expired). The `MainThreadMarker` argument is a
    /// compile-time witness that AppKit mutation is safe here.
    pub fn dismiss(&self, _: MainThreadMarker) {
        if let Some(prev) = &self.saved_clipboard {
            write_clipboard(prev);
        } else {
            clear_clipboard();
        }
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
    // semibold = 0.6 on the AppKit font-weight scale (-1.0 ultraLight … 1.0 black).
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

fn capture_clipboard() -> Option<String> {
    let pb = NSPasteboard::generalPasteboard();
    unsafe {
        pb.stringForType(NSPasteboardTypeString)
            .map(|s| s.to_string())
    }
}

fn write_clipboard(value: &str) {
    let pb = NSPasteboard::generalPasteboard();
    pb.clearContents();
    unsafe {
        pb.setString_forType(&NSString::from_str(value), NSPasteboardTypeString);
    }
}

fn clear_clipboard() {
    let pb = NSPasteboard::generalPasteboard();
    pb.clearContents();
}

fn open_url(url: &str) {
    let ns_url = NSURL::URLWithString(&NSString::from_str(url));
    if let Some(u) = ns_url {
        let workspace = NSWorkspace::sharedWorkspace();
        let _ = workspace.openURL(&u);
    }
}

// ObjC trampoline for the two AppKit button actions. Held inside the modal so
// it stays alive as long as the buttons reference it.
mod responder {
    use objc2::define_class;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{msg_send, DefinedClass, MainThreadOnly};
    use objc2_app_kit::NSResponder;
    use parking_lot::Mutex;
    use winit::event_loop::EventLoopProxy;

    use crate::auth_actor::AuthCancellation;
    use crate::UserEvent;

    use super::{open_url, write_clipboard};

    pub struct Ivars {
        user_code: String,
        verification_uri: String,
        cancellation: AuthCancellation,
        proxy: EventLoopProxy<UserEvent>,
    }

    // Send+Sync witness — required by define_class!.
    const _: fn() = || {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Mutex<Ivars>>();
    };

    define_class!(
        // SAFETY: NSResponder is the AppKit base class for objects participating
        // in the responder chain. We do not override any of its methods.
        #[unsafe(super(NSResponder))]
        #[name = "VectorAuthModalResponder"]
        #[ivars = Mutex<Ivars>]
        pub struct AuthModalResponder;

        impl AuthModalResponder {
            // UI-SPEC §5.1 primary: re-copy + open URL on every click.
            #[unsafe(method(primaryClicked:))]
            fn primary_clicked(&self, _sender: &AnyObject) {
                let g = self.ivars().lock();
                write_clipboard(&g.user_code);
                open_url(&g.verification_uri);
            }

            // UI-SPEC §5.1 secondary: cancel poll + ask App to dismiss + toast.
            #[unsafe(method(cancelClicked:))]
            fn cancel_clicked(&self, _sender: &AnyObject) {
                let g = self.ivars().lock();
                g.cancellation.cancel();
                let _ = g.proxy.send_event(UserEvent::AuthFailed {
                    reason: "cancelled".into(),
                });
            }
        }
    );

    impl AuthModalResponder {
        pub fn new(
            mtm: objc2::MainThreadMarker,
            user_code: String,
            verification_uri: String,
            cancellation: AuthCancellation,
            proxy: EventLoopProxy<UserEvent>,
        ) -> Retained<Self> {
            let ivars = Ivars {
                user_code,
                verification_uri,
                cancellation,
                proxy,
            };
            let this = Self::alloc(mtm).set_ivars(Mutex::new(ivars));
            unsafe { msg_send![super(this), init] }
        }
    }
}
