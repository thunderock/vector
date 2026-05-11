//! CORE-01: DECSTBM scroll regions + tab stops (HTS, CHT, TBC).

use alacritty_terminal::index::{Column, Line, Point};
use vector_term::Term;

#[test]
fn decstbm_constrains_scroll_to_region() {
    let mut term = Term::new(80, 24, 1000);
    // Pre-mark row 1 (outside region) and row 23 (outside region).
    term.feed(b"\x1b[2;1H"); // row 2 (0-based row 1)
    term.feed(b"TOP_OUTSIDE");
    term.feed(b"\x1b[24;1H"); // row 24 (0-based row 23)
    term.feed(b"BOT_OUTSIDE");
    // Set scroll region rows 5..10 (1-based).
    term.feed(b"\x1b[5;10r");
    // Move into the region and emit lines that force scrolling within it.
    term.feed(b"\x1b[10;1H"); // 0-based row 9 (bottom of region)
    for i in 0..20 {
        term.feed(format!("region_line_{i}\n").as_bytes());
    }
    // Row 1 (TOP_OUTSIDE) and row 23 (BOT_OUTSIDE) must be untouched.
    let top = &term.grid()[Point::new(Line(1), Column(0))];
    let bot = &term.grid()[Point::new(Line(23), Column(0))];
    assert_eq!(top.c, 'T', "row above DECSTBM region must not scroll");
    assert_eq!(bot.c, 'B', "row below DECSTBM region must not scroll");
}

#[test]
fn hts_sets_tab_stop_then_cht_jumps_to_it() {
    let mut term = Term::new(80, 24, 1000);
    // Clear all tab stops (CSI 3 g).
    term.feed(b"\x1b[3g");
    // Move to column 16 (1-based 16 = 0-based 15) and set tab stop (HTS = ESC H).
    term.feed(b"\x1b[1;16H\x1bH");
    // Home then CHT 1 — should jump to the tab stop we set.
    term.feed(b"\x1b[1;1H\x1b[1I");
    let (col, row) = term.cursor();
    assert_eq!(row, 0);
    assert_eq!(
        col, 15,
        "CHT must jump to tab stop set by HTS at col 15 (0-based)"
    );
}
