//! Mux trait surface (D-38) + Phase-4 topology (D-67).
//!
//! Phase 2 ships:
//!   - `PtyTransport` + `Domain` traits in FINAL shape (Phases 7/8/9 only fill bodies).
//!   - `LocalDomain` fully implemented atop `vector_pty::LocalPty`.
//!   - `CodespaceDomain` + `DevTunnelDomain` stubs that `unimplemented!()` at runtime.
//!
//! Phase 4 Plan 02 adds:
//!   - `Mux` singleton + `Window` + `Tab` + `Pane` + `PaneNode` split tree
//!   - Pure-algorithm `split_tree` module: layout, mutation, directional focus, nudge
//!   - `CloseResult` / `Direction` / `SplitDirection` mux-level enums

pub use codespace_domain::CodespaceDomain;
pub use cwd::{inherit_cwd, inherit_cwd_with};
pub use devtunnel_domain::DevTunnelDomain;
pub use domain::{Domain, SpawnCommand};
pub use ids::{
    CloseResult, Direction, IdAllocator, NudgeError, PaneId, SplitDirection, SplitError, TabId,
    WindowId, MIN_PANE_COLS, MIN_PANE_ROWS,
};
pub use local_domain::{LocalDomain, LocalTransport};
pub use mux::Mux;
pub use pane::{
    format_tab_title, spawn_cwd_for, spawn_cwd_for_with_proc, Pane, PaneCwdView, PaneNode,
    SplitRatio,
};
pub use proc_tracker::{proc_name_poll_loop, spawn_proc_tracker};
pub use spawned_pane::SpawnedPane;
pub use split_tree::{
    compute_layout, get_pane_direction, nudge_ratio, redistribute, remove_leaf, split_at_leaf, Rect,
};
pub use tab::Tab;
pub use transport::{PtyTransport, TransportKind};
pub use window::Window;

mod codespace_domain;
pub mod cwd;
mod devtunnel_domain;
mod domain;
pub mod ids;
mod local_domain;
pub mod mux;
pub mod pane;
pub mod proc_tracker;
pub mod spawned_pane;
pub mod split_tree;
pub mod tab;
mod transport;
pub mod window;
