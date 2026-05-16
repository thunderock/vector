//! Per-Tab winit Window state. Plan 04-04 (D-56).
//!
//! Each NSWindowTabbingMode-grouped winit::Window owns one TabWindow holding
//! its compositors keyed by `PaneId`, the render host, the per-window overlay,
//! and the first-paint gate. Cmd-T spawns a new `TabWindow` via
//! [`crate::mux_commands::create_tabbed_winit_window`].

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use vector_mux::{Mux, PaneId, TabId, WindowId as MuxWindowId};
use vector_render::Compositor;
use winit::window::Window;

use crate::pty_actor::PtyActorRouter;
use crate::{overlay::Overlay, render_host::RenderHost};

/// D-49 resize-debounce window. Matches the constant in `app.rs`.
pub const RESIZE_DEBOUNCE: Duration = Duration::from_millis(50);

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

    /// D-49 per-TabWindow resize debounce flush. If a pending resize is at least
    /// `RESIZE_DEBOUNCE` old, route it through the Mux (so all panes in this
    /// window's tab learn the new viewport) and through the router (kernel
    /// SIGWINCH per pane). Returns `true` iff a flush occurred — caller may
    /// recompute per-pane viewports + request_redraw.
    pub fn flush_pending_resize_if_quiescent(
        &mut self,
        now: Instant,
        mux: &Mux,
        router: &PtyActorRouter,
    ) -> bool {
        let (Some(at), Some((rows, cols))) = (self.last_resize_at, self.pending_resize) else {
            return false;
        };
        if now.duration_since(at) < RESIZE_DEBOUNCE {
            return false;
        }
        for (pane_id, rows, cols) in mux.resize_window(self.mux_window_id, rows, cols) {
            router.send_resize(pane_id, rows, cols);
        }
        self.pending_resize = None;
        self.last_resize_at = None;
        true
    }
}
