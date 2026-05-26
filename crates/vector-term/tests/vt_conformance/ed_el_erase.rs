//! HARDEN-02 Scenario 4: ED (erase display) + EL (erase line).
//! Maps to PITFALLS.md Pitfall 1 (VT parser correctness).
//! Mirrors `crates/vector-term/tests/ed_el_erase.rs`.

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use regex::Regex;
use vector_term::Term;

#[test]
fn ed_2_clears_visible_grid_not_scrollback() {
    let mut term = Term::new(80, 24, 1000);
    for i in 0..50 {
        term.feed(format!("line {i}\r\n").as_bytes());
    }
    let history_before = term.grid().history_size();
    assert!(history_before > 0, "fixture must have produced scrollback");
    term.feed(b"\x1b[2J");
    let cell = &term.grid()[Point::new(Line(0), Column(0))];
    assert_eq!(cell.c, ' ', "ED 2 must clear visible grid");
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
    // Move to 1-based col 4 (= 0-based col 3), then EL 0.
    term.feed(b"\x1b[1;4H\x1b[0K");
    let row0 = Line(0);
    assert_eq!(term.grid()[Point::new(row0, Column(0))].c, 'A');
    assert_eq!(term.grid()[Point::new(row0, Column(1))].c, 'B');
    assert_eq!(term.grid()[Point::new(row0, Column(2))].c, 'C');
    assert_eq!(term.grid()[Point::new(row0, Column(3))].c, ' ');
    assert_eq!(term.grid()[Point::new(row0, Column(7))].c, ' ');
}

#[test]
fn el_1_erases_from_start_of_line_to_cursor() {
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"ABCDEFGH");
    // Move to 1-based col 4 (= 0-based col 3), then EL 1 (erase BOL through cursor).
    term.feed(b"\x1b[1;4H\x1b[1K");
    let row0 = Line(0);
    assert_eq!(term.grid()[Point::new(row0, Column(0))].c, ' ');
    assert_eq!(term.grid()[Point::new(row0, Column(3))].c, ' ');
    // Past cursor untouched.
    assert_eq!(term.grid()[Point::new(row0, Column(4))].c, 'E');
    assert_eq!(term.grid()[Point::new(row0, Column(7))].c, 'H');
}
