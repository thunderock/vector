//! POLISH-07 / D-75 / UI-SPEC §5.3 — Cmd-Shift-P profile picker.

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use vector_config::Kind;

#[derive(Debug, Clone)]
pub struct PickerEntry {
    pub name: String,
    pub kind: Kind,
}

/// D-75: fuzzy-rank profile names. Returns matched entries (highest score first).
#[must_use]
pub fn match_profiles<'a>(entries: &'a [PickerEntry], query: &str) -> Vec<&'a PickerEntry> {
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, &PickerEntry)> = entries
        .iter()
        .filter_map(|e| matcher.fuzzy_match(&e.name, query).map(|s| (s, e)))
        .collect();
    scored.sort_unstable_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, e)| e).collect()
}

pub struct ProfilePicker {
    pub entries: Vec<PickerEntry>,
    pub query: String,
    pub filtered: Vec<usize>,
    pub selected_idx: usize,
    pub open: bool,
}

impl ProfilePicker {
    #[must_use]
    pub fn new(entries: Vec<PickerEntry>) -> Self {
        let n = entries.len();
        Self {
            entries,
            query: String::new(),
            filtered: (0..n).collect(),
            selected_idx: 0,
            open: false,
        }
    }

    pub fn open(&mut self) {
        self.open = true;
        self.query.clear();
        self.filtered = (0..self.entries.len()).collect();
        self.selected_idx = 0;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn set_query(&mut self, q: &str) {
        q.clone_into(&mut self.query);
        if q.is_empty() {
            self.filtered = (0..self.entries.len()).collect();
        } else {
            let refs = match_profiles(&self.entries, q);
            self.filtered = refs
                .into_iter()
                .filter_map(|e| self.entries.iter().position(|x| x.name == e.name))
                .collect();
        }
        self.selected_idx = 0;
    }

    /// UI-SPEC §5.3: Codespace + DevTunnel rows show `Phase 6+` suffix.
    #[must_use]
    pub fn row_label(&self, filtered_idx: usize) -> String {
        let entry = &self.entries[self.filtered[filtered_idx]];
        match entry.kind {
            Kind::Local => entry.name.clone(),
            Kind::Codespace | Kind::DevTunnel => format!("{}  Phase 6+", entry.name),
        }
    }

    #[must_use]
    pub fn select_active(&self) -> Option<&PickerEntry> {
        self.filtered.get(self.selected_idx).map(|&i| &self.entries[i])
    }
}
