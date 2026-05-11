//! Modifier state extracted from winit's `ModifiersState`. Used by the keymap encoder.

use winit::keyboard::ModifiersState;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // 4 modifier flags maps 1:1 to xterm mod_param.
pub struct ModState {
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
    pub cmd: bool,
}

impl ModState {
    #[must_use]
    pub fn from_winit(s: ModifiersState) -> Self {
        Self {
            shift: s.shift_key(),
            alt: s.alt_key(),
            ctrl: s.control_key(),
            cmd: s.super_key(),
        }
    }

    /// xterm mod_param: 1..=8. Cmd is NOT a terminal modifier (Cmd-* are app shortcuts).
    #[must_use]
    pub fn xterm_mod_param(&self) -> u8 {
        let mut p = 1u8;
        if self.shift {
            p += 1;
        }
        if self.alt {
            p += 2;
        }
        if self.ctrl {
            p += 4;
        }
        p
    }
}
