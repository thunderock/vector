//! Pane + PaneNode + SplitRatio (D-67 recursive binary split tree).
//!
//! `PaneNode` leaves hold a `PaneId` (NOT `Arc<Pane>`) so the tree can be
//! mutated without taking pane state locks. Pane state is fetched separately
//! from `Mux.panes` keyed by id.

use std::os::fd::RawFd;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::ids::PaneId;
use crate::transport::PtyTransport;

/// Cell-count storage for split proportions (D-60 / D-67).
/// `first + second + 1 (divider) == axis_size_in_cells` is the invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitRatio {
    pub first: u16,
    pub second: u16,
}

/// Recursive binary split tree.
#[derive(Debug)]
pub enum PaneNode {
    Leaf(PaneId),
    HSplit {
        left: Box<PaneNode>,
        right: Box<PaneNode>,
        ratio: SplitRatio,
    },
    VSplit {
        top: Box<PaneNode>,
        bottom: Box<PaneNode>,
        ratio: SplitRatio,
    },
}

impl PaneNode {
    #[must_use]
    pub fn is_leaf(&self) -> bool {
        matches!(self, Self::Leaf(_))
    }

    /// Depth-first collect of all leaf PaneIds in left/top -> right/bottom order.
    #[must_use]
    pub fn leaves(&self) -> Vec<PaneId> {
        let mut out = Vec::new();
        self.collect_leaves(&mut out);
        out
    }

    fn collect_leaves(&self, out: &mut Vec<PaneId>) {
        match self {
            Self::Leaf(id) => out.push(*id),
            Self::HSplit { left, right, .. } => {
                left.collect_leaves(out);
                right.collect_leaves(out);
            }
            Self::VSplit { top, bottom, .. } => {
                top.collect_leaves(out);
                bottom.collect_leaves(out);
            }
        }
    }

    /// True if any leaf in this subtree carries `target`.
    #[must_use]
    pub fn contains(&self, target: PaneId) -> bool {
        match self {
            Self::Leaf(id) => *id == target,
            Self::HSplit { left, right, .. } => left.contains(target) || right.contains(target),
            Self::VSplit { top, bottom, .. } => top.contains(target) || bottom.contains(target),
        }
    }
}

/// Per-pane runtime state. Plan 04-02 ships fields; Plan 04-03 wires the
/// pty_actor router to call `take_transport()` and own the transport thereafter.
pub struct Pane {
    pub id: PaneId,
    pub term: Arc<Mutex<vector_term::Term>>,
    /// Transport ownership bridge for Plan 04-03 (pty_actor router takes it).
    /// `Mutex<Option<...>>` so it can be moved out without &mut Pane.
    pub transport: Mutex<Option<Box<dyn PtyTransport>>>,
    pub pid: Option<i32>,
    pub master_fd: Option<RawFd>,
    /// Updated by Plan 04-03 proc_tracker (D-57).
    pub last_proc_name: Mutex<String>,
    /// Flipped by pty_actor on transport.wait() completion.
    pub exited: AtomicBool,
}

impl Pane {
    /// Construct a Pane from a freshly-spawned transport.
    #[must_use]
    pub fn new(
        id: PaneId,
        term: Arc<Mutex<vector_term::Term>>,
        transport: Box<dyn PtyTransport>,
        pid: Option<i32>,
        master_fd: Option<RawFd>,
    ) -> Self {
        Self {
            id,
            term,
            transport: Mutex::new(Some(transport)),
            pid,
            master_fd,
            last_proc_name: Mutex::new(String::new()),
            exited: AtomicBool::new(false),
        }
    }

    /// One-shot transport handoff. Plan 04-03 pty_actor router calls this.
    /// Subsequent calls return None.
    pub fn take_transport(&self) -> Option<Box<dyn PtyTransport>> {
        self.transport.lock().take()
    }
}

impl std::fmt::Debug for Pane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pane")
            .field("id", &self.id)
            .field("pid", &self.pid)
            .field("master_fd", &self.master_fd)
            .field(
                "exited",
                &self.exited.load(std::sync::atomic::Ordering::Relaxed),
            )
            .finish_non_exhaustive()
    }
}
