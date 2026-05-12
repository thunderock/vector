//! Keymap + paste + selection state. Phase 3 (D-52, D-53, D-54).

mod keymap;
mod mods;
mod paste;
mod selection;

pub use keymap::{encode, encode_key, EncodedKey, MuxCommand};
pub use mods::ModState;
pub use paste::wrap_bracketed_paste;
pub use selection::{SelectionRange, SelectionState};
