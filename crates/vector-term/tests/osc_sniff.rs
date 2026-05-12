//! POLISH-04 D-79 / Pitfall 3 — OSC 7 (cwd) + OSC 133 (prompt marks) sniffer.

use vector_term::{osc_sniff::PromptKind, Term};

fn make_term() -> Term {
    Term::new(80, 24, 1000)
}

#[test]
fn osc7_file_url_parses() {
    let mut t = make_term();
    t.feed(b"\x1b]7;file://localhost/Users/foo/dev/\x07");
    let cwd = t.cwd_ring().back().expect("cwd captured");
    assert_eq!(cwd, &std::path::PathBuf::from("/Users/foo/dev/"));
}

#[test]
fn osc7_percent_encoded() {
    let mut t = make_term();
    t.feed(b"\x1b]7;file://localhost/Users/foo/dev%20space/\x07");
    let cwd = t.cwd_ring().back().expect("cwd captured");
    assert_eq!(cwd, &std::path::PathBuf::from("/Users/foo/dev space/"));
}

#[test]
fn osc133_marks() {
    let mut t = make_term();
    t.feed(b"\x1b]133;A\x07");
    t.feed(b"\x1b]133;C\x07");
    t.feed(b"\x1b]133;D;0\x07");
    let marks: Vec<_> = t.prompt_marks().iter().collect();
    assert_eq!(marks.len(), 3);
    assert_eq!(marks[0].kind, PromptKind::Start);
    assert_eq!(marks[1].kind, PromptKind::Output);
    assert_eq!(marks[2].kind, PromptKind::End);
    assert_eq!(marks[2].exit_code, Some(0));
}

#[test]
fn prompt_ring_1000() {
    let mut t = make_term();
    for _ in 0..1100 {
        t.feed(b"\x1b]133;A\x07");
    }
    assert_eq!(t.prompt_marks().len(), 1000, "D-79: ring caps at 1000");
}
