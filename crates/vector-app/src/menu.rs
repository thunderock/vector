//! AppKit menu bar install per UI-SPEC §"Menu bar items (Phase 1)".

use objc2::rc::Retained;
use objc2::runtime::Sel;
use objc2::sel;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem};
use objc2_foundation::NSString;

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
    add_disabled(mtm, &submenu, "New Window", "n");
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

// View menu (UI-SPEC): Enter Full Screen — Cmd-Ctrl-F.
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
