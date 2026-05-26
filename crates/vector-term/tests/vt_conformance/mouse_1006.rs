//! HARDEN-02 Scenario 5: DECSET 1000 + 1006 mouse mode flags.
//! Maps to PITFALLS.md Pitfall 8 (mouse-mode interaction with tmux); D-07 ROADMAP criterion.
//! Mirrors `crates/vector-term/tests/dcs_dispatch.rs::mouse_mode_1006_sgr_sets_state`.

use alacritty_terminal::term::TermMode;
use vector_term::Term;

#[test]
fn mouse_1006_sgr_sets_state() {
    let mut term = Term::new(80, 24, 1000);
    assert!(!term.mode().contains(TermMode::SGR_MOUSE));
    assert!(!term.mode().contains(TermMode::MOUSE_REPORT_CLICK));
    term.feed(b"\x1b[?1000h\x1b[?1006h");
    assert!(
        term.mode().contains(TermMode::MOUSE_REPORT_CLICK),
        "DECSET 1000 must enable click reporting"
    );
    assert!(
        term.mode().contains(TermMode::SGR_MOUSE),
        "DECSET 1006 must enable SGR encoding"
    );
}

#[test]
fn mouse_1006_sgr_reset_clears_state() {
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"\x1b[?1000h\x1b[?1006h");
    term.feed(b"\x1b[?1006l\x1b[?1000l");
    assert!(
        !term.mode().contains(TermMode::SGR_MOUSE),
        "DECRST 1006 must disable SGR encoding"
    );
    assert!(
        !term.mode().contains(TermMode::MOUSE_REPORT_CLICK),
        "DECRST 1000 must disable click reporting"
    );
}
