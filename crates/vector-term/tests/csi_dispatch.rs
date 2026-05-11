//! CORE-01: CSI cursor positioning + SGR + ED/EL dispatch.

use alacritty_terminal::index::{Column, Line, Point};
use vector_term::Term;

#[test]
fn echo_hello_lands_in_cell_0_0() {
    // ROADMAP success criterion #1.
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"hello");
    let cell = &term.grid()[Point::new(Line(0), Column(0))];
    assert_eq!(cell.c, 'h');
}

#[test]
fn cursor_position_csi_h() {
    // CSI row;colH is 1-based; we expose 0-based (col, row).
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"\x1b[3;5H");
    assert_eq!(term.cursor(), (4, 2));
}

#[test]
fn cursor_movement_csi_abcd() {
    let mut term = Term::new(80, 24, 1000);
    // From (0,0): CUD 2 → row 2; CUF 3 → col 3.
    term.feed(b"\x1b[2B\x1b[3C");
    assert_eq!(term.cursor(), (3, 2));
}
