//! HARDEN-02 Scenario 8: DECSCUSR cursor-shape selection.
//! Maps to PITFALLS.md Pitfall 1 (VT parser correctness).
//! Mirrors `crates/vector-term/tests/dcs_dispatch.rs::decscusr_cursor_shape_sets_state`.

use alacritty_terminal::index::{Column, Line, Point};
use vector_term::Term;

#[test]
fn decscusr_blink_bar_dispatches_cleanly() {
    // DECSCUSR 5 = blinking bar. The grid getter exposes no cursor-shape field
    // today; we prove the parser dispatched the sequence by ensuring follow-up
    // output still lands correctly at the cursor.
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"\x1b[5 q");
    term.feed(b"X");
    let cell = &term.grid()[Point::new(Line(0), Column(0))];
    assert_eq!(
        cell.c, 'X',
        "DECSCUSR 5 must not corrupt subsequent character placement"
    );
}

#[test]
fn decscusr_reset_default_dispatches_cleanly() {
    // DECSCUSR 0 = reset to default.
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"\x1b[5 q");
    term.feed(b"\x1b[0 q");
    term.feed(b"Y");
    let cell = &term.grid()[Point::new(Line(0), Column(0))];
    assert_eq!(
        cell.c, 'Y',
        "DECSCUSR 0 must not corrupt subsequent character placement"
    );
}
