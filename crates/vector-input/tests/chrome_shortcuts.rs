//! Plan 05-13: chrome shortcut keymap entries.
//!
//! Cmd-N / Cmd-F / Cmd-Shift-P / Cmd-Shift-R must produce
//! `EncodedKey::App(AppShortcut::...)` with ZERO bytes leaking to PTY.
//! Existing Phase-4 Mux shortcuts (Cmd-T/D/W/Shift-D/Shift-]/Shift-[) must
//! continue to return `EncodedKey::Mux(...)` unchanged.

use vector_input::{encode, AppShortcut, EncodedKey, ModState, MuxCommand};
use winit::event::ElementState;
use winit::keyboard::{Key, SmolStr};

fn cmd() -> ModState {
    ModState {
        shift: false,
        alt: false,
        ctrl: false,
        cmd: true,
    }
}

fn cmd_shift() -> ModState {
    ModState {
        shift: true,
        alt: false,
        ctrl: false,
        cmd: true,
    }
}

fn ch(s: &str, mods: ModState) -> Option<EncodedKey> {
    encode(
        &Key::Character(SmolStr::new(s)),
        Some(s),
        ElementState::Pressed,
        mods,
    )
}

#[test]
fn cmd_n_spawns_new_window() {
    assert_eq!(
        ch("n", cmd()),
        Some(EncodedKey::App(AppShortcut::SpawnNewWindow))
    );
}

#[test]
fn cmd_f_toggles_search() {
    assert_eq!(
        ch("f", cmd()),
        Some(EncodedKey::App(AppShortcut::ToggleSearch))
    );
}

#[test]
fn cmd_shift_p_opens_profile_picker() {
    assert_eq!(
        ch("P", cmd_shift()),
        Some(EncodedKey::App(AppShortcut::OpenProfilePicker))
    );
    assert_eq!(
        ch("p", cmd_shift()),
        Some(EncodedKey::App(AppShortcut::OpenProfilePicker))
    );
}

#[test]
fn cmd_shift_r_reloads_config() {
    assert_eq!(
        ch("R", cmd_shift()),
        Some(EncodedKey::App(AppShortcut::ReloadConfig))
    );
    assert_eq!(
        ch("r", cmd_shift()),
        Some(EncodedKey::App(AppShortcut::ReloadConfig))
    );
}

#[test]
fn plain_n_still_goes_to_pty() {
    assert_eq!(
        ch("n", ModState::default()),
        Some(EncodedKey::Pty(b"n".to_vec()))
    );
}

#[test]
fn cmd_t_still_returns_mux_new_tab() {
    assert_eq!(ch("t", cmd()), Some(EncodedKey::Mux(MuxCommand::NewTab)));
}

#[test]
fn cmd_shift_d_still_returns_mux_split_vertical() {
    assert_eq!(
        ch("D", cmd_shift()),
        Some(EncodedKey::Mux(MuxCommand::SplitVertical))
    );
}

// Phase 8 / D-11: Cmd-Shift-T opens Dev Tunnels picker; Cmd-T alone stays NewTab.

#[test]
fn cmd_shift_t_opens_devtunnels_picker_upper() {
    assert_eq!(
        ch("T", cmd_shift()),
        Some(EncodedKey::App(AppShortcut::OpenDevTunnelsPicker))
    );
}

#[test]
fn cmd_shift_t_opens_devtunnels_picker_lower() {
    assert_eq!(
        ch("t", cmd_shift()),
        Some(EncodedKey::App(AppShortcut::OpenDevTunnelsPicker))
    );
}

#[test]
fn cmd_t_alone_still_returns_mux_new_tab_not_devtunnels() {
    // Phase-4 regression guard: Cmd-T (no shift) must keep returning NewTab.
    assert_eq!(ch("t", cmd()), Some(EncodedKey::Mux(MuxCommand::NewTab)));
    assert_eq!(ch("T", cmd()), Some(EncodedKey::Mux(MuxCommand::NewTab)));
}
