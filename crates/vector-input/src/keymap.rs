//! xterm-compatible key encoder. D-52: full xterm key table coverage.

use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, NamedKey};

use crate::mods::ModState;

/// Encode a winit key event into xterm-compatible bytes.
/// Returns None for Released/Dead/Unidentified or unmapped keys.
///
/// Delegates to [`encode`] for the parts actually used. `KeyEvent` has a private
/// `platform_specific` field so it cannot be constructed in tests — call `encode` directly
/// from unit tests, and `encode_key` from the live `WindowEvent::KeyboardInput` handler.
#[must_use]
pub fn encode_key(ev: &KeyEvent, mods: ModState) -> Option<Vec<u8>> {
    encode(&ev.logical_key, ev.text.as_deref(), ev.state, mods)
}

/// Test-friendly core. Takes the fields `encode_key` reads off `KeyEvent`.
#[must_use]
pub fn encode(
    logical_key: &Key,
    text: Option<&str>,
    state: ElementState,
    mods: ModState,
) -> Option<Vec<u8>> {
    if state != ElementState::Pressed {
        return None;
    }
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
