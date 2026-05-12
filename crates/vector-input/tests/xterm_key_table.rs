//! Plan 03-04 Task 1: xterm key table coverage (D-52). ≥ 80 cases.
//!
//! winit 0.30's `KeyEvent` has a private `platform_specific` field — tests must call
//! `vector_input::encode` directly instead of constructing a `KeyEvent`.

use vector_input::{encode, ModState};
use winit::event::ElementState;
use winit::keyboard::{Key, NamedKey, SmolStr};

fn named(k: NamedKey, mods: ModState) -> Option<Vec<u8>> {
    encode(&Key::Named(k), None, ElementState::Pressed, mods)
}

fn ch(s: &str, mods: ModState) -> Option<Vec<u8>> {
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

// ── Arrows × 8 mod combos (32 tests) ────────────────────────────────────────

#[test]
fn arrow_up_no_mod() {
    assert_eq!(
        named(NamedKey::ArrowUp, ModState::default()),
        Some(b"\x1b[A".to_vec())
    );
}
#[test]
fn arrow_up_shift() {
    assert_eq!(
        named(NamedKey::ArrowUp, mods(true, false, false)),
        Some(b"\x1b[1;2A".to_vec())
    );
}
#[test]
fn arrow_up_alt() {
    assert_eq!(
        named(NamedKey::ArrowUp, mods(false, true, false)),
        Some(b"\x1b[1;3A".to_vec())
    );
}
#[test]
fn arrow_up_shift_alt() {
    assert_eq!(
        named(NamedKey::ArrowUp, mods(true, true, false)),
        Some(b"\x1b[1;4A".to_vec())
    );
}
#[test]
fn arrow_up_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowUp, mods(false, false, true)),
        Some(b"\x1b[1;5A".to_vec())
    );
}
#[test]
fn arrow_up_shift_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowUp, mods(true, false, true)),
        Some(b"\x1b[1;6A".to_vec())
    );
}
#[test]
fn arrow_up_alt_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowUp, mods(false, true, true)),
        Some(b"\x1b[1;7A".to_vec())
    );
}
#[test]
fn arrow_up_shift_alt_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowUp, mods(true, true, true)),
        Some(b"\x1b[1;8A".to_vec())
    );
}

#[test]
fn arrow_down_no_mod() {
    assert_eq!(
        named(NamedKey::ArrowDown, ModState::default()),
        Some(b"\x1b[B".to_vec())
    );
}
#[test]
fn arrow_down_shift() {
    assert_eq!(
        named(NamedKey::ArrowDown, mods(true, false, false)),
        Some(b"\x1b[1;2B".to_vec())
    );
}
#[test]
fn arrow_down_alt() {
    assert_eq!(
        named(NamedKey::ArrowDown, mods(false, true, false)),
        Some(b"\x1b[1;3B".to_vec())
    );
}
#[test]
fn arrow_down_shift_alt() {
    assert_eq!(
        named(NamedKey::ArrowDown, mods(true, true, false)),
        Some(b"\x1b[1;4B".to_vec())
    );
}
#[test]
fn arrow_down_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowDown, mods(false, false, true)),
        Some(b"\x1b[1;5B".to_vec())
    );
}
#[test]
fn arrow_down_shift_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowDown, mods(true, false, true)),
        Some(b"\x1b[1;6B".to_vec())
    );
}
#[test]
fn arrow_down_alt_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowDown, mods(false, true, true)),
        Some(b"\x1b[1;7B".to_vec())
    );
}
#[test]
fn arrow_down_shift_alt_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowDown, mods(true, true, true)),
        Some(b"\x1b[1;8B".to_vec())
    );
}

#[test]
fn arrow_right_no_mod() {
    assert_eq!(
        named(NamedKey::ArrowRight, ModState::default()),
        Some(b"\x1b[C".to_vec())
    );
}
#[test]
fn arrow_right_shift() {
    assert_eq!(
        named(NamedKey::ArrowRight, mods(true, false, false)),
        Some(b"\x1b[1;2C".to_vec())
    );
}
#[test]
fn arrow_right_alt() {
    assert_eq!(
        named(NamedKey::ArrowRight, mods(false, true, false)),
        Some(b"\x1b[1;3C".to_vec())
    );
}
#[test]
fn arrow_right_shift_alt() {
    assert_eq!(
        named(NamedKey::ArrowRight, mods(true, true, false)),
        Some(b"\x1b[1;4C".to_vec())
    );
}
#[test]
fn arrow_right_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowRight, mods(false, false, true)),
        Some(b"\x1b[1;5C".to_vec())
    );
}
#[test]
fn arrow_right_shift_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowRight, mods(true, false, true)),
        Some(b"\x1b[1;6C".to_vec())
    );
}
#[test]
fn arrow_right_alt_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowRight, mods(false, true, true)),
        Some(b"\x1b[1;7C".to_vec())
    );
}
#[test]
fn arrow_right_shift_alt_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowRight, mods(true, true, true)),
        Some(b"\x1b[1;8C".to_vec())
    );
}

#[test]
fn arrow_left_no_mod() {
    assert_eq!(
        named(NamedKey::ArrowLeft, ModState::default()),
        Some(b"\x1b[D".to_vec())
    );
}
#[test]
fn arrow_left_shift() {
    assert_eq!(
        named(NamedKey::ArrowLeft, mods(true, false, false)),
        Some(b"\x1b[1;2D".to_vec())
    );
}
#[test]
fn arrow_left_alt() {
    assert_eq!(
        named(NamedKey::ArrowLeft, mods(false, true, false)),
        Some(b"\x1b[1;3D".to_vec())
    );
}
#[test]
fn arrow_left_shift_alt() {
    assert_eq!(
        named(NamedKey::ArrowLeft, mods(true, true, false)),
        Some(b"\x1b[1;4D".to_vec())
    );
}
#[test]
fn arrow_left_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowLeft, mods(false, false, true)),
        Some(b"\x1b[1;5D".to_vec())
    );
}
#[test]
fn arrow_left_shift_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowLeft, mods(true, false, true)),
        Some(b"\x1b[1;6D".to_vec())
    );
}
#[test]
fn arrow_left_alt_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowLeft, mods(false, true, true)),
        Some(b"\x1b[1;7D".to_vec())
    );
}
#[test]
fn arrow_left_shift_alt_ctrl() {
    assert_eq!(
        named(NamedKey::ArrowLeft, mods(true, true, true)),
        Some(b"\x1b[1;8D".to_vec())
    );
}

// ── F1..F12 no-mod + a few modified (16 tests) ─────────────────────────────

#[test]
fn f1_no_mod() {
    assert_eq!(
        named(NamedKey::F1, ModState::default()),
        Some(b"\x1bOP".to_vec())
    );
}
#[test]
fn f2_no_mod() {
    assert_eq!(
        named(NamedKey::F2, ModState::default()),
        Some(b"\x1bOQ".to_vec())
    );
}
#[test]
fn f3_no_mod() {
    assert_eq!(
        named(NamedKey::F3, ModState::default()),
        Some(b"\x1bOR".to_vec())
    );
}
#[test]
fn f4_no_mod() {
    assert_eq!(
        named(NamedKey::F4, ModState::default()),
        Some(b"\x1bOS".to_vec())
    );
}
#[test]
fn f5_no_mod() {
    assert_eq!(
        named(NamedKey::F5, ModState::default()),
        Some(b"\x1b[15~".to_vec())
    );
}
#[test]
fn f6_no_mod() {
    assert_eq!(
        named(NamedKey::F6, ModState::default()),
        Some(b"\x1b[17~".to_vec())
    );
}
#[test]
fn f7_no_mod() {
    assert_eq!(
        named(NamedKey::F7, ModState::default()),
        Some(b"\x1b[18~".to_vec())
    );
}
#[test]
fn f8_no_mod() {
    assert_eq!(
        named(NamedKey::F8, ModState::default()),
        Some(b"\x1b[19~".to_vec())
    );
}
#[test]
fn f9_no_mod() {
    assert_eq!(
        named(NamedKey::F9, ModState::default()),
        Some(b"\x1b[20~".to_vec())
    );
}
#[test]
fn f10_no_mod() {
    assert_eq!(
        named(NamedKey::F10, ModState::default()),
        Some(b"\x1b[21~".to_vec())
    );
}
#[test]
fn f11_no_mod() {
    assert_eq!(
        named(NamedKey::F11, ModState::default()),
        Some(b"\x1b[23~".to_vec())
    );
}
#[test]
fn f12_no_mod() {
    assert_eq!(
        named(NamedKey::F12, ModState::default()),
        Some(b"\x1b[24~".to_vec())
    );
}
#[test]
fn f1_shift() {
    assert_eq!(
        named(NamedKey::F1, mods(true, false, false)),
        Some(b"\x1b[1;2P".to_vec())
    );
}
#[test]
fn f5_ctrl() {
    assert_eq!(
        named(NamedKey::F5, mods(false, false, true)),
        Some(b"\x1b[15;5~".to_vec())
    );
}
#[test]
fn f12_shift_alt() {
    assert_eq!(
        named(NamedKey::F12, mods(true, true, false)),
        Some(b"\x1b[24;4~".to_vec())
    );
}
#[test]
fn f4_ctrl() {
    assert_eq!(
        named(NamedKey::F4, mods(false, false, true)),
        Some(b"\x1b[1;5S".to_vec())
    );
}

// ── Navigation: Home, End, PgUp, PgDn, Insert, Delete (12 tests) ───────────

#[test]
fn home_no_mod() {
    assert_eq!(
        named(NamedKey::Home, ModState::default()),
        Some(b"\x1b[H".to_vec())
    );
}
#[test]
fn home_shift() {
    assert_eq!(
        named(NamedKey::Home, mods(true, false, false)),
        Some(b"\x1b[1;2H".to_vec())
    );
}
#[test]
fn end_no_mod() {
    assert_eq!(
        named(NamedKey::End, ModState::default()),
        Some(b"\x1b[F".to_vec())
    );
}
#[test]
fn end_shift_alt() {
    assert_eq!(
        named(NamedKey::End, mods(true, true, false)),
        Some(b"\x1b[1;4F".to_vec())
    );
}
#[test]
fn pgup_no_mod() {
    assert_eq!(
        named(NamedKey::PageUp, ModState::default()),
        Some(b"\x1b[5~".to_vec())
    );
}
#[test]
fn pgup_shift() {
    assert_eq!(
        named(NamedKey::PageUp, mods(true, false, false)),
        Some(b"\x1b[5;2~".to_vec())
    );
}
#[test]
fn pgdn_no_mod() {
    assert_eq!(
        named(NamedKey::PageDown, ModState::default()),
        Some(b"\x1b[6~".to_vec())
    );
}
#[test]
fn pgdn_ctrl() {
    assert_eq!(
        named(NamedKey::PageDown, mods(false, false, true)),
        Some(b"\x1b[6;5~".to_vec())
    );
}
#[test]
fn insert_no_mod() {
    assert_eq!(
        named(NamedKey::Insert, ModState::default()),
        Some(b"\x1b[2~".to_vec())
    );
}
#[test]
fn insert_shift() {
    assert_eq!(
        named(NamedKey::Insert, mods(true, false, false)),
        Some(b"\x1b[2;2~".to_vec())
    );
}
#[test]
fn delete_no_mod() {
    assert_eq!(
        named(NamedKey::Delete, ModState::default()),
        Some(b"\x1b[3~".to_vec())
    );
}
#[test]
fn delete_ctrl() {
    assert_eq!(
        named(NamedKey::Delete, mods(false, false, true)),
        Some(b"\x1b[3;5~".to_vec())
    );
}

// ── Special single-byte keys (6 tests) ─────────────────────────────────────

#[test]
fn escape_byte() {
    assert_eq!(
        named(NamedKey::Escape, ModState::default()),
        Some(vec![0x1B])
    );
}
#[test]
fn enter_byte() {
    assert_eq!(
        named(NamedKey::Enter, ModState::default()),
        Some(vec![0x0D])
    );
}
#[test]
fn tab_byte() {
    assert_eq!(named(NamedKey::Tab, ModState::default()), Some(vec![0x09]));
}
#[test]
fn shift_tab() {
    assert_eq!(
        named(NamedKey::Tab, mods(true, false, false)),
        Some(b"\x1b[Z".to_vec())
    );
}
#[test]
fn backspace_byte() {
    assert_eq!(
        named(NamedKey::Backspace, ModState::default()),
        Some(vec![0x7F])
    );
}
#[test]
fn space_byte() {
    assert_eq!(
        named(NamedKey::Space, ModState::default()),
        Some(vec![0x20])
    );
}

// ── Ctrl chords (8 tests) ──────────────────────────────────────────────────

#[test]
fn ctrl_a() {
    assert_eq!(ch("a", mods(false, false, true)), Some(vec![0x01]));
}
#[test]
fn ctrl_c() {
    assert_eq!(ch("c", mods(false, false, true)), Some(vec![0x03]));
}
#[test]
fn ctrl_d() {
    assert_eq!(ch("d", mods(false, false, true)), Some(vec![0x04]));
}
#[test]
fn ctrl_m_equals_enter_byte() {
    assert_eq!(ch("m", mods(false, false, true)), Some(vec![0x0D]));
}
#[test]
fn ctrl_z() {
    assert_eq!(ch("z", mods(false, false, true)), Some(vec![0x1A]));
}
#[test]
fn ctrl_uppercase_treated_same() {
    assert_eq!(ch("A", mods(false, false, true)), Some(vec![0x01]));
}
#[test]
fn ctrl_space() {
    assert_eq!(
        named(NamedKey::Space, mods(false, false, true)),
        Some(vec![0x00])
    );
}
#[test]
fn ctrl_l() {
    assert_eq!(ch("l", mods(false, false, true)), Some(vec![0x0C]));
}

// ── Option (Alt) chords (5 tests) ──────────────────────────────────────────

#[test]
fn opt_h() {
    assert_eq!(ch("h", mods(false, true, false)), Some(b"\x1bh".to_vec()));
}
#[test]
fn opt_backslash() {
    assert_eq!(ch("\\", mods(false, true, false)), Some(b"\x1b\\".to_vec()));
}
#[test]
fn opt_period() {
    assert_eq!(ch(".", mods(false, true, false)), Some(b"\x1b.".to_vec()));
}
#[test]
fn opt_shift_a() {
    assert_eq!(ch("A", mods(true, true, false)), Some(b"\x1bA".to_vec()));
}
#[test]
fn opt_digit() {
    assert_eq!(ch("1", mods(false, true, false)), Some(b"\x1b1".to_vec()));
}

// ── Plain character keys (4 tests) ─────────────────────────────────────────

#[test]
fn char_a_plain() {
    assert_eq!(ch("a", ModState::default()), Some(b"a".to_vec()));
}
#[test]
fn char_shift_a_plain() {
    assert_eq!(ch("A", mods(true, false, false)), Some(b"A".to_vec()));
}
#[test]
fn char_unicode_cjk() {
    assert_eq!(
        ch("中", ModState::default()),
        Some("中".as_bytes().to_vec())
    );
}
#[test]
fn char_digit() {
    assert_eq!(ch("7", ModState::default()), Some(b"7".to_vec()));
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

// ── Wave-0 stubs: Plan 04-04 Mux keybindings (D-59/60/61/62) ────────────────
// Stub bodies panic until Plan 04-04 rewrites each to assert
// `encode(...) == Some(EncodedKey::Mux(MuxCommand::*))`.

#[test]
#[ignore = "Wave-0 stub: Plan 04-04"]
fn cmd_t_returns_mux_new_tab() {
    panic!("Wave-0 stub — implemented by Plan 04-04");
}

#[test]
#[ignore = "Wave-0 stub: Plan 04-04"]
fn cmd_d_returns_mux_split_horizontal() {
    panic!("Wave-0 stub — implemented by Plan 04-04");
}

#[test]
#[ignore = "Wave-0 stub: Plan 04-04"]
fn cmd_shift_d_returns_mux_split_vertical() {
    panic!("Wave-0 stub — implemented by Plan 04-04");
}

#[test]
#[ignore = "Wave-0 stub: Plan 04-04"]
fn cmd_w_returns_mux_close_pane() {
    panic!("Wave-0 stub — implemented by Plan 04-04");
}

#[test]
#[ignore = "Wave-0 stub: Plan 04-04"]
fn cmd_shift_close_bracket_returns_mux_next_tab() {
    panic!("Wave-0 stub — implemented by Plan 04-04");
}

#[test]
#[ignore = "Wave-0 stub: Plan 04-04"]
fn cmd_shift_open_bracket_returns_mux_prev_tab() {
    panic!("Wave-0 stub — implemented by Plan 04-04");
}

#[test]
#[ignore = "Wave-0 stub: Plan 04-04"]
fn cmd_opt_left_returns_mux_focus_left() {
    panic!("Wave-0 stub — implemented by Plan 04-04");
}

#[test]
#[ignore = "Wave-0 stub: Plan 04-04"]
fn cmd_opt_right_returns_mux_focus_right() {
    panic!("Wave-0 stub — implemented by Plan 04-04");
}

#[test]
#[ignore = "Wave-0 stub: Plan 04-04"]
fn cmd_opt_up_returns_mux_focus_up() {
    panic!("Wave-0 stub — implemented by Plan 04-04");
}

#[test]
#[ignore = "Wave-0 stub: Plan 04-04"]
fn cmd_opt_down_returns_mux_focus_down() {
    panic!("Wave-0 stub — implemented by Plan 04-04");
}

#[test]
#[ignore = "Wave-0 stub: Plan 04-04"]
fn cmd_shift_left_returns_mux_resize_nudge_left() {
    panic!("Wave-0 stub — implemented by Plan 04-04");
}

#[test]
#[ignore = "Wave-0 stub: Plan 04-04"]
fn cmd_shift_right_returns_mux_resize_nudge_right() {
    panic!("Wave-0 stub — implemented by Plan 04-04");
}

#[test]
#[ignore = "Wave-0 stub: Plan 04-04"]
fn cmd_shift_up_returns_mux_resize_nudge_up() {
    panic!("Wave-0 stub — implemented by Plan 04-04");
}

#[test]
#[ignore = "Wave-0 stub: Plan 04-04"]
fn cmd_shift_down_returns_mux_resize_nudge_down() {
    panic!("Wave-0 stub — implemented by Plan 04-04");
}
