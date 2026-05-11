//! CORE-01: OSC title + color dispatch — parser must survive OSC without corrupting state.

use alacritty_terminal::index::{Column, Line, Point};
use vector_term::Term;

#[test]
fn osc_2_sets_window_title() {
    // OSC 2 "set window title" — we don't expose title getter in Phase 2, so we
    // prove the parser handled the sequence by feeding a follow-up char and
    // confirming it lands cleanly (parser state recovered).
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"\x1b]2;mytitle\x1b\\X");
    let cell = &term.grid()[Point::new(Line(0), Column(0))];
    assert_eq!(cell.c, 'X');
}

#[test]
fn osc_10_11_query_default_colors() {
    // OSC 10 / 11 = foreground / background color queries. Sender expects a
    // reply on the PTY out-channel; the parser must accept and not corrupt.
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"\x1b]10;?\x1b\\\x1b]11;?\x1b\\hello");
    let cell = &term.grid()[Point::new(Line(0), Column(0))];
    assert_eq!(cell.c, 'h');
}
