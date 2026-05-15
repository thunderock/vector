//! POLISH-06 / D-76 / D-77 — search bar logic: smart-case + 1000-cap cache + esc-restore.

use vector_app::search_bar::{
    smart_case_regex, MatchCache, MatchOverflow, SearchBar, MAX_CACHED_MATCHES,
};

#[test]
fn smart_case_lower() {
    let re = smart_case_regex("hello");
    assert!(
        re.is_match("Hello"),
        "all-lowercase query → case-insensitive"
    );
    assert!(re.is_match("HELLO"));
    assert!(re.is_match("hello"));
}

#[test]
fn smart_case_upper() {
    let re = smart_case_regex("Hello");
    assert!(re.is_match("Hello"));
    assert!(
        !re.is_match("hello"),
        "any uppercase → case-sensitive (D-77)"
    );
    assert!(!re.is_match("HELLO"));
}

#[test]
fn cache_1000_lazy() {
    let dummy = vector_term::Match {
        start_row: 0,
        start_col: 0,
        end_row: 0,
        end_col: 1,
    };
    let v = vec![dummy; 1500];
    let cache = MatchCache::from_matches(v);
    assert_eq!(
        cache.overflow,
        MatchOverflow::OverThousand,
        "D-77: overflow flag set at >1000"
    );
    assert_eq!(
        cache.matches().len(),
        MAX_CACHED_MATCHES,
        "cache truncates to 1000"
    );
    let (idx, of) = cache.counter();
    assert_eq!(idx, 1);
    assert_eq!(of, MatchOverflow::OverThousand);
}

#[test]
fn esc_restores_selection() {
    let mut bar = SearchBar::default();
    let prior = vector_input::SelectionRange {
        anchor: (0, 0),
        cursor: (1, 0),
    };
    bar.open_with(Some(prior));
    assert!(bar.open);
    let returned = bar.close().expect("Esc must return the prior selection");
    assert_eq!(returned, prior);
    assert!(!bar.open);
}
