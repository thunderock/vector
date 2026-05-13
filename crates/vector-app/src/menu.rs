//! AppKit menu bar install per UI-SPEC §"Menu bar items (Phase 1)".

use std::sync::OnceLock;

use objc2::rc::Retained;
use objc2::runtime::Sel;
use objc2::sel;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem};
use objc2_foundation::NSString;

use vector_config::{ConfigFile, Kind};

/// Newtype that asserts main-thread-only access to a `Retained<NSMenu>`. AppKit
/// objects are `!Sync` by default; access is always gated by `MainThreadMarker`
/// at the call sites below, which makes parking the handle in a `static`
/// `OnceLock` safe in practice. `dispatch2::MainThreadBound` would be the
/// upstream equivalent; we keep the dependency surface tight here.
struct MainThreadOnly<T>(T);
// SAFETY: every consumer below holds a `MainThreadMarker`, so the contained
// `Retained<NSMenu>` is only ever read on the AppKit main thread.
unsafe impl<T> Sync for MainThreadOnly<T> {}
unsafe impl<T> Send for MainThreadOnly<T> {}

/// MEDIUM-4 (05-REVIEWS.md): direct handle to the Switch Profile NSMenu.
/// Populated exactly once by `add_switch_profile_submenu()` at install time;
/// consumed by `rebuild_switch_profile_submenu()`. No NSApplication.mainMenu
/// walk — the original index-0 + title-string approach was fragile.
static SWITCH_PROFILE_SUBMENU: OnceLock<MainThreadOnly<Retained<NSMenu>>> = OnceLock::new();

/// Install the standard AppKit menu bar (UI-SPEC).
///
/// # Safety
/// Caller must invoke this on the macOS main thread; winit's `resumed`
/// callback guarantees that invariant for production callers.
pub unsafe fn install_main_menu() {
    let mtm = MainThreadMarker::new().expect("must be called on main thread");
    let app = NSApplication::sharedApplication(mtm);
    let main_menu = NSMenu::new(mtm);

    let app_item = app_menu(mtm);
    let file_item = file_menu(mtm);
    let edit_item = edit_menu(mtm);
    let view_item = view_menu(mtm);
    let window_item = window_menu(mtm);
    let help_item = help_menu(mtm);

    main_menu.addItem(&app_item);
    main_menu.addItem(&file_item);
    main_menu.addItem(&edit_item);
    main_menu.addItem(&view_item);
    main_menu.addItem(&window_item);
    main_menu.addItem(&help_item);

    if let Some(win_sub) = window_item.submenu() {
        app.setWindowsMenu(Some(&win_sub));
    }
    if let Some(help_sub) = help_item.submenu() {
        app.setHelpMenu(Some(&help_sub));
    }
    app.setMainMenu(Some(&main_menu));
}

fn app_menu(mtm: MainThreadMarker) -> Retained<NSMenuItem> {
    let item = NSMenuItem::new(mtm);
    let submenu = NSMenu::new(mtm);
    submenu.setTitle(&NSString::from_str("Vector"));
    add(
        mtm,
        &submenu,
        "About Vector",
        sel!(orderFrontStandardAboutPanel:),
        "",
    );
    submenu.addItem(&NSMenuItem::separatorItem(mtm));
    add_disabled(mtm, &submenu, "Preferences\u{2026}", ",");
    // Plan 05-10 UI-SPEC §5.8 — Switch Profile submenu. Populated dynamically
    // from the active ConfigFile at first-paint; for now ship a disabled
    // placeholder so the menu-bar surface is discoverable.
    add_switch_profile_submenu(mtm, &submenu);
    // Plan 05-10 D-80 — Secure Keyboard Entry (no shortcut). Key-only so the
    // App's keymap can pump `UserEvent::ToggleSecureKeyboardEntry`.
    add_disabled(mtm, &submenu, "Secure Keyboard Entry", "");
    submenu.addItem(&NSMenuItem::separatorItem(mtm));
    add_services(mtm, &submenu);
    submenu.addItem(&NSMenuItem::separatorItem(mtm));
    add(mtm, &submenu, "Hide Vector", sel!(hide:), "h");
    add_with_modifiers(
        mtm,
        &submenu,
        "Hide Others",
        sel!(hideOtherApplications:),
        "h",
        NSEventModifierFlags::Command | NSEventModifierFlags::Option,
    );
    add(mtm, &submenu, "Show All", sel!(unhideAllApplications:), "");
    submenu.addItem(&NSMenuItem::separatorItem(mtm));
    add(mtm, &submenu, "Quit Vector", sel!(terminate:), "q");
    item.setSubmenu(Some(&submenu));
    item
}

// File menu (UI-SPEC): New Window (Cmd-N, disabled — Phase 5/D-65), New Tab
// (Cmd-T, Plan 04-04 enabled — no AppKit action; winit KeyboardInput sees Cmd-T
// and routes to `MuxCommand::NewTab` which our App handles), separator,
// Close (Cmd-W, performClose:).
fn file_menu(mtm: MainThreadMarker) -> Retained<NSMenuItem> {
    let item = NSMenuItem::new(mtm);
    let submenu = NSMenu::new(mtm);
    submenu.setTitle(&NSString::from_str("File"));
    // Plan 05-10 D-82: New Window enabled via key-only — winit KeyboardInput sees
    // Cmd-N and the App's keymap dispatch routes to `UserEvent::SpawnNewWindow`.
    add_key_only(mtm, &submenu, "New Window", "n");
    // Plan 04-04: "New Tab" enabled (not greyed); key event flows through winit
    // to our keymap which encodes it as `EncodedKey::Mux(MuxCommand::NewTab)`.
    add_key_only(mtm, &submenu, "New Tab", "t");
    submenu.addItem(&NSMenuItem::separatorItem(mtm));
    add(mtm, &submenu, "Close", sel!(performClose:), "w");
    item.setSubmenu(Some(&submenu));
    item
}

// Edit menu (UI-SPEC): Undo/Redo/Cut/Copy/Paste/Select All — ALL disabled in Phase 1.
fn edit_menu(mtm: MainThreadMarker) -> Retained<NSMenuItem> {
    let item = NSMenuItem::new(mtm);
    let submenu = NSMenu::new(mtm);
    submenu.setTitle(&NSString::from_str("Edit"));
    add_disabled(mtm, &submenu, "Undo", "z");
    add_disabled_with_modifiers(
        mtm,
        &submenu,
        "Redo",
        "z",
        NSEventModifierFlags::Command | NSEventModifierFlags::Shift,
    );
    submenu.addItem(&NSMenuItem::separatorItem(mtm));
    add_disabled(mtm, &submenu, "Cut", "x");
    add_disabled(mtm, &submenu, "Copy", "c");
    add_disabled(mtm, &submenu, "Paste", "v");
    add_disabled(mtm, &submenu, "Select All", "a");
    item.setSubmenu(Some(&submenu));
    item
}

// View menu (UI-SPEC): Enter Full Screen — Cmd-Ctrl-F. Plan 05-10 M4: Reload Config — Cmd-Shift-R.
fn view_menu(mtm: MainThreadMarker) -> Retained<NSMenuItem> {
    let item = NSMenuItem::new(mtm);
    let submenu = NSMenu::new(mtm);
    submenu.setTitle(&NSString::from_str("View"));
    add_with_modifiers(
        mtm,
        &submenu,
        "Enter Full Screen",
        sel!(toggleFullScreen:),
        "f",
        NSEventModifierFlags::Command | NSEventModifierFlags::Control,
    );
    // Plan 05-10 M4 / D-69: "Reload Config" — Cmd-Shift-R. Key-only; the App
    // keymap routes the keystroke to `UserEvent::ReloadConfig`.
    add_key_only_with_modifiers(
        mtm,
        &submenu,
        "Reload Config",
        "r",
        NSEventModifierFlags::Command | NSEventModifierFlags::Shift,
    );
    item.setSubmenu(Some(&submenu));
    item
}

// Window menu (UI-SPEC): Minimize (Cmd-M), Zoom, separator, Bring All to Front.
fn window_menu(mtm: MainThreadMarker) -> Retained<NSMenuItem> {
    let item = NSMenuItem::new(mtm);
    let submenu = NSMenu::new(mtm);
    submenu.setTitle(&NSString::from_str("Window"));
    add(mtm, &submenu, "Minimize", sel!(performMiniaturize:), "m");
    add(mtm, &submenu, "Zoom", sel!(performZoom:), "");
    submenu.addItem(&NSMenuItem::separatorItem(mtm));
    add(
        mtm,
        &submenu,
        "Bring All to Front",
        sel!(arrangeInFront:),
        "",
    );
    item.setSubmenu(Some(&submenu));
    item
}

// Help menu (UI-SPEC): Vector Help — disabled in Phase 1.
fn help_menu(mtm: MainThreadMarker) -> Retained<NSMenuItem> {
    let item = NSMenuItem::new(mtm);
    let submenu = NSMenu::new(mtm);
    submenu.setTitle(&NSString::from_str("Help"));
    add_disabled(mtm, &submenu, "Vector Help", "");
    item.setSubmenu(Some(&submenu));
    item
}

// ---- helpers ----------------------------------------------------------

/// Append a functional menu item. Default modifier mask = Cmd.
fn add(mtm: MainThreadMarker, menu: &NSMenu, title: &str, action: Sel, key: &str) {
    let mi = NSMenuItem::new(mtm);
    mi.setTitle(&NSString::from_str(title));
    // SAFETY: AppKit selectors are static; setAction is unsafe only because
    // objc2 cannot prove the receiver implements the target.
    unsafe { mi.setAction(Some(action)) };
    mi.setKeyEquivalent(&NSString::from_str(key));
    menu.addItem(&mi);
}

/// Append a menu entry whose only purpose is to show the key equivalent in the
/// menu — the keystroke flows through to winit because no AppKit action is
/// installed. Used by Plan 04-04 for Cmd-T (App handles it via the keymap).
fn add_key_only(mtm: MainThreadMarker, menu: &NSMenu, title: &str, key: &str) {
    let mi = NSMenuItem::new(mtm);
    mi.setTitle(&NSString::from_str(title));
    mi.setKeyEquivalent(&NSString::from_str(key));
    // Leave the item enabled (default) and no action installed.
    menu.addItem(&mi);
}

/// Append a greyed-out item: no `setAction`, explicitly `setEnabled(false)`.
fn add_disabled(mtm: MainThreadMarker, menu: &NSMenu, title: &str, key: &str) {
    let mi = NSMenuItem::new(mtm);
    mi.setTitle(&NSString::from_str(title));
    mi.setKeyEquivalent(&NSString::from_str(key));
    mi.setEnabled(false);
    menu.addItem(&mi);
}

/// Append a functional item with explicit modifier mask.
fn add_with_modifiers(
    mtm: MainThreadMarker,
    menu: &NSMenu,
    title: &str,
    action: Sel,
    key: &str,
    modifiers: NSEventModifierFlags,
) {
    let mi = NSMenuItem::new(mtm);
    mi.setTitle(&NSString::from_str(title));
    unsafe { mi.setAction(Some(action)) };
    mi.setKeyEquivalent(&NSString::from_str(key));
    mi.setKeyEquivalentModifierMask(modifiers);
    menu.addItem(&mi);
}

/// Append a disabled item with explicit modifier mask.
fn add_disabled_with_modifiers(
    mtm: MainThreadMarker,
    menu: &NSMenu,
    title: &str,
    key: &str,
    modifiers: NSEventModifierFlags,
) {
    let mi = NSMenuItem::new(mtm);
    mi.setTitle(&NSString::from_str(title));
    mi.setKeyEquivalent(&NSString::from_str(key));
    mi.setKeyEquivalentModifierMask(modifiers);
    mi.setEnabled(false);
    menu.addItem(&mi);
}

/// Append a key-only item with explicit modifier mask. Stays enabled but installs
/// no AppKit action, so the keystroke flows through winit to the App's keymap.
fn add_key_only_with_modifiers(
    mtm: MainThreadMarker,
    menu: &NSMenu,
    title: &str,
    key: &str,
    modifiers: NSEventModifierFlags,
) {
    let mi = NSMenuItem::new(mtm);
    mi.setTitle(&NSString::from_str(title));
    mi.setKeyEquivalent(&NSString::from_str(key));
    mi.setKeyEquivalentModifierMask(modifiers);
    menu.addItem(&mi);
}

/// Plan 05-10 UI-SPEC §5.8 / Plan 05-11 (POLISH-07) — install the Switch Profile
/// submenu and capture its NSMenu in `SWITCH_PROFILE_SUBMENU` for later rebuilds
/// (MEDIUM-4). The submenu is left empty at install time; the first
/// `UserEvent::ConfigReloaded` fills it via `rebuild_switch_profile_submenu`.
fn add_switch_profile_submenu(mtm: MainThreadMarker, menu: &NSMenu) {
    let item = NSMenuItem::new(mtm);
    item.setTitle(&NSString::from_str("Switch Profile"));
    let sub = NSMenu::new(mtm);
    sub.setTitle(&NSString::from_str("Switch Profile"));
    item.setSubmenu(Some(&sub));
    menu.addItem(&item);
    // MEDIUM-4: store the submenu reference so rebuild doesn't walk mainMenu.
    // `set` is idempotent — first call wins; reinstalling the menu (rare) is a no-op.
    let _ = SWITCH_PROFILE_SUBMENU.set(MainThreadOnly(sub));
}

/// Plan 05-11 (POLISH-07, UI-SPEC §6.4) — produce the `(label, enabled)` rows
/// that the Switch Profile submenu should display for `cfg`. Local profiles are
/// enabled; Codespace/DevTunnel profiles ship with `(phase 6+)` suffix and are
/// disabled. Rows are in `BTreeMap` (alphabetical) order.
#[must_use]
pub fn submenu_rows_for(cfg: &ConfigFile) -> Vec<(String, bool)> {
    cfg.profile
        .iter()
        .map(|(name, block)| match block.kind {
            Some(Kind::Codespace) | Some(Kind::DevTunnel) => {
                (format!("{name} (phase 6+)"), false)
            }
            // Default (None) and explicit Local are first-class.
            _ => (name.clone(), true),
        })
        .collect()
}

/// Plan 05-11 (POLISH-07, MEDIUM-4) — drain and repopulate the Switch Profile
/// submenu from `cfg`. Looks up the submenu via the `SWITCH_PROFILE_SUBMENU`
/// OnceLock — no NSApplication.mainMenu walk. Caller must be on the main thread.
///
/// # Safety
/// AppKit NSMenu mutation must occur on the macOS main thread; callers pass
/// `MainThreadMarker` to prove that invariant.
pub unsafe fn rebuild_switch_profile_submenu(mtm: MainThreadMarker, cfg: &ConfigFile) {
    let Some(bound) = SWITCH_PROFILE_SUBMENU.get() else {
        tracing::warn!(
            "rebuild_switch_profile_submenu called before add_switch_profile_submenu install"
        );
        return;
    };
    let submenu: &NSMenu = &bound.0;
    // Drain — repeatedly remove index 0 until empty.
    while submenu.numberOfItems() > 0 {
        submenu.removeItemAtIndex(0);
    }
    for (label, enabled) in submenu_rows_for(cfg) {
        if enabled {
            add_key_only(mtm, submenu, &label, "");
        } else {
            add_disabled(mtm, submenu, &label, "");
        }
    }
}

/// Add the "Services" submenu wired to NSApp.setServicesMenu.
fn add_services(mtm: MainThreadMarker, menu: &NSMenu) {
    let item = NSMenuItem::new(mtm);
    item.setTitle(&NSString::from_str("Services"));
    let services_menu = NSMenu::new(mtm);
    services_menu.setTitle(&NSString::from_str("Services"));
    let app = NSApplication::sharedApplication(mtm);
    app.setServicesMenu(Some(&services_menu));
    item.setSubmenu(Some(&services_menu));
    menu.addItem(&item);
}
