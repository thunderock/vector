//! POLISH-04 D-78 / Pitfall 4 — OSC 8 hyperlink scheme allowlist + per-row grouping.

use vector_term::hyperlink::{group_row, is_allowed_scheme};

fn cell(col: usize, uri: &str, id: Option<&str>) -> (usize, Option<(String, Option<String>)>) {
    (col, Some((uri.to_owned(), id.map(String::from))))
}

#[test]
fn id_groups_run() {
    let cells: Vec<_> = (0..5)
        .map(|c| cell(c, "https://x.com", Some("foo")))
        .collect();
    let runs = group_row(0, cells);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].col_start, 0);
    assert_eq!(runs[0].col_end, 5);
    assert_eq!(runs[0].id.as_deref(), Some("foo"));
}

#[test]
fn anonymous_by_uri() {
    let cells = vec![
        cell(0, "https://a.com", None),
        cell(1, "https://a.com", None),
        cell(2, "https://b.com", None),
        cell(3, "https://b.com", None),
    ];
    let runs = group_row(0, cells);
    assert_eq!(
        runs.len(),
        2,
        "Pitfall 4: anonymous links grouped by URI + contiguity, NOT merged"
    );
    assert_eq!(runs[0].uri, "https://a.com");
    assert_eq!(runs[1].uri, "https://b.com");
}

#[test]
fn scheme_allowlist() {
    assert!(is_allowed_scheme("https://x.com"));
    assert!(is_allowed_scheme("http://x.com"));
    assert!(is_allowed_scheme("mailto:user@host"));
    assert!(is_allowed_scheme("file:///etc/passwd"));
    assert!(!is_allowed_scheme("gopher://x"));
    assert!(!is_allowed_scheme("javascript:alert(1)"));
    assert!(!is_allowed_scheme("data:text/html,<script>"));
}
