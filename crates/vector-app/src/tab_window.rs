//! Per-Tab winit Window state. Plan 04-04 (D-56).
//!
//! Each NSWindowTabbingMode-grouped winit::Window owns one TabWindow holding
//! its compositors keyed by `PaneId`, the render host, the per-window overlay,
//! and the first-paint gate. Cmd-T spawns a new `TabWindow` via
//! [`crate::mux_commands::create_tabbed_winit_window`].

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use vector_mux::{PaneId, TabId, WindowId as MuxWindowId};
use vector_render::Compositor;
use winit::window::Window;

use crate::{overlay::Overlay, render_host::RenderHost};

/// Per-Tab winit window state.
pub struct TabWindow {
    pub mux_window_id: MuxWindowId,
    pub tab_id: TabId,
    pub winit_window: Arc<Window>,
    pub render_host: RenderHost,
    pub overlay: Option<Overlay>,
    pub overlay_dropped: bool,
    pub first_paint_ready: bool,
    pub last_resize_at: Option<Instant>,
    pub pending_resize: Option<(u16, u16)>,
    /// Per-pane compositors. Plan 04-04 ships single-pane today; the map is the
    /// seam Plan 04-05 polish + multi-pane rendering will consume.
    pub compositors: HashMap<PaneId, Compositor>,
    pub active_pane_id: PaneId,
}

impl TabWindow {
    pub fn new(
        mux_window_id: MuxWindowId,
        tab_id: TabId,
        winit_window: Arc<Window>,
        render_host: RenderHost,
        overlay: Option<Overlay>,
        active_pane_id: PaneId,
    ) -> Self {
        Self {
            mux_window_id,
            tab_id,
            winit_window,
            render_host,
            overlay,
            overlay_dropped: false,
            first_paint_ready: false,
            last_resize_at: None,
            pending_resize: None,
            compositors: HashMap::new(),
            active_pane_id,
        }
    }

    pub fn request_redraw(&self) {
        self.winit_window.request_redraw();
    }
}
