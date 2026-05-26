//! HARDEN-02 Scenario 1: DECSET 1049 alt-screen enter/exit + cursor save/restore.
//! Maps to PITFALLS.md Pitfall 1 (VT parser correctness).
//! Mirrors `crates/vector-term/tests/alt_screen_1049.rs`.

use alacritty_terminal::index::{Column, Line, Point};
use vector_term::Term;

#[test]
fn decset_1049_alt_screen_isolates_primary() {
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"primary content\n");
    term.feed(b"\x1b[?1049h");
    term.feed(b"alt content");
    term.feed(b"\x1b[?1049l");
    let cell = &term.grid()[Point::new(Line(0), Column(0))];
    assert_eq!(
        cell.c, 'p',
        "primary content must be restored after 1049 exit"
    );
}

#[test]
fn decset_1049_restores_cursor_position_on_exit() {
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"\x1b[5;5H");
    assert_eq!(term.cursor(), (4, 4));
    term.feed(b"\x1b[?1049h");
    term.feed(b"\x1b[20;20H");
    term.feed(b"\x1b[?1049l");
    assert_eq!(term.cursor(), (4, 4));
}
