//! Window — owns a `Vec<Tab>` directly (Tabs are not shared across Windows).

use crate::ids::{TabId, WindowId};
use crate::tab::Tab;

#[derive(Debug)]
pub struct Window {
    pub id: WindowId,
    pub tabs: Vec<Tab>,
    pub active_tab_id: Option<TabId>,
}

impl Window {
    #[must_use]
    pub fn new(id: WindowId) -> Self {
        Self {
            id,
            tabs: Vec::new(),
            active_tab_id: None,
        }
    }

    #[must_use]
    pub fn active_tab(&self) -> Option<&Tab> {
        let id = self.active_tab_id?;
        self.tabs.iter().find(|t| t.id == id)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        let id = self.active_tab_id?;
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    /// Cmd-Shift-] — advance active tab with wrap.
    pub fn cycle_next(&mut self) {
        let Some(active) = self.active_tab_id else {
            return;
        };
        if self.tabs.len() <= 1 {
            return;
        }
        if let Some(idx) = self.tabs.iter().position(|t| t.id == active) {
            let next = (idx + 1) % self.tabs.len();
            self.active_tab_id = Some(self.tabs[next].id);
        }
    }

    /// Cmd-Shift-[ — retreat active tab with wrap.
    pub fn cycle_prev(&mut self) {
        let Some(active) = self.active_tab_id else {
            return;
        };
        if self.tabs.len() <= 1 {
            return;
        }
        if let Some(idx) = self.tabs.iter().position(|t| t.id == active) {
            let prev = if idx == 0 {
                self.tabs.len() - 1
            } else {
                idx - 1
            };
            self.active_tab_id = Some(self.tabs[prev].id);
        }
    }
}
