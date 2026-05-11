//! CORE-02: emoji ZWJ + East Asian wide cell width.

use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::cell::Flags;
use vector_term::Term;

#[test]
fn emoji_zwj_family_occupies_two_cells() {
    let mut term = Term::new(80, 24, 1000);
    // 👨‍👩‍👧 (man+ZWJ+woman+ZWJ+girl) — alacritty stores the lead char in cell 0
    // with WIDE_CHAR set, and cell 1 with WIDE_CHAR_SPACER.
    term.feed("👨\u{200D}👩\u{200D}👧".as_bytes());
    let row0 = Line(0);
    let lead = &term.grid()[Point::new(row0, Column(0))];
    let spacer = &term.grid()[Point::new(row0, Column(1))];
    assert!(
        lead.flags.contains(Flags::WIDE_CHAR),
        "lead cell must carry WIDE_CHAR flag for emoji"
    );
    assert!(
        spacer.flags.contains(Flags::WIDE_CHAR_SPACER),
        "next cell must carry WIDE_CHAR_SPACER for a wide grapheme"
    );
}

#[test]
fn east_asian_wide_cjk_char_occupies_two_cells() {
    let mut term = Term::new(80, 24, 1000);
    term.feed("中".as_bytes());
    let row0 = Line(0);
    let lead = &term.grid()[Point::new(row0, Column(0))];
    let spacer = &term.grid()[Point::new(row0, Column(1))];
    assert_eq!(lead.c, '中');
    assert!(
        lead.flags.contains(Flags::WIDE_CHAR),
        "CJK lead cell must carry WIDE_CHAR flag"
    );
    assert!(
        spacer.flags.contains(Flags::WIDE_CHAR_SPACER),
        "cell to the right of a wide CJK char must be WIDE_CHAR_SPACER"
    );
}
