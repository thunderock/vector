//! Plan 05-11 (POLISH-06) — `GridAccess` adapter for `vector_term::Term`.
//!
//! B1 fix: trait impl lives in `vector-app` (not `vector-term`) to avoid
//! a vector-input -> vector-mux -> vector-term -> vector-input cycle.

use vector_input::GridAccess;
use vector_term::Term;

/// Newtype wrapper letting `selection_to_string` read cells from `Term`.
pub struct TermGridAccess<'a>(pub &'a Term);

impl GridAccess for TermGridAccess<'_> {
    fn cell_char(&self, row: usize, col: usize) -> Option<char> {
        self.0.cell_at(row, col).map(|(c, _)| c)
    }
    fn cell_is_wide_spacer(&self, row: usize, col: usize) -> bool {
        self.0.cell_at(row, col).is_some_and(|(_, w)| w)
    }
    fn cols(&self) -> usize {
        self.0.grid_cols()
    }
}
