//! CORE-01: ED (erase display) + EL (erase line) semantics.

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use regex::Regex;
use vector_term::Term;

#[test]
fn ed_2_clears_visible_grid_not_scrollback() {
    // ED 2 leaves scrollback intact (matches xterm).
    let mut term = Term::new(80, 24, 1000);
    for i in 0..50 {
        term.feed(format!("line {i}\r\n").as_bytes());
    }
    let history_before = term.grid().history_size();
    assert!(history_before > 0, "fixture must have produced scrollback");
    term.feed(b"\x1b[2J");
    // Visible viewport should be cleared.
    let cell = &term.grid()[Point::new(Line(0), Column(0))];
    assert_eq!(cell.c, ' ', "ED 2 must clear visible grid");
    // Scrollback must still contain earlier lines — searchable end-to-end.
    let needle = Regex::new(r"line 5").unwrap();
    let matches = term.search(&needle);
    assert!(
        !matches.is_empty(),
        "ED 2 must not clear scrollback: search after ED 2 finds {} matches",
        matches.len()
    );
}

#[test]
fn el_0_erases_to_end_of_line() {
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"ABCDEFGH");
    // Move to col 4 (1-based) = 0-based col 3, then EL 0 (erase to EOL).
    term.feed(b"\x1b[1;4H\x1b[0K");
    let row0 = Line(0);
    assert_eq!(term.grid()[Point::new(row0, Column(0))].c, 'A');
    assert_eq!(term.grid()[Point::new(row0, Column(1))].c, 'B');
    assert_eq!(term.grid()[Point::new(row0, Column(2))].c, 'C');
    // EL 0 erases from cursor (col 3) through end-of-line — those cells reset to ' '.
    assert_eq!(term.grid()[Point::new(row0, Column(3))].c, ' ');
    assert_eq!(term.grid()[Point::new(row0, Column(7))].c, ' ');
}
