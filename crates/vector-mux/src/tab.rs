//! Tab — owns one `PaneNode` tree + an active-pane pointer + last viewport size.

use crate::ids::{PaneId, TabId};
use crate::pane::PaneNode;

#[derive(Debug)]
pub struct Tab {
    pub id: TabId,
    pub root: PaneNode,
    pub active_pane_id: PaneId,
    /// Last known viewport (cells). Plan 04-03 updates on WindowEvent::Resized.
    pub last_rows: u16,
    pub last_cols: u16,
}

impl Tab {
    #[must_use]
    pub fn new(id: TabId, first_pane: PaneId, rows: u16, cols: u16) -> Self {
        Self {
            id,
            root: PaneNode::Leaf(first_pane),
            active_pane_id: first_pane,
            last_rows: rows,
            last_cols: cols,
        }
    }

    #[must_use]
    pub fn pane_count(&self) -> usize {
        self.root.leaves().len()
    }

    #[must_use]
    pub fn contains(&self, pane_id: PaneId) -> bool {
        self.root.contains(pane_id)
    }
}
