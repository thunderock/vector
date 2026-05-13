//! xterm-compatible key encoder. D-52: full xterm key table coverage.
//! D-59/D-60/D-61/D-62: Cmd-* mux shortcuts return `EncodedKey::Mux(...)` and
//! are recognized at the keymap layer BEFORE the xterm key table.

use vector_mux::Direction;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, NamedKey};

use crate::mods::ModState;

/// Output of [`encode`] / [`encode_key`].
///
/// `Pty(bytes)` → App routes to `router.send_write(active_pane, bytes)`.
/// `Mux(cmd)` → App dispatches to the mux command handler; never reaches PTY.
/// `App(shortcut)` → Plan 05-13: chrome shortcut (Cmd-N/F/Shift-P/Shift-R);
/// consumed by Plan 05-14 in vector-app's `encode_key` match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodedKey {
    Pty(Vec<u8>),
    Mux(MuxCommand),
    App(AppShortcut),
}

/// App-layer mux command produced by Cmd-* shortcuts (D-59/D-60/D-61/D-62).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuxCommand {
    NewTab,
    SplitHorizontal,
    SplitVertical,
    ClosePane,
    CycleTabNext,
    CycleTabPrev,
    FocusDir(Direction),
    NudgeSplit(Direction),
}

/// Chrome shortcut produced by Cmd-* keys that target the app shell, not the PTY.
/// Plan 05-13 — Plan 05-14 wires the App-side handlers (D-69/D-75/D-76/D-82).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppShortcut {
    SpawnNewWindow,    // Cmd-N       -> UserEvent::SpawnNewWindow      (D-82)
    ToggleSearch,      // Cmd-F       -> UserEvent::ToggleSearch        (D-76)
    OpenProfilePicker, // Cmd-Shift-P -> UserEvent::OpenProfilePicker   (D-75)
    ReloadConfig,      // Cmd-Shift-R -> UserEvent::ReloadConfig        (D-69 menu fallback)
}

/// Encode a winit key event. Returns None for Released/Dead/Unidentified or unmapped keys.
///
/// Delegates to [`encode`] for the parts actually used. `KeyEvent` has a private
/// `platform_specific` field so it cannot be constructed in tests — call `encode` directly
/// from unit tests, and `encode_key` from the live `WindowEvent::KeyboardInput` handler.
#[must_use]
pub fn encode_key(ev: &KeyEvent, mods: ModState) -> Option<EncodedKey> {
    encode(&ev.logical_key, ev.text.as_deref(), ev.state, mods)
}

/// Test-friendly core. Takes the fields `encode_key` reads off `KeyEvent`.
#[must_use]
pub fn encode(
    logical_key: &Key,
    text: Option<&str>,
    state: ElementState,
    mods: ModState,
) -> Option<EncodedKey> {
    if state != ElementState::Pressed {
        return None;
    }

    // Cmd-* mux shortcuts (D-59/D-60/D-61/D-62) — recognized BEFORE xterm table.
    // Precedence: Cmd-Opt-Arrow → FocusDir; Cmd-Shift-Arrow → NudgeSplit;
    // Cmd-T/D/W/Shift-D/Shift-]/Shift-[ → tab/split/close commands.
    if let Some(cmd) = match_mux_command(logical_key, mods) {
        return Some(EncodedKey::Mux(cmd));
    }

    // Plan 05-13: chrome shortcuts (Cmd-N/F/Shift-P/Shift-R) — after Mux, before PTY.
    if let Some(app) = match_app_shortcut(logical_key, mods) {
        return Some(EncodedKey::App(app));
    }

    encode_pty(logical_key, text, mods).map(EncodedKey::Pty)
}

/// Recognize the four chrome shortcuts. Returns None if the key isn't a chrome binding.
/// Precedence: called AFTER `match_mux_command` so Cmd-T/D/W shortcuts still win.
fn match_app_shortcut(key: &Key, mods: ModState) -> Option<AppShortcut> {
    if !mods.cmd || mods.ctrl || mods.alt {
        return None;
    }
    let s = match key {
        Key::Character(s) => s.as_str(),
        _ => return None,
    };
    if mods.shift {
        return match s {
            "P" | "p" => Some(AppShortcut::OpenProfilePicker),
            "R" | "r" => Some(AppShortcut::ReloadConfig),
            _ => None,
        };
    }
    match s {
        "n" | "N" => Some(AppShortcut::SpawnNewWindow),
        "f" | "F" => Some(AppShortcut::ToggleSearch),
        _ => None,
    }
}

/// Recognize the 14 Cmd-* mux shortcuts. Returns None if the key isn't a mux binding.
fn match_mux_command(key: &Key, mods: ModState) -> Option<MuxCommand> {
    // Arrow keys: Cmd+Opt → FocusDir; Cmd+Shift → NudgeSplit. Reject if Ctrl held.
    if mods.cmd && !mods.ctrl {
        if mods.alt && !mods.shift {
            return match key {
                Key::Named(NamedKey::ArrowLeft) => Some(MuxCommand::FocusDir(Direction::Left)),
                Key::Named(NamedKey::ArrowRight) => Some(MuxCommand::FocusDir(Direction::Right)),
                Key::Named(NamedKey::ArrowUp) => Some(MuxCommand::FocusDir(Direction::Up)),
                Key::Named(NamedKey::ArrowDown) => Some(MuxCommand::FocusDir(Direction::Down)),
                _ => character_shortcut(key, mods),
            };
        }
        if mods.shift && !mods.alt {
            return match key {
                Key::Named(NamedKey::ArrowLeft) => Some(MuxCommand::NudgeSplit(Direction::Left)),
                Key::Named(NamedKey::ArrowRight) => Some(MuxCommand::NudgeSplit(Direction::Right)),
                Key::Named(NamedKey::ArrowUp) => Some(MuxCommand::NudgeSplit(Direction::Up)),
                Key::Named(NamedKey::ArrowDown) => Some(MuxCommand::NudgeSplit(Direction::Down)),
                _ => character_shortcut(key, mods),
            };
        }
        return character_shortcut(key, mods);
    }
    None
}

/// Cmd-only and Cmd-Shift character shortcuts. macOS sends the shifted glyph
/// in `Key::Character` when Shift is held (`"D"`, `"}"`, `"{"`).
fn character_shortcut(key: &Key, mods: ModState) -> Option<MuxCommand> {
    let s = match key {
        Key::Character(s) => s.as_str(),
        _ => return None,
    };
    if mods.alt || mods.ctrl {
        return None;
    }
    if mods.shift {
        // Cmd-Shift-D / Cmd-Shift-] / Cmd-Shift-[. Accept both shifted and unshifted forms.
        return match s {
            "D" | "d" => Some(MuxCommand::SplitVertical),
            "]" | "}" => Some(MuxCommand::CycleTabNext),
            "[" | "{" => Some(MuxCommand::CycleTabPrev),
            _ => None,
        };
    }
    // Cmd-T / Cmd-D / Cmd-W (no shift).
    match s {
        "t" | "T" => Some(MuxCommand::NewTab),
        "d" | "D" => Some(MuxCommand::SplitHorizontal),
        "w" | "W" => Some(MuxCommand::ClosePane),
        _ => None,
    }
}

/// Encode the PTY-bound bytes for a key. Returns None for Released/Dead/Unidentified or
/// unmapped keys. The Cmd-* mux shortcuts above never reach this function.
fn encode_pty(logical_key: &Key, text: Option<&str>, mods: ModState) -> Option<Vec<u8>> {
    let mod_param = mods.xterm_mod_param();

    // Option (Alt) + Character: ESC + bytes. macOS default (D-52).
    // Arrows + nav with Alt use the CSI mod_param form below, not this shortcut.
    if mods.alt {
        if let Key::Character(s) = logical_key {
            let mut out = Vec::with_capacity(1 + s.len());
            out.push(0x1B);
            out.extend_from_slice(s.as_bytes());
            return Some(out);
        }
    }

    // Ctrl + ASCII letter: byte 0x01..=0x1A. Ctrl-Space: NUL.
    if mods.ctrl {
        if let Key::Character(s) = logical_key {
            if let Some(c) = s.chars().next() {
                if c.is_ascii_alphabetic() {
                    return Some(vec![(c.to_ascii_uppercase() as u8) - b'A' + 1]);
                }
            }
        }
        if let Key::Named(NamedKey::Space) = logical_key {
            return Some(vec![0x00]);
        }
    }

    match logical_key {
        Key::Named(NamedKey::Escape) => Some(vec![0x1B]),
        Key::Named(NamedKey::Enter) => Some(vec![0x0D]),
        Key::Named(NamedKey::Tab) if mods.shift => Some(b"\x1b[Z".to_vec()),
        Key::Named(NamedKey::Tab) => Some(vec![0x09]),
        Key::Named(NamedKey::Backspace) => Some(vec![0x7F]),
        Key::Named(NamedKey::Space) => Some(vec![0x20]),
        Key::Named(NamedKey::ArrowUp) => Some(csi_arrow(mod_param, b'A')),
        Key::Named(NamedKey::ArrowDown) => Some(csi_arrow(mod_param, b'B')),
        Key::Named(NamedKey::ArrowRight) => Some(csi_arrow(mod_param, b'C')),
        Key::Named(NamedKey::ArrowLeft) => Some(csi_arrow(mod_param, b'D')),
        Key::Named(NamedKey::Home) => Some(csi_arrow(mod_param, b'H')),
        Key::Named(NamedKey::End) => Some(csi_arrow(mod_param, b'F')),
        Key::Named(NamedKey::PageUp) => Some(csi_tilde(mod_param, b"5")),
        Key::Named(NamedKey::PageDown) => Some(csi_tilde(mod_param, b"6")),
        Key::Named(NamedKey::Insert) => Some(csi_tilde(mod_param, b"2")),
        Key::Named(NamedKey::Delete) => Some(csi_tilde(mod_param, b"3")),
        Key::Named(NamedKey::F1) => Some(ss3_fkey(mod_param, b'P')),
        Key::Named(NamedKey::F2) => Some(ss3_fkey(mod_param, b'Q')),
        Key::Named(NamedKey::F3) => Some(ss3_fkey(mod_param, b'R')),
        Key::Named(NamedKey::F4) => Some(ss3_fkey(mod_param, b'S')),
        Key::Named(NamedKey::F5) => Some(csi_tilde(mod_param, b"15")),
        Key::Named(NamedKey::F6) => Some(csi_tilde(mod_param, b"17")),
        Key::Named(NamedKey::F7) => Some(csi_tilde(mod_param, b"18")),
        Key::Named(NamedKey::F8) => Some(csi_tilde(mod_param, b"19")),
        Key::Named(NamedKey::F9) => Some(csi_tilde(mod_param, b"20")),
        Key::Named(NamedKey::F10) => Some(csi_tilde(mod_param, b"21")),
        Key::Named(NamedKey::F11) => Some(csi_tilde(mod_param, b"23")),
        Key::Named(NamedKey::F12) => Some(csi_tilde(mod_param, b"24")),
        Key::Character(_) => text.map(|t| t.as_bytes().to_vec()),
        _ => None,
    }
}

/// `ESC [ A` no-mod, `ESC [ 1 ; mod A` with mods. final_byte = `b'A'..b'D' | b'H' | b'F'`.
fn csi_arrow(mod_param: u8, final_byte: u8) -> Vec<u8> {
    if mod_param == 1 {
        vec![0x1B, b'[', final_byte]
    } else {
        vec![0x1B, b'[', b'1', b';', b'0' + mod_param, final_byte]
    }
}

/// `ESC [ N ~` no-mod, `ESC [ N ; mod ~` with mods.
fn csi_tilde(mod_param: u8, n: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(n.len() + 5);
    out.push(0x1B);
    out.push(b'[');
    out.extend_from_slice(n);
    if mod_param != 1 {
        out.push(b';');
        out.push(b'0' + mod_param);
    }
    out.push(b'~');
    out
}

/// `ESC O X` no-mod, `ESC [ 1 ; mod X` with mods. X = `b'P'..b'S'` for F1..F4.
fn ss3_fkey(mod_param: u8, final_byte: u8) -> Vec<u8> {
    if mod_param == 1 {
        vec![0x1B, b'O', final_byte]
    } else {
        vec![0x1B, b'[', b'1', b';', b'0' + mod_param, final_byte]
    }
}
