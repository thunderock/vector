//! HARDEN-02 Scenario 2: DECSTBM scroll region constraint.
//! Maps to PITFALLS.md Pitfall 1 (VT parser correctness).
//! Mirrors `crates/vector-term/tests/decstbm_scroll_region.rs::decstbm_constrains_scroll_to_region`.

use alacritty_terminal::index::{Column, Line, Point};
use vector_term::Term;

#[test]
fn decstbm_constrains_scroll_to_region() {
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"\x1b[2;1H");
    term.feed(b"TOP_OUTSIDE");
    term.feed(b"\x1b[24;1H");
    term.feed(b"BOT_OUTSIDE");
    // DECSTBM rows 5..10 (1-based).
    term.feed(b"\x1b[5;10r");
    term.feed(b"\x1b[10;1H");
    for i in 0..20 {
        term.feed(format!("region_line_{i}\n").as_bytes());
    }
    let top = &term.grid()[Point::new(Line(1), Column(0))];
    let bot = &term.grid()[Point::new(Line(23), Column(0))];
    assert_eq!(top.c, 'T', "row above DECSTBM region must not scroll");
    assert_eq!(bot.c, 'B', "row below DECSTBM region must not scroll");
}

#[test]
fn decstbm_reset_full_screen_region() {
    // Reset DECSTBM (CSI r with no params) restores full-screen scrolling.
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"\x1b[5;10r");
    term.feed(b"\x1b[r"); // reset to full screen
                          // After reset, top-of-screen content should scroll normally.
    term.feed(b"\x1b[1;1HHEADER");
    for _ in 0..30 {
        term.feed(b"\n");
    }
    // HEADER should have scrolled out of viewport entirely.
    let cell = &term.grid()[Point::new(Line(0), Column(0))];
    assert_ne!(
        cell.c, 'H',
        "after DECSTBM reset, full-screen scroll must move HEADER off-screen"
    );
}
