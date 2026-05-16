//! Plan 03-04 Task 1: xterm key table coverage (D-52). ≥ 80 cases.
//! Plan 04-04 Task 1: 14 Cmd-* Mux shortcuts (D-59/D-60/D-61/D-62).
//!
//! winit 0.30's `KeyEvent` has a private `platform_specific` field — tests must call
//! `vector_input::encode` directly instead of constructing a `KeyEvent`.

use vector_input::{encode, EncodedKey, ModState, MuxCommand};
use vector_mux::Direction;
use winit::event::ElementState;
use winit::keyboard::{Key, NamedKey, SmolStr};

fn named(k: NamedKey, mods: ModState) -> Option<EncodedKey> {
    encode(&Key::Named(k), None, ElementState::Pressed, mods)
}

fn ch(s: &str, mods: ModState) -> Option<EncodedKey> {
    encode(
        &Key::Character(SmolStr::new(s)),
        Some(s),
        ElementState::Pressed,
        mods,
    )
}

fn mods(shift: bool, alt: bool, ctrl: bool) -> ModState {
    ModState {
        shift,
        alt,
        ctrl,
        cmd: false,
    }
}

fn pty(bytes: &[u8]) -> Option<EncodedKey> {
    Some(EncodedKey::Pty(bytes.to_vec()))
}

// ── Arrows × 8 mod combos (32 tests) ────────────────────────────────────────

#[test]
fn arrow_up_no_mod() {
    assert_eq!(
        named(NamedKey::ArrowUp, ModState::default()),
        pty(b"\x1b[A")
    );
}
#[test]
fn arrow_up_shift() {
    assert_eq!(
        named(NamedKey::ArrowUp, mods(true, false, false)),
        pty(b"\x1b[1;2A")
    );
}
#[test]
fn arrow_up_alt() {
    assert_eq!(
        named(NamedKey::ArrowUp, mods(false, true, false)),
        pty(b"\x1b[1;3A")
    );
}
#[test]
fn arrow_up_shift_alt() {
    assert_eq!(
        named(NamedKey::ArrowUp, mods(true, true, false)),
        pty(b"\x1b[1;4A")
    );
}
#[test]
fn arrow_up_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowUp, mods(false, false, true)),
        pty(b"\x1b[1;5A")
    );
}
#[test]
fn arrow_up_shift_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowUp, mods(true, false, true)),
        pty(b"\x1b[1;6A")
    );
}
#[test]
fn arrow_up_alt_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowUp, mods(false, true, true)),
        pty(b"\x1b[1;7A")
    );
}
#[test]
fn arrow_up_shift_alt_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowUp, mods(true, true, true)),
        pty(b"\x1b[1;8A")
    );
}

#[test]
fn arrow_down_no_mod() {
    assert_eq!(
        named(NamedKey::ArrowDown, ModState::default()),
        pty(b"\x1b[B")
    );
}
#[test]
fn arrow_down_shift() {
    assert_eq!(
        named(NamedKey::ArrowDown, mods(true, false, false)),
        pty(b"\x1b[1;2B")
    );
}
#[test]
fn arrow_down_alt() {
    assert_eq!(
        named(NamedKey::ArrowDown, mods(false, true, false)),
        pty(b"\x1b[1;3B")
    );
}
#[test]
fn arrow_down_shift_alt() {
    assert_eq!(
        named(NamedKey::ArrowDown, mods(true, true, false)),
        pty(b"\x1b[1;4B")
    );
}
#[test]
fn arrow_down_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowDown, mods(false, false, true)),
        pty(b"\x1b[1;5B")
    );
}
#[test]
fn arrow_down_shift_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowDown, mods(true, false, true)),
        pty(b"\x1b[1;6B")
    );
}
#[test]
fn arrow_down_alt_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowDown, mods(false, true, true)),
        pty(b"\x1b[1;7B")
    );
}
#[test]
fn arrow_down_shift_alt_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowDown, mods(true, true, true)),
        pty(b"\x1b[1;8B")
    );
}

#[test]
fn arrow_right_no_mod() {
    assert_eq!(
        named(NamedKey::ArrowRight, ModState::default()),
        pty(b"\x1b[C")
    );
}
#[test]
fn arrow_right_shift() {
    assert_eq!(
        named(NamedKey::ArrowRight, mods(true, false, false)),
        pty(b"\x1b[1;2C")
    );
}
#[test]
fn arrow_right_alt() {
    assert_eq!(
        named(NamedKey::ArrowRight, mods(false, true, false)),
        pty(b"\x1b[1;3C")
    );
}
#[test]
fn arrow_right_shift_alt() {
    assert_eq!(
        named(NamedKey::ArrowRight, mods(true, true, false)),
        pty(b"\x1b[1;4C")
    );
}
#[test]
fn arrow_right_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowRight, mods(false, false, true)),
        pty(b"\x1b[1;5C")
    );
}
#[test]
fn arrow_right_shift_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowRight, mods(true, false, true)),
        pty(b"\x1b[1;6C")
    );
}
#[test]
fn arrow_right_alt_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowRight, mods(false, true, true)),
        pty(b"\x1b[1;7C")
    );
}
#[test]
fn arrow_right_shift_alt_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowRight, mods(true, true, true)),
        pty(b"\x1b[1;8C")
    );
}

#[test]
fn arrow_left_no_mod() {
    assert_eq!(
        named(NamedKey::ArrowLeft, ModState::default()),
        pty(b"\x1b[D")
    );
}
#[test]
fn arrow_left_shift() {
    assert_eq!(
        named(NamedKey::ArrowLeft, mods(true, false, false)),
        pty(b"\x1b[1;2D")
    );
}
#[test]
fn arrow_left_alt() {
    assert_eq!(
        named(NamedKey::ArrowLeft, mods(false, true, false)),
        pty(b"\x1b[1;3D")
    );
}
#[test]
fn arrow_left_shift_alt() {
    assert_eq!(
        named(NamedKey::ArrowLeft, mods(true, true, false)),
        pty(b"\x1b[1;4D")
    );
}
#[test]
fn arrow_left_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowLeft, mods(false, false, true)),
        pty(b"\x1b[1;5D")
    );
}
#[test]
fn arrow_left_shift_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowLeft, mods(true, false, true)),
        pty(b"\x1b[1;6D")
    );
}
#[test]
fn arrow_left_alt_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowLeft, mods(false, true, true)),
        pty(b"\x1b[1;7D")
    );
}
#[test]
fn arrow_left_shift_alt_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowLeft, mods(true, true, true)),
        pty(b"\x1b[1;8D")
    );
}

// ── F1..F12 no-mod + a few modified (16 tests) ─────────────────────────────

#[test]
fn f1_no_mod() {
    assert_eq!(named(NamedKey::F1, ModState::default()), pty(b"\x1bOP"));
}
#[test]
fn f2_no_mod() {
    assert_eq!(named(NamedKey::F2, ModState::default()), pty(b"\x1bOQ"));
}
#[test]
fn f3_no_mod() {
    assert_eq!(named(NamedKey::F3, ModState::default()), pty(b"\x1bOR"));
}
#[test]
fn f4_no_mod() {
    assert_eq!(named(NamedKey::F4, ModState::default()), pty(b"\x1bOS"));
}
#[test]
fn f5_no_mod() {
    assert_eq!(named(NamedKey::F5, ModState::default()), pty(b"\x1b[15~"));
}
#[test]
fn f6_no_mod() {
    assert_eq!(named(NamedKey::F6, ModState::default()), pty(b"\x1b[17~"));
}
#[test]
fn f7_no_mod() {
    assert_eq!(named(NamedKey::F7, ModState::default()), pty(b"\x1b[18~"));
}
#[test]
fn f8_no_mod() {
    assert_eq!(named(NamedKey::F8, ModState::default()), pty(b"\x1b[19~"));
}
#[test]
fn f9_no_mod() {
    assert_eq!(named(NamedKey::F9, ModState::default()), pty(b"\x1b[20~"));
}
#[test]
fn f10_no_mod() {
    assert_eq!(named(NamedKey::F10, ModState::default()), pty(b"\x1b[21~"));
}
#[test]
fn f11_no_mod() {
    assert_eq!(named(NamedKey::F11, ModState::default()), pty(b"\x1b[23~"));
}
#[test]
fn f12_no_mod() {
    assert_eq!(named(NamedKey::F12, ModState::default()), pty(b"\x1b[24~"));
}
#[test]
fn f1_shift() {
    assert_eq!(
        named(NamedKey::F1, mods(true, false, false)),
        pty(b"\x1b[1;2P")
    );
}
#[test]
fn f5_ctrl() {
    assert_eq!(
        named(NamedKey::F5, mods(false, false, true)),
        pty(b"\x1b[15;5~")
    );
}
#[test]
fn f12_shift_alt() {
    assert_eq!(
        named(NamedKey::F12, mods(true, true, false)),
        pty(b"\x1b[24;4~")
    );
}
#[test]
fn f4_ctrl() {
    assert_eq!(
        named(NamedKey::F4, mods(false, false, true)),
        pty(b"\x1b[1;5S")
    );
}

// ── Navigation: Home, End, PgUp, PgDn, Insert, Delete (12 tests) ───────────

#[test]
fn home_no_mod() {
    assert_eq!(named(NamedKey::Home, ModState::default()), pty(b"\x1b[H"));
}
#[test]
fn home_shift() {
    assert_eq!(
        named(NamedKey::Home, mods(true, false, false)),
        pty(b"\x1b[1;2H")
    );
}
#[test]
fn end_no_mod() {
    assert_eq!(named(NamedKey::End, ModState::default()), pty(b"\x1b[F"));
}
#[test]
fn end_shift_alt() {
    assert_eq!(
        named(NamedKey::End, mods(true, true, false)),
        pty(b"\x1b[1;4F")
    );
}
#[test]
fn pgup_no_mod() {
    assert_eq!(
        named(NamedKey::PageUp, ModState::default()),
        pty(b"\x1b[5~")
    );
}
#[test]
fn pgup_shift() {
    assert_eq!(
        named(NamedKey::PageUp, mods(true, false, false)),
        pty(b"\x1b[5;2~")
    );
}
#[test]
fn pgdn_no_mod() {
    assert_eq!(
        named(NamedKey::PageDown, ModState::default()),
        pty(b"\x1b[6~")
    );
}
#[test]
fn pgdn_ctrl() {
    assert_eq!(
        named(NamedKey::PageDown, mods(false, false, true)),
        pty(b"\x1b[6;5~")
    );
}
#[test]
fn insert_no_mod() {
    assert_eq!(
        named(NamedKey::Insert, ModState::default()),
        pty(b"\x1b[2~")
    );
}
#[test]
fn insert_shift() {
    assert_eq!(
        named(NamedKey::Insert, mods(true, false, false)),
        pty(b"\x1b[2;2~")
    );
}
#[test]
fn delete_no_mod() {
    assert_eq!(
        named(NamedKey::Delete, ModState::default()),
        pty(b"\x1b[3~")
    );
}
#[test]
fn delete_ctrl() {
    assert_eq!(
        named(NamedKey::Delete, mods(false, false, true)),
        pty(b"\x1b[3;5~")
    );
}

// ── Special single-byte keys (6 tests) ─────────────────────────────────────

#[test]
fn escape_byte() {
    assert_eq!(named(NamedKey::Escape, ModState::default()), pty(&[0x1B]));
}
#[test]
fn enter_byte() {
    assert_eq!(named(NamedKey::Enter, ModState::default()), pty(&[0x0D]));
}
#[test]
fn tab_byte() {
    assert_eq!(named(NamedKey::Tab, ModState::default()), pty(&[0x09]));
}
#[test]
fn shift_tab() {
    assert_eq!(
        named(NamedKey::Tab, mods(true, false, false)),
        pty(b"\x1b[Z")
    );
}
#[test]
fn backspace_byte() {
    assert_eq!(
        named(NamedKey::Backspace, ModState::default()),
        pty(&[0x7F])
    );
}
#[test]
fn space_byte() {
    assert_eq!(named(NamedKey::Space, ModState::default()), pty(&[0x20]));
}

// ── Ctrl chords (8 tests) ──────────────────────────────────────────────────

#[test]
fn ctrl_a() {
    assert_eq!(ch("a", mods(false, false, true)), pty(&[0x01]));
}
#[test]
fn ctrl_c() {
    assert_eq!(ch("c", mods(false, false, true)), pty(&[0x03]));
}
#[test]
fn ctrl_d() {
    assert_eq!(ch("d", mods(false, false, true)), pty(&[0x04]));
}
#[test]
fn ctrl_m_equals_enter_byte() {
    assert_eq!(ch("m", mods(false, false, true)), pty(&[0x0D]));
}
#[test]
fn ctrl_z() {
    assert_eq!(ch("z", mods(false, false, true)), pty(&[0x1A]));
}
#[test]
fn ctrl_uppercase_treated_same() {
    assert_eq!(ch("A", mods(false, false, true)), pty(&[0x01]));
}
#[test]
fn ctrl_space() {
    assert_eq!(
        named(NamedKey::Space, mods(false, false, true)),
        pty(&[0x00])
    );
}
#[test]
fn ctrl_l() {
    assert_eq!(ch("l", mods(false, false, true)), pty(&[0x0C]));
}

// ── Option (Alt) chords (5 tests) ──────────────────────────────────────────

#[test]
fn opt_h() {
    assert_eq!(ch("h", mods(false, true, false)), pty(b"\x1bh"));
}
#[test]
fn opt_backslash() {
    assert_eq!(ch("\\", mods(false, true, false)), pty(b"\x1b\\"));
}
#[test]
fn opt_period() {
    assert_eq!(ch(".", mods(false, true, false)), pty(b"\x1b."));
}
#[test]
fn opt_shift_a() {
    assert_eq!(ch("A", mods(true, true, false)), pty(b"\x1bA"));
}
#[test]
fn opt_digit() {
    assert_eq!(ch("1", mods(false, true, false)), pty(b"\x1b1"));
}

// ── Plain character keys (4 tests) ─────────────────────────────────────────

#[test]
fn char_a_plain() {
    assert_eq!(ch("a", ModState::default()), pty(b"a"));
}
#[test]
fn char_shift_a_plain() {
    assert_eq!(ch("A", mods(true, false, false)), pty(b"A"));
}
#[test]
fn char_unicode_cjk() {
    assert_eq!(ch("中", ModState::default()), pty("中".as_bytes()));
}
#[test]
fn char_digit() {
    assert_eq!(ch("7", ModState::default()), pty(b"7"));
}

// ── Released / unmapped (3 tests) ──────────────────────────────────────────

#[test]
fn released_returns_none() {
    assert_eq!(
        encode(
            &Key::Named(NamedKey::Escape),
            None,
            ElementState::Released,
            ModState::default()
        ),
        None
    );
}
#[test]
fn released_char_returns_none() {
    assert_eq!(
        encode(
            &Key::Character(SmolStr::new("a")),
            Some("a"),
            ElementState::Released,
            ModState::default()
        ),
        None
    );
}
#[test]
fn unmapped_named_returns_none() {
    assert_eq!(named(NamedKey::Hyper, ModState::default()), None);
}

// ── Plan 04-04 Mux keybindings (D-59/60/61/62) ─────────────────────────────
// 14 Cmd-* shortcuts recognized at the keymap layer BEFORE the xterm key table.

fn cmd(shift: bool, alt: bool) -> ModState {
    ModState {
        shift,
        alt,
        ctrl: false,
        cmd: true,
    }
}

#[test]
fn cmd_t_returns_mux_new_tab() {
    assert_eq!(
        ch("t", cmd(false, false)),
        Some(EncodedKey::Mux(MuxCommand::NewTab))
    );
}

#[test]
fn cmd_d_returns_mux_split_horizontal() {
    assert_eq!(
        ch("d", cmd(false, false)),
        Some(EncodedKey::Mux(MuxCommand::SplitHorizontal))
    );
}

#[test]
fn cmd_shift_d_returns_mux_split_vertical() {
    // macOS may send shifted glyph "D" or unshifted "d"; both map to SplitVertical.
    assert_eq!(
        ch("D", cmd(true, false)),
        Some(EncodedKey::Mux(MuxCommand::SplitVertical))
    );
    assert_eq!(
        ch("d", cmd(true, false)),
        Some(EncodedKey::Mux(MuxCommand::SplitVertical))
    );
}

#[test]
fn cmd_w_returns_mux_close_pane() {
    assert_eq!(
        ch("w", cmd(false, false)),
        Some(EncodedKey::Mux(MuxCommand::ClosePane))
    );
}

#[test]
fn cmd_shift_close_bracket_returns_mux_next_tab() {
    // Accept both shifted "}" and unshifted "]".
    assert_eq!(
        ch("]", cmd(true, false)),
        Some(EncodedKey::Mux(MuxCommand::CycleTabNext))
    );
    assert_eq!(
        ch("}", cmd(true, false)),
        Some(EncodedKey::Mux(MuxCommand::CycleTabNext))
    );
}

#[test]
fn cmd_shift_open_bracket_returns_mux_prev_tab() {
    assert_eq!(
        ch("[", cmd(true, false)),
        Some(EncodedKey::Mux(MuxCommand::CycleTabPrev))
    );
    assert_eq!(
        ch("{", cmd(true, false)),
        Some(EncodedKey::Mux(MuxCommand::CycleTabPrev))
    );
}

#[test]
fn cmd_opt_left_returns_mux_focus_left() {
    assert_eq!(
        named(NamedKey::ArrowLeft, cmd(false, true)),
        Some(EncodedKey::Mux(MuxCommand::FocusDir(Direction::Left)))
    );
}

#[test]
fn cmd_opt_right_returns_mux_focus_right() {
    assert_eq!(
        named(NamedKey::ArrowRight, cmd(false, true)),
        Some(EncodedKey::Mux(MuxCommand::FocusDir(Direction::Right)))
    );
}

#[test]
fn cmd_opt_up_returns_mux_focus_up() {
    assert_eq!(
        named(NamedKey::ArrowUp, cmd(false, true)),
        Some(EncodedKey::Mux(MuxCommand::FocusDir(Direction::Up)))
    );
}

#[test]
fn cmd_opt_down_returns_mux_focus_down() {
    assert_eq!(
        named(NamedKey::ArrowDown, cmd(false, true)),
        Some(EncodedKey::Mux(MuxCommand::FocusDir(Direction::Down)))
    );
}

#[test]
fn cmd_shift_left_returns_mux_resize_nudge_left() {
    assert_eq!(
        named(NamedKey::ArrowLeft, cmd(true, false)),
        Some(EncodedKey::Mux(MuxCommand::NudgeSplit(Direction::Left)))
    );
}

#[test]
fn cmd_shift_right_returns_mux_resize_nudge_right() {
    assert_eq!(
        named(NamedKey::ArrowRight, cmd(true, false)),
        Some(EncodedKey::Mux(MuxCommand::NudgeSplit(Direction::Right)))
    );
}

#[test]
fn cmd_shift_up_returns_mux_resize_nudge_up() {
    assert_eq!(
        named(NamedKey::ArrowUp, cmd(true, false)),
        Some(EncodedKey::Mux(MuxCommand::NudgeSplit(Direction::Up)))
    );
}

#[test]
fn cmd_shift_down_returns_mux_resize_nudge_down() {
    assert_eq!(
        named(NamedKey::ArrowDown, cmd(true, false)),
        Some(EncodedKey::Mux(MuxCommand::NudgeSplit(Direction::Down)))
    );
}
