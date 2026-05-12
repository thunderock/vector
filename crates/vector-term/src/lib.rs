//! VT parser + grid + scrollback wrapper around `alacritty_terminal 0.26`.

mod dims;
mod listener;
pub mod osc_sniff;
mod parser;
mod search;
mod term;

pub use alacritty_terminal::term::{LineDamageBounds, TermDamage, TermDamageIterator};
pub use osc_sniff::{PromptKind, PromptMark};
pub use search::Match;
pub use term::Term;
