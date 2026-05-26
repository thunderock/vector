//! HARDEN-02 Scenario 7: DECSET 2004 bracketed paste mode.
//! Maps to PITFALLS.md Pitfall 8.
//! Mirrors `crates/vector-term/tests/dcs_dispatch.rs::bracketed_paste_mode_2004_sets_state`.

use alacritty_terminal::term::TermMode;
use vector_term::Term;

#[test]
fn bracketed_paste_2004_sets_state() {
    let mut term = Term::new(80, 24, 1000);
    assert!(!term.mode().contains(TermMode::BRACKETED_PASTE));
    term.feed(b"\x1b[?2004h");
    assert!(term.mode().contains(TermMode::BRACKETED_PASTE));
}

#[test]
fn bracketed_paste_2004_reset_clears_state() {
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"\x1b[?2004h");
    term.feed(b"\x1b[?2004l");
    assert!(!term.mode().contains(TermMode::BRACKETED_PASTE));
}
