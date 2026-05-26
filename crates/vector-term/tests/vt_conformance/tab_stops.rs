//! HARDEN-02 Scenario 3: tab stops — HTS sets, CHT jumps, TBC clears.
//! Maps to PITFALLS.md Pitfall 1; extracted from decstbm_scroll_region.rs.

use vector_term::Term;

#[test]
fn hts_sets_tab_stop_then_cht_jumps_to_it() {
    let mut term = Term::new(80, 24, 1000);
    // Clear all tab stops (CSI 3 g = TBC all).
    term.feed(b"\x1b[3g");
    // Move to column 16 (1-based) and set tab stop via HTS (ESC H).
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

#[test]
fn tbc_3_clears_all_tab_stops() {
    let mut term = Term::new(80, 24, 1000);
    // Set a tab stop at col 16.
    term.feed(b"\x1b[1;16H\x1bH");
    // Clear all tab stops.
    term.feed(b"\x1b[3g");
    // Home + CHT 1 — with no tab stops left, CHT advances to last column (alacritty
    // convention: cursor stops at right margin when no tab is set).
    term.feed(b"\x1b[1;1H\x1b[1I");
    let (col, _row) = term.cursor();
    assert_ne!(
        col, 15,
        "TBC 3 must clear the HTS-set tab so CHT no longer lands on col 15"
    );
}
