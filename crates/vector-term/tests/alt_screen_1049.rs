//! CORE-01: DECSET 1049 alt-screen enter/exit + cursor save/restore (vim pattern).

use alacritty_terminal::index::{Column, Line, Point};
use vector_term::Term;

#[test]
fn decset_1049_alt_screen_isolates_primary() {
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"primary content\n");
    term.feed(b"\x1b[?1049h"); // enter alt + save cursor
    term.feed(b"alt content");
    term.feed(b"\x1b[?1049l"); // exit alt + restore cursor
    let cell = &term.grid()[Point::new(Line(0), Column(0))];
    assert_eq!(
        cell.c, 'p',
        "primary content must be restored after 1049 exit"
    );
}

#[test]
fn decset_1049_restores_cursor_position_on_exit() {
    let mut term = Term::new(80, 24, 1000);
    // Move cursor to (col=4, row=4) — CSI is 1-based.
    term.feed(b"\x1b[5;5H");
    let before = term.cursor();
    assert_eq!(before, (4, 4));
    term.feed(b"\x1b[?1049h"); // enter alt — cursor saved
    term.feed(b"\x1b[20;20H"); // move elsewhere in alt
    term.feed(b"\x1b[?1049l"); // exit alt — cursor restored
    assert_eq!(term.cursor(), (4, 4));
}
