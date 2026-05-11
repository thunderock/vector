//! Library-only regex search across visible grid + scrollback (CORE-03, D-39).
//! Streaming DFA via `alacritty_terminal::term::search` — never materializes
//! scrollback into a `String` (Pitfall 7).

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Direction, Point};
use alacritty_terminal::term::search::{RegexIter, RegexSearch};
use regex::Regex;

use crate::term::Term;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub start_row: i32,
    pub start_col: u16,
    pub end_row: i32,
    pub end_col: u16,
}

impl Term {
    #[allow(clippy::cast_possible_truncation)]
    pub fn search(&self, regex: &Regex) -> Vec<Match> {
        let Ok(mut dfa) = RegexSearch::new(regex.as_str()) else {
            return Vec::new();
        };
        let inner = self.inner();
        let start = Point::new(inner.topmost_line(), Column(0));
        let end = Point::new(inner.bottommost_line(), inner.last_column());
        RegexIter::new(start, end, Direction::Right, inner, &mut dfa)
            .map(|m| Match {
                start_row: m.start().line.0,
                start_col: m.start().column.0 as u16,
                end_row: m.end().line.0,
                end_col: m.end().column.0 as u16,
            })
            .collect()
    }
}
