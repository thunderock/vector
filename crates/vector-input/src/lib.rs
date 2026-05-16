//! Keymap + paste + selection state. Phase 3 (D-52, D-53, D-54).
//! POLISH-05 (Plan 05-06): OSC 52 outbound emitter with 58-byte chunking (D-71).

pub mod clipboard;
mod keymap;
mod mods;
mod paste;
mod selection;
mod selection_string;

pub use clipboard::{osc52_outbound, MAX_CHUNK_BASE64};
pub use keymap::{encode, encode_key, AppShortcut, EncodedKey, MuxCommand};
pub use mods::ModState;
pub use paste::wrap_bracketed_paste;
pub use selection::{SelectionRange, SelectionState};
pub use selection_string::{selection_to_string, GridAccess, SelectionMode};
