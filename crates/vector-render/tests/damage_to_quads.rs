//! Plan 03-03 Task 1: smoke that Term reports damage after `feed`. Task 2 upgrades to a
//! pixel-level offscreen render assertion.

#[test]
fn term_reports_damage_after_feed() {
    let mut term = vector_term::Term::new(40, 10, 1_000);
    term.feed(b"\x1b[31mA\x1b[0m");
    match term.damage() {
        vector_term::TermDamage::Full => {}
        vector_term::TermDamage::Partial(iter) => {
            assert!(iter.count() > 0, "expected at least one damaged row");
        }
    }
}
