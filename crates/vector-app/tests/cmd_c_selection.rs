//! Plan 05-11 — Cmd-C selection-string extraction (gap #5).
//! Drives `selection_to_string` against a real `vector_term::Term` via the
//! `TermGridAccess` newtype defined in `vector-app` (B1 fix: lives here to
//! avoid a vector-input -> vector-mux -> vector-term -> vector-input cycle).

use vector_app::term_grid_access::TermGridAccess;
use vector_input::{selection_to_string, SelectionMode, SelectionRange};
use vector_term::Term;

#[test]
fn selects_basic_word() {
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"hello world");
    let range = SelectionRange {
        anchor: (0, 0),
        cursor: (4, 0),
    };
    let text = selection_to_string(&range, &TermGridAccess(&term), SelectionMode::Stream);
    assert_eq!(text, "hello");
}

#[test]
fn strips_trailing_whitespace_per_row() {
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"foo");
    // Select the entire first row [0..80) — trailing blanks must be stripped.
    let range = SelectionRange {
        anchor: (0, 0),
        cursor: (79, 0),
    };
    let text = selection_to_string(&range, &TermGridAccess(&term), SelectionMode::Stream);
    assert_eq!(text, "foo");
}

#[test]
fn skips_wide_char_spacer() {
    let mut term = Term::new(80, 24, 1000);
    // U+3042 HIRAGANA LETTER A — full-width; occupies cols 0 and 1 (cell 1 = WIDE_CHAR_SPACER).
    term.feed("あ".as_bytes());
    let range = SelectionRange {
        anchor: (0, 0),
        cursor: (1, 0),
    };
    let text = selection_to_string(&range, &TermGridAccess(&term), SelectionMode::Stream);
    assert_eq!(text, "あ", "wide-char spacer must be skipped (Pitfall 8)");
    assert_eq!(text.chars().count(), 1);
}
