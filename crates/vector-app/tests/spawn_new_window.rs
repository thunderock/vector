//! Plan 05-14 Task 2 TDD — Test 4: EncodedKey::App(AppShortcut::SpawnNewWindow)
//! for Cmd-N. End-to-end regression guard against Plan 05-13 keymap regressions.

use vector_input::{encode, AppShortcut, EncodedKey, ModState};
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

#[test]
fn cmd_n_encodes_spawn_new_window() {
    let result = encode(
        &Key::Character(SmolStr::new("n")),
        Some("n"),
        ElementState::Pressed,
        cmd(),
    );
    assert!(
        matches!(result, Some(EncodedKey::App(AppShortcut::SpawnNewWindow))),
        "Cmd-N must encode to EncodedKey::App(SpawnNewWindow); got {result:?}",
    );
}
