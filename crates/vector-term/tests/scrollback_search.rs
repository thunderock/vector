//! CORE-03: 10,000+ line scrollback regex search (D-39).

use std::time::Instant;

use regex::Regex;
use vector_term::{Match, Term};

#[test]
fn ten_thousand_lines_regex_search_finds_match() {
    let mut term = Term::new(80, 24, 10_001);
    for i in 0..10_001u32 {
        term.feed(format!("line {i}\r\n").as_bytes());
    }
    let re = Regex::new(r"line 9999").unwrap();
    let matches = term.search(&re);
    assert!(
        !matches.is_empty(),
        "expected at least 1 match for 'line 9999'"
    );
    assert_eq!(matches[0].start_col, 0);
}

#[test]
fn search_api_shape_returns_vec_of_matches() {
    // Compile-time API contract — proves `search(&Regex) -> Vec<Match>` exists.
    let term = Term::new(80, 24, 100);
    let _: Vec<Match> = term.search(&Regex::new(r"x").unwrap());
}

#[test]
fn ten_thousand_line_search_completes_under_one_second() {
    let mut term = Term::new(80, 24, 10_001);
    for i in 0..10_001u32 {
        term.feed(format!("line {i}\r\n").as_bytes());
    }
    let re = Regex::new(r"line \d+").unwrap();
    let start = Instant::now();
    let matches = term.search(&re);
    let elapsed = start.elapsed();
    assert!(
        matches.len() >= 10_000,
        "expected >= 10000 matches, got {}",
        matches.len()
    );
    assert!(
        elapsed.as_millis() < 1000,
        "search took {} ms; Pitfall 7 budget is 1000",
        elapsed.as_millis()
    );
}
