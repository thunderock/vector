//! Mux ID newtypes (D-67).

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PaneId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TabId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

/// Monotonic u64 allocator. Mux owns one per ID kind.
#[derive(Debug, Default)]
pub struct IdAllocator {
    next: AtomicU64,
}

impl IdAllocator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            next: AtomicU64::new(1),
        }
    }
    pub fn allocate_pane(&self) -> PaneId {
        PaneId(self.next.fetch_add(1, Ordering::Relaxed))
    }
    pub fn allocate_tab(&self) -> TabId {
        TabId(self.next.fetch_add(1, Ordering::Relaxed))
    }
    pub fn allocate_window(&self) -> WindowId {
        WindowId(self.next.fetch_add(1, Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::{IdAllocator, PaneId, TabId};

    #[test]
    fn ids_are_distinct_and_monotonic() {
        let a = IdAllocator::new();
        assert_eq!(a.allocate_pane(), PaneId(1));
        assert_eq!(a.allocate_pane(), PaneId(2));
        assert_eq!(a.allocate_tab(), TabId(3));
    }
}
