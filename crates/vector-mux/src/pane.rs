//! Pane + PaneNode + SplitRatio (D-67 recursive binary split tree).
//!
//! `PaneNode` leaves hold a `PaneId` (NOT `Arc<Pane>`) so the tree can be
//! mutated without taking pane state locks. Pane state is fetched separately
//! from `Mux.panes` keyed by id.

use std::os::fd::RawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::ids::PaneId;
use crate::transport::{PtyTransport, TransportKind};

/// Plan 09-04 — pane UI state for `format_tab_title` badge selection.
/// `Active` keeps the existing `[remote]` badge; `Reconnecting` swaps it to
/// `[reconnecting]`. Local panes ignore this variant (no badge either way).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneUiState {
    Active,
    Reconnecting,
}

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
    /// Plan 07-04 / CS-06 — cached `TransportKind` captured at Pane construction
    /// time, so `format_tab_title` and tint helpers can read it after
    /// `take_transport()` has handed the live transport to the pty_actor router.
    pub transport_kind: TransportKind,
    pub pid: Option<i32>,
    pub master_fd: Option<RawFd>,
    /// Updated by Plan 04-03 proc_tracker (D-57).
    pub last_proc_name: Mutex<String>,
    /// D-79 OSC 7 consumer: synced from `Term::cwd_ring().back()` after each
    /// PtyOutput drain. Used by `spawn_cwd_for` for child-pane cwd inheritance
    /// and by `format_tab_title` for the cwd-stem suffix.
    pub cwd: Mutex<Option<PathBuf>>,
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
        let transport_kind = transport.kind();
        Self {
            id,
            term,
            transport: Mutex::new(Some(transport)),
            transport_kind,
            pid,
            master_fd,
            last_proc_name: Mutex::new(String::new()),
            cwd: Mutex::new(None),
            exited: AtomicBool::new(false),
        }
    }

    /// Plan 07-04 / CS-06 — cached transport kind (still readable after
    /// `take_transport()` moved the transport into the pty_actor router).
    #[must_use]
    pub fn transport_kind(&self) -> TransportKind {
        self.transport_kind
    }

    /// One-shot transport handoff. Plan 04-03 pty_actor router calls this.
    /// Subsequent calls return None.
    pub fn take_transport(&self) -> Option<Box<dyn PtyTransport>> {
        self.transport.lock().take()
    }

    /// Child shell PID (None for non-local transports or after wait()).
    #[must_use]
    pub fn shell_pid(&self) -> Option<i32> {
        self.pid
    }

    /// Master PTY fd (None for non-local transports).
    #[must_use]
    pub fn master_fd(&self) -> Option<RawFd> {
        self.master_fd
    }
}

/// Lightweight read-only view of the Pane fields needed for cwd resolution.
/// Lets tests + plan-05-10 wiring share `spawn_cwd_for*` without constructing
/// a full Pane (which needs a Term + transport).
#[derive(Debug, Clone)]
pub struct PaneCwdView {
    pub cwd: Option<PathBuf>,
    pub pid: Option<i32>,
}

impl From<&Pane> for PaneCwdView {
    fn from(p: &Pane) -> Self {
        Self {
            cwd: p.cwd.lock().clone(),
            pid: p.pid,
        }
    }
}

/// D-79 B2 fix: resolve cwd for a new-pane / new-tab spawn.
/// Precedence: OSC 7 ring back (pane.cwd) → proc_pidinfo fallback (D-63) → `$HOME`.
#[must_use]
pub fn spawn_cwd_for(view: &PaneCwdView) -> PathBuf {
    spawn_cwd_for_with_proc(
        view,
        |pid| crate::cwd::pidcwd(pid).ok(),
        || std::env::var_os("HOME").map(PathBuf::from),
    )
}

/// Test seam variant: callers inject the proc_pidinfo + HOME lookups.
#[must_use]
pub fn spawn_cwd_for_with_proc(
    view: &PaneCwdView,
    proc_fn: impl Fn(i32) -> Option<PathBuf>,
    home_fn: impl Fn() -> Option<PathBuf>,
) -> PathBuf {
    if let Some(cwd) = &view.cwd {
        return cwd.clone();
    }
    if let Some(pid) = view.pid {
        if let Some(cwd) = proc_fn(pid) {
            return cwd;
        }
    }
    home_fn().unwrap_or_else(|| PathBuf::from("/"))
}

/// D-79 B2 fix: tab title with cwd-stem suffix when OSC 7 is present.
/// Plan 07-04 (CS-06): appends ` [remote]` for non-Local transports.
/// Returns `"zsh: vector"` when cwd=`/Users/me/vector`; `"zsh"` when cwd=None.
#[must_use]
pub fn format_tab_title(
    process_name: &str,
    cwd: Option<&Path>,
    kind: crate::transport::TransportKind,
    ui_state: PaneUiState,
) -> String {
    let base = match cwd.and_then(Path::file_name).and_then(|s| s.to_str()) {
        Some(stem) if !stem.is_empty() => format!("{process_name}: {stem}"),
        _ => process_name.to_owned(),
    };
    match (kind, ui_state) {
        (crate::transport::TransportKind::Local, _) => base,
        (crate::transport::TransportKind::DevTunnel, PaneUiState::Active) => {
            format!("{base} [remote]")
        }
        (crate::transport::TransportKind::DevTunnel, PaneUiState::Reconnecting) => {
            format!("{base} [reconnecting]")
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::TransportKind;
    use std::path::PathBuf;

    #[test]
    fn format_tab_title_remote_appends_suffix() {
        let s = format_tab_title("zsh", None, TransportKind::DevTunnel, PaneUiState::Active);
        assert!(s.ends_with(" [remote]"), "got: {s}");
    }

    #[test]
    fn format_tab_title_local_no_suffix() {
        let s = format_tab_title("zsh", None, TransportKind::Local, PaneUiState::Active);
        assert!(!s.contains("[remote]"), "got: {s}");
    }

    #[test]
    fn format_tab_title_remote_with_cwd() {
        let cwd = PathBuf::from("/Users/me/vector");
        let s = format_tab_title(
            "zsh",
            Some(&cwd),
            TransportKind::DevTunnel,
            PaneUiState::Active,
        );
        assert_eq!(s, "zsh: vector [remote]");
    }

    #[test]
    fn format_tab_title_reconnecting_appends_reconnecting() {
        let s = format_tab_title(
            "zsh",
            None,
            TransportKind::DevTunnel,
            PaneUiState::Reconnecting,
        );
        assert!(s.ends_with(" [reconnecting]"), "got: {s}");
    }

    #[test]
    fn format_tab_title_active_keeps_remote() {
        let s = format_tab_title("zsh", None, TransportKind::DevTunnel, PaneUiState::Active);
        assert!(s.ends_with(" [remote]"), "got: {s}");
        assert!(!s.contains("[reconnecting]"), "got: {s}");
    }

    #[test]
    fn format_tab_title_local_never_emits_reconnecting() {
        let s_active = format_tab_title("zsh", None, TransportKind::Local, PaneUiState::Active);
        let s_reconn =
            format_tab_title("zsh", None, TransportKind::Local, PaneUiState::Reconnecting);
        assert!(!s_active.contains("[reconnecting]"), "got: {s_active}");
        assert!(!s_reconn.contains("[reconnecting]"), "got: {s_reconn}");
        assert!(!s_active.contains("[remote]"), "got: {s_active}");
        assert!(!s_reconn.contains("[remote]"), "got: {s_reconn}");
    }
}
