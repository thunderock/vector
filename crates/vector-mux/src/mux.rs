//! Mux singleton (D-67). Owns windows + panes + ID allocator + default domain.

use std::collections::HashMap;
use std::os::fd::RawFd;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use anyhow::Result;
use parking_lot::{Mutex, RwLock};

use crate::cwd::inherit_cwd;
use crate::domain::SpawnCommand;
use crate::ids::{
    CloseResult, Direction, IdAllocator, NudgeError, PaneId, SplitDirection, SplitError, TabId,
    WindowId, MIN_PANE_COLS, MIN_PANE_ROWS,
};
use crate::local_domain::LocalDomain;
use crate::pane::{Pane, PaneNode};
use crate::split_tree::{self, Rect};
use crate::tab::Tab;
use crate::window::Window;

static MUX: OnceLock<Arc<Mux>> = OnceLock::new();

pub struct Mux {
    windows: RwLock<HashMap<WindowId, Window>>,
    panes: RwLock<HashMap<PaneId, Arc<Pane>>>,
    ids: IdAllocator,
    /// Phase 4 only; Phase 7 will add CodespaceDomain etc.
    #[allow(dead_code)]
    default_domain: Arc<LocalDomain>,
}

impl Mux {
    #[must_use]
    pub fn new(default_domain: Arc<LocalDomain>) -> Arc<Self> {
        Arc::new(Self {
            windows: RwLock::new(HashMap::new()),
            panes: RwLock::new(HashMap::new()),
            ids: IdAllocator::new(),
            default_domain,
        })
    }

    /// Install the global Mux singleton. Panics on second call.
    pub fn install(mux: Arc<Mux>) {
        MUX.set(mux).ok().expect("Mux::install called twice");
    }

    /// Fetch the global singleton. Panics if `install` was never called.
    #[must_use]
    pub fn get() -> Arc<Mux> {
        MUX.get().cloned().expect("Mux::install not called yet")
    }

    /// Fallible variant of `get` — returns `None` if `install` hasn't been called.
    /// Plan 04-04 callers (App's Cmd-* handlers) probe the singleton without panicking
    /// because the App can be running before the I/O thread completes Mux setup.
    #[must_use]
    pub fn try_get() -> Option<Arc<Mux>> {
        MUX.get().cloned()
    }

    /// Plan 04-04: snapshot of all currently-registered `WindowId`s.
    #[must_use]
    pub fn window_ids_snapshot(&self) -> Vec<WindowId> {
        self.windows.read().keys().copied().collect()
    }

    /// Plan 04-04: the first active PaneId observed across all windows. Returns
    /// the active pane of the active tab of an arbitrary window; `None` if mux
    /// is empty. Multi-window disambiguation by key-NSWindow lands in Plan 04-05.
    #[must_use]
    pub fn any_active_pane_id(&self) -> Option<PaneId> {
        let windows = self.windows.read();
        for w in windows.values() {
            if let Some(tab_id) = w.active_tab_id {
                if let Some(tab) = w.tabs.iter().find(|t| t.id == tab_id) {
                    return Some(tab.active_pane_id);
                }
            }
        }
        None
    }

    pub fn allocate_pane_id(&self) -> PaneId {
        self.ids.allocate_pane()
    }
    pub fn allocate_tab_id(&self) -> TabId {
        self.ids.allocate_tab()
    }
    pub fn allocate_window_id(&self) -> WindowId {
        self.ids.allocate_window()
    }

    /// Insert a brand-new empty Window.
    pub fn create_window(&self) -> WindowId {
        let id = self.ids.allocate_window();
        self.windows.write().insert(id, Window::new(id));
        id
    }

    /// Phase-4-internal: install a pre-constructed Pane as the first leaf of a new tab.
    /// Plan 04-03 wraps this in an async helper that drives `LocalDomain::spawn_local`.
    pub fn install_tab(
        &self,
        window_id: WindowId,
        pane: Arc<Pane>,
        rows: u16,
        cols: u16,
    ) -> (TabId, PaneId) {
        let pane_id = pane.id;
        let tab_id = self.ids.allocate_tab();
        {
            let mut panes = self.panes.write();
            panes.insert(pane_id, pane);
        }
        let mut windows = self.windows.write();
        let window = windows
            .get_mut(&window_id)
            .expect("install_tab: window_id not found");
        window.tabs.push(Tab::new(tab_id, pane_id, rows, cols));
        window.active_tab_id = Some(tab_id);
        (tab_id, pane_id)
    }

    /// Mutate the tab containing `pane_id`: bisect the leaf into a new split,
    /// register `new_pane` in `self.panes`, mark new pane active.
    pub fn split_pane(
        &self,
        pane_id: PaneId,
        dir: SplitDirection,
        new_pane: Arc<Pane>,
    ) -> Result<PaneId, SplitError> {
        let new_pane_id = new_pane.id;
        let (window_id, tab_id) = self.locate_pane(pane_id).ok_or(SplitError::PaneNotFound)?;
        let mut windows = self.windows.write();
        let window = windows
            .get_mut(&window_id)
            .expect("split_pane: window gone");
        let tab = window
            .tabs
            .iter_mut()
            .find(|t| t.id == tab_id)
            .expect("split_pane: tab gone");
        let viewport = Rect {
            x: 0,
            y: 0,
            w: tab.last_cols,
            h: tab.last_rows,
        };
        let prev_root = std::mem::replace(&mut tab.root, PaneNode::Leaf(pane_id));
        let new_root =
            match split_tree::split_at_leaf(prev_root, pane_id, new_pane_id, dir, viewport) {
                Ok(n) => n,
                Err(e) => {
                    // Failed split — original tree was moved out; the simplest correct
                    // restoration is to recompute by reapplying the same shape isn't
                    // possible since prev_root is gone. Reconstruct via the leaves we know.
                    // Practically the call sites pre-check viable size, but to keep the
                    // function total, rebuild a Leaf root with the original pane and
                    // surface the error.
                    tab.root = PaneNode::Leaf(pane_id);
                    return Err(e);
                }
            };
        tab.root = new_root;
        tab.active_pane_id = new_pane_id;
        drop(windows);
        self.panes.write().insert(new_pane_id, new_pane);
        Ok(new_pane_id)
    }

    /// Cmd-Shift-]/[ — cycle active tab in the window.
    /// `Direction::Right` -> next; `Direction::Left` -> prev; Up/Down -> no-op.
    pub fn cycle_tab(&self, window_id: WindowId, dir: Direction) {
        let mut windows = self.windows.write();
        let Some(window) = windows.get_mut(&window_id) else {
            return;
        };
        match dir {
            Direction::Right => window.cycle_next(),
            Direction::Left => window.cycle_prev(),
            Direction::Up | Direction::Down => {}
        }
    }

    /// D-61 cascade decision. Mutates topology; does NOT shut down the transport
    /// (Plan 04-03 pty_actor handles that on its own). Returns the cascade outcome
    /// for the App layer to route side-effects (drop winit Window, exit loop).
    pub fn close_pane(&self, pane_id: PaneId) -> CloseResult {
        let Some((window_id, tab_id)) = self.locate_pane(pane_id) else {
            // Treat unknown pane as already-gone: report as last-window-closed iff empty.
            return if self.windows.read().is_empty() {
                CloseResult::LastWindowClosed
            } else {
                CloseResult::PaneClosed { tab_id: TabId(0) }
            };
        };
        let result = {
            let mut windows = self.windows.write();
            let window = windows
                .get_mut(&window_id)
                .expect("close_pane: window gone");
            let tab_idx = window
                .tabs
                .iter()
                .position(|t| t.id == tab_id)
                .expect("close_pane: tab gone");
            let tab = &mut window.tabs[tab_idx];

            // Step 1: try to collapse within the tab.
            let prev_root = std::mem::replace(&mut tab.root, PaneNode::Leaf(pane_id));
            if let Some(new_root) = split_tree::remove_leaf(prev_root, pane_id) {
                // Pane left the tree; sibling absorbs the space.
                let new_active = *new_root
                    .leaves()
                    .first()
                    .expect("post-remove tree must have ≥1 leaf");
                tab.root = new_root;
                tab.active_pane_id = new_active;
                CloseResult::PaneClosed { tab_id }
            } else {
                // Tab is empty — drop the tab.
                window.tabs.remove(tab_idx);
                if window.tabs.is_empty() {
                    let window_was_only = windows.len() == 1;
                    windows.remove(&window_id);
                    if window_was_only {
                        CloseResult::LastWindowClosed
                    } else {
                        CloseResult::WindowClosed { window_id }
                    }
                } else {
                    let new_idx = tab_idx.min(window.tabs.len() - 1);
                    window.active_tab_id = Some(window.tabs[new_idx].id);
                    CloseResult::TabClosed { window_id }
                }
            }
        };
        // Drop the pane from the pane registry.
        self.panes.write().remove(&pane_id);
        result
    }

    /// D-59 directional focus delegated to the algorithm in split_tree.
    #[must_use]
    pub fn focus_direction(&self, from: PaneId, dir: Direction) -> Option<PaneId> {
        let (window_id, tab_id) = self.locate_pane(from)?;
        let windows = self.windows.read();
        let window = windows.get(&window_id)?;
        let tab = window.tabs.iter().find(|t| t.id == tab_id)?;
        split_tree::get_pane_direction(tab, from, dir)
    }

    /// D-60 keyboard 1-cell nudge. Delegates to split_tree::nudge_ratio with the
    /// MIN_PANE_COLS floor for L/R or MIN_PANE_ROWS for U/D.
    pub fn nudge_split(&self, focused_pane: PaneId, dir: Direction) -> Result<(), NudgeError> {
        let (window_id, tab_id) = self
            .locate_pane(focused_pane)
            .ok_or(NudgeError::NoSplitInDirection)?;
        let min = match dir {
            Direction::Left | Direction::Right => MIN_PANE_COLS,
            Direction::Up | Direction::Down => MIN_PANE_ROWS,
        };
        let mut windows = self.windows.write();
        let window = windows
            .get_mut(&window_id)
            .ok_or(NudgeError::NoSplitInDirection)?;
        let tab = window
            .tabs
            .iter_mut()
            .find(|t| t.id == tab_id)
            .ok_or(NudgeError::NoSplitInDirection)?;
        split_tree::nudge_ratio(&mut tab.root, focused_pane, dir, min)
    }

    /// Plan-04-03 proc_tracker input: (pane_id, master_fd, pid) tuples.
    #[must_use]
    pub fn panes_snapshot(&self) -> Vec<(PaneId, Option<RawFd>, Option<i32>)> {
        self.panes
            .read()
            .values()
            .map(|p| (p.id, p.master_fd, p.pid))
            .collect()
    }

    #[must_use]
    pub fn pane(&self, id: PaneId) -> Option<Arc<Pane>> {
        self.panes.read().get(&id).cloned()
    }

    /// Scan windows for the (window, tab) that contains `pane_id`.
    #[must_use]
    pub fn locate_pane(&self, pane_id: PaneId) -> Option<(WindowId, TabId)> {
        let windows = self.windows.read();
        for (wid, window) in windows.iter() {
            for tab in &window.tabs {
                if tab.contains(pane_id) {
                    return Some((*wid, tab.id));
                }
            }
        }
        None
    }

    /// Inspection helpers (used in tests + by Plan 04-03 wiring).
    #[must_use]
    pub fn window_count(&self) -> usize {
        self.windows.read().len()
    }
    #[must_use]
    pub fn pane_count(&self) -> usize {
        self.panes.read().len()
    }
    #[must_use]
    pub fn tab_count(&self, window_id: WindowId) -> usize {
        self.windows
            .read()
            .get(&window_id)
            .map_or(0, |w| w.tabs.len())
    }
    #[must_use]
    pub fn active_tab_id(&self, window_id: WindowId) -> Option<TabId> {
        self.windows
            .read()
            .get(&window_id)
            .and_then(|w| w.active_tab_id)
    }
    #[must_use]
    pub fn active_pane_id(&self, window_id: WindowId, tab_id: TabId) -> Option<PaneId> {
        let windows = self.windows.read();
        let window = windows.get(&window_id)?;
        let tab = window.tabs.iter().find(|t| t.id == tab_id)?;
        Some(tab.active_pane_id)
    }

    /// Read-only access for tests that need to inspect tab.root shape.
    /// Returns a snapshot of (active_tab_id, active_pane_id) and applies a closure
    /// to the `PaneNode` root under the windows RwLock.
    pub fn with_tab<R>(
        &self,
        window_id: WindowId,
        tab_id: TabId,
        f: impl FnOnce(&Tab) -> R,
    ) -> Option<R> {
        let windows = self.windows.read();
        let window = windows.get(&window_id)?;
        let tab = window.tabs.iter().find(|t| t.id == tab_id)?;
        Some(f(tab))
    }

    /// Plan-04-03 async helper: drives `LocalDomain::spawn_local` and installs
    /// the resulting Pane as the first leaf of a new tab on `window_id`.
    ///
    /// Pitfall B: the `.await` happens BEFORE any RwLock write — no held lock
    /// across await points. install_tab takes the lock synchronously.
    pub async fn create_tab_async(
        &self,
        window_id: WindowId,
        cwd: Option<PathBuf>,
        rows: u16,
        cols: u16,
    ) -> Result<(TabId, PaneId)> {
        let cwd = cwd.or_else(|| Some(inherit_cwd(None)));
        let spawned = self
            .default_domain
            .spawn_local(SpawnCommand {
                argv: None,
                cwd,
                rows,
                cols,
                env: vec![],
            })
            .await?;
        let pane_id = self.allocate_pane_id();
        let term = Arc::new(Mutex::new(vector_term::Term::new(cols, rows, 10_000)));
        let pane = Arc::new(Pane::new(
            pane_id,
            term,
            spawned.transport,
            spawned.pid,
            spawned.master_fd,
        ));
        Ok(self.install_tab(window_id, pane, rows, cols))
    }

    /// Plan-04-03 async helper: split the given pane, spawning a sibling shell
    /// in the inherited cwd of the focused pane (D-63).
    pub async fn split_pane_async(
        &self,
        pane_id: PaneId,
        dir: SplitDirection,
        cwd: Option<PathBuf>,
    ) -> Result<PaneId> {
        let parent_pid = self.pane(pane_id).and_then(|p| p.shell_pid());
        // viewport size for the new pane: inherit the tab's current size.
        let (rows, cols) = self
            .locate_pane(pane_id)
            .and_then(|(wid, tid)| self.with_tab(wid, tid, |t| (t.last_rows, t.last_cols)))
            .unwrap_or((24, 80));
        let cwd = cwd.or_else(|| Some(inherit_cwd(parent_pid)));
        let spawned = self
            .default_domain
            .spawn_local(SpawnCommand {
                argv: None,
                cwd,
                rows,
                cols,
                env: vec![],
            })
            .await?;
        let new_pane_id = self.allocate_pane_id();
        let term = Arc::new(Mutex::new(vector_term::Term::new(cols, rows, 10_000)));
        let pane = Arc::new(Pane::new(
            new_pane_id,
            term,
            spawned.transport,
            spawned.pid,
            spawned.master_fd,
        ));
        self.split_pane(pane_id, dir, pane).map_err(Into::into)
    }

    /// Window-resize hook: update each tab's viewport, redistribute split ratios,
    /// and return (PaneId, rows, cols) tuples so the App layer can push the new
    /// dims through each pane's resize channel. CORE-04 reuse — kernel SIGWINCH
    /// reaches child shells through `PtyTransport::resize`.
    pub fn resize_window(
        &self,
        window_id: WindowId,
        rows: u16,
        cols: u16,
    ) -> Vec<(PaneId, u16, u16)> {
        let mut out = Vec::new();
        let mut windows = self.windows.write();
        let Some(window) = windows.get_mut(&window_id) else {
            return out;
        };
        for tab in &mut window.tabs {
            tab.last_rows = rows;
            tab.last_cols = cols;
            let viewport = Rect {
                x: 0,
                y: 0,
                w: cols,
                h: rows,
            };
            split_tree::redistribute(&mut tab.root, viewport);
            let layout = split_tree::compute_layout(&tab.root, viewport);
            for (pane_id, rect) in layout {
                out.push((pane_id, rect.h, rect.w));
            }
        }
        out
    }
}
