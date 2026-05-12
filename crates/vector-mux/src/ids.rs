//! Mux ID newtypes (D-67) + mux-level enums introduced by Plan 04-02.

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PaneId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TabId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub u64);

/// Per-kind monotonic u64 allocators. Mux owns one IdAllocator.
#[derive(Debug, Default)]
#[allow(clippy::struct_field_names)]
pub struct IdAllocator {
    next_pane: AtomicU64,
    next_tab: AtomicU64,
    next_window: AtomicU64,
}

impl IdAllocator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_pane: AtomicU64::new(1),
            next_tab: AtomicU64::new(1),
            next_window: AtomicU64::new(1),
        }
    }
    pub fn allocate_pane(&self) -> PaneId {
        PaneId(self.next_pane.fetch_add(1, Ordering::Relaxed))
    }
    pub fn allocate_tab(&self) -> TabId {
        TabId(self.next_tab.fetch_add(1, Ordering::Relaxed))
    }
    pub fn allocate_window(&self) -> WindowId {
        WindowId(self.next_window.fetch_add(1, Ordering::Relaxed))
    }
}

/// Direction of a split mutation (Cmd-D vs Cmd-Shift-D).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// Four-way directional pane focus (D-59 Cmd-Opt-Arrow).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// Decision result from `Mux::close_pane` — caller routes the AppKit side-effect.
#[derive(Debug, PartialEq, Eq)]
pub enum CloseResult {
    PaneClosed { tab_id: TabId },
    TabClosed { window_id: WindowId },
    WindowClosed { window_id: WindowId },
    LastWindowClosed,
}

/// Split mutation errors. `BelowMinimum` enforces the 20x4 cell floor.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum SplitError {
    #[error("split would drop a pane below the minimum {MIN_PANE_COLS}x{MIN_PANE_ROWS} floor")]
    BelowMinimum,
    #[error("pane id not found in tree")]
    PaneNotFound,
}

/// Nudge errors. `BelowMinimumSize` rejects a 1-cell shift if it would shrink either side below the floor.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum NudgeError {
    #[error("nudge would shrink a pane below the minimum size floor")]
    BelowMinimumSize,
    #[error("no ancestor split matches the requested direction's axis")]
    NoSplitInDirection,
}

/// Minimum cell floor enforced on split + nudge (CONTEXT.md Claude's Discretion).
pub const MIN_PANE_COLS: u16 = 20;
pub const MIN_PANE_ROWS: u16 = 4;

#[cfg(test)]
mod tests {
    use super::{IdAllocator, PaneId, TabId, WindowId};

    #[test]
    fn ids_are_distinct_and_monotonic_per_kind() {
        let a = IdAllocator::new();
        assert_eq!(a.allocate_pane(), PaneId(1));
        assert_eq!(a.allocate_pane(), PaneId(2));
        assert_eq!(a.allocate_tab(), TabId(1));
        assert_eq!(a.allocate_tab(), TabId(2));
        assert_eq!(a.allocate_window(), WindowId(1));
    }
}
