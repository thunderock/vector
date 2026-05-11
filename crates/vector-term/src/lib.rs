//! VT parser + grid + scrollback wrapper around `alacritty_terminal 0.26`.

mod dims;
mod listener;
mod parser;
mod search;
mod term;

pub use alacritty_terminal::term::{LineDamageBounds, TermDamage, TermDamageIterator};
pub use search::Match;
pub use term::Term;
