//! Vector app shell crate. The binary entry is `main.rs`; this library exposes
//! the modules so integration tests (and Plan 04-04's multi_window_tabbing test)
//! can drive them without spinning up a real event loop.

#![allow(unsafe_code)]

use vector_mux::PaneId;

pub mod app;
pub mod frame_tick;
pub mod input_bridge;
pub mod lpm;
pub mod menu;
pub mod mux_commands;
pub mod overlay;
pub mod pty_actor;
pub mod render_host;
pub mod tab_window;

pub use mux_commands::{WindowFactory, WinitWindowFactory, VECTOR_TABBING_IDENTIFIER};
pub use tab_window::TabWindow;

/// Phase-4 cross-thread event variants. Plan 04-03 keyed PtyOutput / Resized by PaneId.
#[derive(Debug, Clone)]
pub enum UserEvent {
    PaneOutput {
        pane_id: PaneId,
        bytes: Vec<u8>,
    },
    PaneResized {
        pane_id: PaneId,
        rows: u16,
        cols: u16,
    },
    PaneExited(PaneId),
    PaneTitleChanged {
        pane_id: PaneId,
        label: String,
    },
    LpmChanged(bool),
}
