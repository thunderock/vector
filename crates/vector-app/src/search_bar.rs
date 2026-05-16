//! POLISH-06 / D-76 / D-77 — search bar state machine + smart-case + 1000-cap cache.
//!
//! D-76: Cmd-F opens, Esc closes + restores prior selection.
//! D-77: smart-case (case-insensitive when query all-lowercase; case-sensitive when any
//!       uppercase). Always-regex; non-regex chars work as literal patterns.
//!       Up to 1000 matches cached; beyond shows `1000+` lazy step.

use regex::Regex;
use vector_input::SelectionRange;
use vector_term::{Match, Term};

pub const MAX_CACHED_MATCHES: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchOverflow {
    Bounded(usize),
    OverThousand,
}

pub struct MatchCache {
    matches: Vec<Match>,
    pub overflow: MatchOverflow,
    pub active_idx: usize,
}

impl MatchCache {
    #[must_use]
    pub fn from_matches(mut all: Vec<Match>) -> Self {
        let overflow = if all.len() > MAX_CACHED_MATCHES {
            all.truncate(MAX_CACHED_MATCHES);
            MatchOverflow::OverThousand
        } else {
            MatchOverflow::Bounded(all.len())
        };
        Self {
            matches: all,
            overflow,
            active_idx: 0,
        }
    }

    pub fn next(&mut self) {
        if !self.matches.is_empty() {
            self.active_idx = (self.active_idx + 1) % self.matches.len();
        }
    }

    pub fn prev(&mut self) {
        if !self.matches.is_empty() {
            self.active_idx = if self.active_idx == 0 {
                self.matches.len() - 1
            } else {
                self.active_idx - 1
            };
        }
    }

    #[must_use]
    pub fn counter(&self) -> (usize, MatchOverflow) {
        let idx = if self.matches.is_empty() {
            0
        } else {
            self.active_idx + 1
        };
        (idx, self.overflow.clone())
    }

    #[must_use]
    pub fn matches(&self) -> &[Match] {
        &self.matches
    }
}

/// D-77 smart-case: all-lowercase → case-insensitive; any uppercase → case-sensitive.
/// Always-regex; falls back to escaped literal pattern if the query is not a valid regex.
#[must_use]
pub fn smart_case_regex(query: &str) -> Regex {
    let has_upper = query.chars().any(char::is_uppercase);
    let pattern = if has_upper {
        query.to_owned()
    } else {
        format!("(?i){query}")
    };
    Regex::new(&pattern).unwrap_or_else(|_| {
        let escaped = regex::escape(query);
        let fallback = if has_upper {
            escaped
        } else {
            format!("(?i){escaped}")
        };
        Regex::new(&fallback).expect("escaped pattern always compiles")
    })
}

#[derive(Default)]
pub struct SearchBar {
    pub query: String,
    pub cache: Option<MatchCache>,
    pub saved_selection: Option<SelectionRange>,
    pub open: bool,
}

impl SearchBar {
    pub fn open_with(&mut self, prior_selection: Option<SelectionRange>) {
        self.open = true;
        self.saved_selection = prior_selection;
        self.query.clear();
        self.cache = None;
    }

    /// Closes the bar; returns the saved selection so the caller can restore it.
    pub fn close(&mut self) -> Option<SelectionRange> {
        self.open = false;
        self.query.clear();
        self.cache = None;
        self.saved_selection.take()
    }

    pub fn set_query(&mut self, q: &str, term: &Term) {
        q.clone_into(&mut self.query);
        if q.is_empty() {
            self.cache = None;
            return;
        }
        let re = smart_case_regex(q);
        let matches = term.search(&re);
        self.cache = Some(MatchCache::from_matches(matches));
    }
}
