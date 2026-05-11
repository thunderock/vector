//! CORE-01: partial UTF-8 split across `feed()` calls. Pitfall 4 — parser
//! reassembles internally; we never decode bytes at the boundary.

use alacritty_terminal::index::{Column, Line, Point};
use vector_term::Term;

#[test]
fn utf8_multibyte_split_across_two_feeds() {
    // 世 = U+4E16 = E4 B8 96. Split 2+1.
    let mut term = Term::new(80, 24, 1000);
    term.feed(&[0xE4, 0xB8]);
    term.feed(&[0x96]);
    let cell = &term.grid()[Point::new(Line(0), Column(0))];
    assert_eq!(cell.c, '世');
}

#[test]
fn utf8_four_byte_codepoint_split_three_ways() {
    // 🦀 = U+1F980 = F0 9F A6 80. Split 2+1+1.
    let mut term = Term::new(80, 24, 1000);
    term.feed(&[0xF0, 0x9F]);
    term.feed(&[0xA6]);
    term.feed(&[0x80]);
    let cell = &term.grid()[Point::new(Line(0), Column(0))];
    assert_eq!(cell.c, '🦀');
}
