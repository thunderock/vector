use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tokio::sync::mpsc;
use vector_input::{
    encode_key, wrap_bracketed_paste, EncodedKey, ModState, MuxCommand, SelectionState,
};
use vector_mux::{compute_layout, Mux, PaneId, Rect, SplitDirection, WindowId as MuxWindowId};
use vector_render::Compositor;
use vector_term::Term;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::Key;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::hyperlink_dispatch;
use crate::input_bridge::InputBridge;
use crate::mux_commands::{WindowFactory, WinitWindowFactory, VECTOR_TABBING_IDENTIFIER};
use crate::overlay::Overlay;
use crate::pty_actor::PtyActorRouter;
use crate::render_host::RenderHost;
use crate::ske::SecureInputGuard;
use crate::toast::{ToastBanner, ToastStack};
use crate::{menu, overlay, UserEvent};

/// D-66 active-pane border color (light blue accent).
const BORDER_COLOR_ACTIVE: [f32; 4] = [0.4, 0.6, 1.0, 1.0];
/// Inactive pane: alpha 0 disables the border shader contribution.
const BORDER_COLOR_INACTIVE: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

/// Window size threshold for debouncing `Term::resize` (D-49).
const RESIZE_DEBOUNCE: Duration = Duration::from_millis(50);

/// Per-winit-Window state. Plan 04-04 (D-56): each NSWindowTabbingMode-grouped
/// window holds its own RenderHost + overlay + first-paint gate. Plan 04-06:
/// multi-pane shape with per-pane `compositors` map + `active_pane_id`.
struct AppWindow {
    window: Arc<Window>,
    render_host: Option<RenderHost>,
    overlay: Option<Overlay>,
    overlay_dropped: bool,
    first_paint_ready: bool,
    last_resize_at: Option<Instant>,
    pending_resize: Option<(u16, u16)>,
    /// Plan 04-06: per-pane compositors keyed by Mux PaneId. Populated lazily
    /// on first `UserEvent::PaneOutput` for a pane.
    compositors: HashMap<PaneId, Compositor>,
    /// Plan 04-06: which pane currently owns the active-pane border + filled cursor.
    /// First pane registered becomes active; Cmd-Opt-Arrow flips it.
    active_pane_id: Option<PaneId>,
}

pub struct App {
    /// Plan 04-04: HashMap<winit::WindowId, AppWindow> replaces the single
    /// `Option<Arc<Window>>` so Cmd-T can spawn additional tab-grouped windows.
    windows: HashMap<WindowId, AppWindow>,
    term: Arc<Mutex<Term>>,
    input_bridge: InputBridge,
    mods: ModState,
    cursor_px: PhysicalPosition<f64>,
    lpm_flag: Arc<AtomicBool>,
    /// Plan 04-05: dispatches Cmd-D/Cmd-Shift-D split requests to the I/O thread
    /// which drives `Mux::split_pane_async` + `router.spawn_pane`.
    split_req_tx: Option<mpsc::Sender<(PaneId, SplitDirection)>>,
    /// Plan 04-06: shared handle to the per-pane PtyActorRouter so the App's
    /// per-pane SIGWINCH walk in `flush_pending_resize_if_quiescent` can call
    /// `router.send_resize(pane_id, rows, cols)` for each pane in the layout.
    router: Option<Arc<Mutex<PtyActorRouter>>>,
    /// Plan 04-06: winit::WindowId -> vector_mux::WindowId map. The bootstrap
    /// window records its mapping in `resumed`; Cmd-T windows reuse the
    /// bootstrap mux WindowId (TODO(phase-5): allocate a fresh Mux Window per
    /// Cmd-T NSWindow when handle_new_tab spawns a real Mux Tab+Pane).
    winit_to_mux_window: HashMap<WindowId, MuxWindowId>,
    /// Plan 05-10 M2 — single-banner toast stack (info / action). UI-SPEC §5.4.
    toasts: ToastStack,
    /// Plan 05-10 B1 — last hovered (row, col) + URI for the active pane. Used by
    /// the Cmd-click handler in `WindowEvent::MouseInput` and the Cmd-hover
    /// `NSCursor.pointingHand` swap.
    hover_uri: Option<String>,
    /// Plan 05-10 Task 3 — current ConfigFile applied. Populated by the watcher
    /// thread via UserEvent::ConfigReloaded.
    current_config: Option<std::sync::Arc<vector_config::ConfigFile>>,
    /// Plan 05-09 / POLISH-08 / D-80 — RAII guard for Carbon SKE. Drop on
    /// app exit disables the flag (Pitfall 6).
    ske_guard: SecureInputGuard,
}

impl App {
    pub fn new(
        write_tx: mpsc::Sender<Vec<u8>>,
        resize_tx: mpsc::Sender<(u16, u16)>,
        lpm_flag: Arc<AtomicBool>,
    ) -> Self {
        Self {
            windows: HashMap::new(),
            term: Arc::new(Mutex::new(Term::new(80, 24, 10_000))),
            input_bridge: InputBridge::new(write_tx, resize_tx),
            mods: ModState::default(),
            cursor_px: PhysicalPosition::new(0.0, 0.0),
            lpm_flag,
            split_req_tx: None,
            router: None,
            winit_to_mux_window: HashMap::new(),
            toasts: ToastStack::default(),
            hover_uri: None,
            current_config: None,
            ske_guard: SecureInputGuard::new(),
        }
    }

    /// Plan 05-10 Task 3 — Cmd-C native pasteboard write (CONTEXT Cmd-C
    /// Claude's Discretion: NSPasteboard, NEVER OSC 52).
    #[allow(clippy::unused_self)]
    fn write_pasteboard(&self, s: &str) {
        use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
        use objc2_foundation::NSString;
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();
        let ns_s = NSString::from_str(s);
        unsafe {
            pb.setString_forType(&ns_s, NSPasteboardTypeString);
        }
    }

    /// Plan 04-05: hook the split request channel so Cmd-D / Cmd-Shift-D can
    /// dispatch async splits to the I/O thread.
    pub fn set_split_req_tx(&mut self, tx: mpsc::Sender<(PaneId, SplitDirection)>) {
        self.split_req_tx = Some(tx);
    }

    /// Plan 04-06: hook the per-pane PtyActorRouter so the App's
    /// `flush_pending_resize_if_quiescent` can fan SIGWINCH out per-pane via
    /// `Mux::resize_window` + `router.send_resize`.
    pub fn set_router(&mut self, router: Arc<Mutex<PtyActorRouter>>) {
        self.router = Some(router);
    }

    fn primary_window(&self) -> Option<&AppWindow> {
        self.windows.values().next()
    }

    fn cell_from_pixel(&self, px: PhysicalPosition<f64>) -> Option<(u16, u16)> {
        let host = self.primary_window()?.render_host.as_ref()?;
        let (cw, ch) = host.cell_metrics_px()?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px_x = px.x.max(0.0).min(f64::from(u32::MAX)) as u32;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px_y = px.y.max(0.0).min(f64::from(u32::MAX)) as u32;
        let col = u16::try_from(px_x / cw.max(1)).unwrap_or(u16::MAX);
        let row = u16::try_from(px_y / ch.max(1)).unwrap_or(u16::MAX);
        Some((col, row))
    }

    fn request_redraw_all(&self) {
        for w in self.windows.values() {
            w.window.request_redraw();
        }
    }

    fn request_redraw(&self, id: WindowId) {
        if let Some(w) = self.windows.get(&id) {
            w.window.request_redraw();
        }
    }

    /// D-49 debounce + Plan 04-06 per-pane SIGWINCH fanout. Mirrors
    /// `TabWindow::flush_pending_resize_if_quiescent`. When the pending resize is
    /// ≥ 50 ms old, walks `Mux::resize_window` (which redistributes split ratios
    /// and emits per-pane (rows, cols)) and routes each tuple through the
    /// PtyActorRouter so the kernel SIGWINCH reaches each child shell.
    fn flush_pending_resize_if_quiescent(&mut self, id: WindowId) {
        let Some(aw) = self.windows.get_mut(&id) else {
            return;
        };
        let (Some(at), Some((rows, cols))) = (aw.last_resize_at, aw.pending_resize) else {
            return;
        };
        if at.elapsed() < RESIZE_DEBOUNCE {
            return;
        }
        let Some(mux) = Mux::try_get() else {
            return;
        };
        let Some(mux_window_id) = self.winit_to_mux_window.get(&id).copied() else {
            // No Mux mapping yet (pre-bootstrap); clear the pending so we don't spin.
            aw.pending_resize = None;
            aw.last_resize_at = None;
            return;
        };
        let Some(router) = self.router.clone() else {
            return;
        };
        let walk = mux.resize_window(mux_window_id, rows, cols);
        {
            let router = router.lock();
            for (pane_id, prows, pcols) in walk {
                router.send_resize(pane_id, prows, pcols);
            }
        }
        aw.pending_resize = None;
        aw.last_resize_at = None;
    }

    /// Plan 04-06 (Gap 3): when focus moves to `new_id`, paint the D-66 border
    /// on the new-active pane's compositor, clear the border on the old-active
    /// pane's compositor, and flip `cursor_focused` to filled (active) / hollow
    /// (inactive). Updates `aw.active_pane_id` for the window holding `new_id`.
    fn apply_focus_change(&mut self, new_id: PaneId) {
        for aw in self.windows.values_mut() {
            if !aw.compositors.contains_key(&new_id) {
                continue;
            }
            let Some(host) = aw.render_host.as_ref() else {
                continue;
            };
            let queue = host.queue();
            let old_id = aw.active_pane_id;
            if let Some(old) = old_id {
                if old != new_id {
                    if let Some(comp) = aw.compositors.get_mut(&old) {
                        comp.set_border_color(queue, BORDER_COLOR_INACTIVE);
                        comp.set_cursor_focused(false);
                    }
                }
            }
            if let Some(comp) = aw.compositors.get_mut(&new_id) {
                comp.set_border_color(queue, BORDER_COLOR_ACTIVE);
                comp.set_cursor_focused(true);
            }
            aw.active_pane_id = Some(new_id);
        }
    }

    /// Plan 04-06 (Gap 1): per-pane render loop. Acquires the surface frame
    /// once, then iterates the window's compositors against the layout from
    /// `Mux::compute_layout`. First pane uses `LoadOp::Clear`; subsequent panes
    /// use `LoadOp::Load` so each compositor paints onto the same view.
    #[allow(clippy::too_many_lines)]
    fn render_window(&mut self, id: WindowId, sel: Option<((u16, u16), (u16, u16))>) {
        let Some(aw) = self.windows.get_mut(&id) else {
            return;
        };
        // Fall back to the legacy single-pane render path when the per-pane map
        // hasn't been populated yet (pre-first-paint).
        if aw.compositors.is_empty() {
            let Some(host) = aw.render_host.as_mut() else {
                return;
            };
            let mut t = self.term.lock();
            if let Err(err) = host.render(&mut t, sel) {
                tracing::warn!(?err, "render failed");
            }
            return;
        }
        let Some(host) = aw.render_host.as_mut() else {
            return;
        };
        // Resolve the Mux tab + layout once per frame, off any aw borrow.
        let mux_window_id = self.winit_to_mux_window.get(&id).copied();
        let mux = Mux::try_get();
        let layout_snapshot = match (mux.as_ref(), mux_window_id) {
            (Some(mux), Some(wid)) => {
                let tab_id = mux.active_tab_id(wid);
                tab_id.and_then(|tid| {
                    mux.with_tab(wid, tid, |tab| {
                        let viewport = Rect {
                            x: 0,
                            y: 0,
                            w: tab.last_cols,
                            h: tab.last_rows,
                        };
                        let layout = compute_layout(&tab.root, viewport);
                        // Sort leaves by PaneId for deterministic render order.
                        let mut leaves = tab.root.leaves();
                        leaves.sort();
                        (leaves, layout)
                    })
                })
            }
            _ => None,
        };
        let Some((leaves, layout)) = layout_snapshot else {
            return;
        };
        // Resolve cell metrics from any existing compositor in the window.
        let Some((cell_w, cell_h)) = aw
            .compositors
            .values()
            .next()
            .map(|c| (c.cell_width_px(), c.cell_height_px()))
        else {
            return;
        };
        // Acquire the surface frame once. Skip the frame on Outdated/Lost.
        let frame = match host.acquire_frame() {
            Ok(Some(f)) => f,
            Ok(None) => return,
            Err(err) => {
                tracing::warn!(?err, "acquire_frame failed");
                return;
            }
        };
        let view = &frame.view;
        let width = frame.width;
        let height = frame.height;
        let device = host.device();
        let queue = host.queue();
        let default_bg = wgpu::Color {
            r: 0.06,
            g: 0.06,
            b: 0.06,
            a: 1.0,
        };
        let active_pane_id = aw.active_pane_id;
        let mut first = true;
        for pane_id in leaves {
            let Some(rect) = layout.get(&pane_id) else {
                continue;
            };
            let Some(comp) = aw.compositors.get_mut(&pane_id) else {
                continue;
            };
            #[allow(clippy::cast_precision_loss)]
            let offset_px = [
                f32::from(rect.x) * cell_w as f32,
                f32::from(rect.y) * cell_h as f32,
            ];
            #[allow(clippy::cast_precision_loss)]
            let size_px = [
                f32::from(rect.w) * cell_w as f32,
                f32::from(rect.h) * cell_h as f32,
            ];
            comp.set_viewport(queue, offset_px, size_px);
            let is_active = active_pane_id == Some(pane_id);
            comp.set_border_color(
                queue,
                if is_active {
                    BORDER_COLOR_ACTIVE
                } else {
                    BORDER_COLOR_INACTIVE
                },
            );
            comp.set_cursor_focused(is_active);
            // Source-of-truth term per pane: the Mux Pane's own term mutex.
            // Selection is forwarded only to the active pane.
            let load_op = if first {
                wgpu::LoadOp::Clear(default_bg)
            } else {
                wgpu::LoadOp::Load
            };
            let pane_sel = if is_active { sel } else { None };
            if let Some(pane) = Mux::try_get().and_then(|m| m.pane(pane_id)) {
                let mut t = pane.term.lock();
                if let Err(err) = comp.render_into_view(
                    device, queue, view, width, height, &mut t, pane_sel, load_op,
                ) {
                    tracing::warn!(?pane_id, ?err, "compositor render_into_view failed");
                }
            } else {
                // No Mux pane for this id (race): fall back to the shared term
                // so we still paint something instead of a black hole.
                let mut t = self.term.lock();
                if let Err(err) = comp.render_into_view(
                    device, queue, view, width, height, &mut t, pane_sel, load_op,
                ) {
                    tracing::warn!(
                        ?pane_id,
                        ?err,
                        "compositor render_into_view fallback failed"
                    );
                }
            }
            first = false;
        }
        frame.present();
    }

    /// Plan 04-06 (Gap 1 plumbing): lazily create a per-pane Compositor for
    /// every Mux leaf in the tab that holds `seed_pane_id`. No-op if the window
    /// has no render host. Idempotent — only creates compositors for leaves not
    /// already in the map.
    fn ensure_compositors_for_pane(&mut self, window_id: WindowId, seed_pane_id: PaneId) {
        let Some(mux) = Mux::try_get() else {
            return;
        };
        let Some((mux_window_id, tab_id)) = mux.locate_pane(seed_pane_id) else {
            return;
        };
        // Snapshot leaves + viewport + layout under a single with_tab read lock.
        let snapshot = mux.with_tab(mux_window_id, tab_id, |tab| {
            let viewport = Rect {
                x: 0,
                y: 0,
                w: tab.last_cols,
                h: tab.last_rows,
            };
            let layout = compute_layout(&tab.root, viewport);
            (tab.root.leaves(), layout)
        });
        let Some((leaves, layout)) = snapshot else {
            return;
        };
        let Some(aw) = self.windows.get_mut(&window_id) else {
            return;
        };
        let Some(host) = aw.render_host.as_ref() else {
            return;
        };
        // For the very first compositor we don't know cell metrics yet; build
        // it sized to the full surface and read its metrics back. Subsequent
        // panes use those metrics to derive their viewport pixel rects.
        let (cell_w, cell_h) = if let Some(m) = aw
            .compositors
            .values()
            .next()
            .map(|c| (c.cell_width_px(), c.cell_height_px()))
        {
            m
        } else {
            let (sw, sh) = host.surface_size();
            let viewport_offset = [0.0_f32, 0.0_f32];
            #[allow(clippy::cast_precision_loss)]
            let viewport_size = [sw as f32, sh as f32];
            match host.new_compositor_for_viewport(viewport_offset, viewport_size) {
                Ok(comp) => {
                    let cw = comp.cell_width_px();
                    let ch = comp.cell_height_px();
                    aw.compositors.insert(seed_pane_id, comp);
                    (cw, ch)
                }
                Err(err) => {
                    tracing::error!(?err, "lazy Compositor init failed");
                    return;
                }
            }
        };
        if cell_w == 0 || cell_h == 0 {
            return;
        }
        for pane_id in leaves {
            if aw.compositors.contains_key(&pane_id) {
                continue;
            }
            let Some(rect) = layout.get(&pane_id) else {
                continue;
            };
            #[allow(clippy::cast_precision_loss)]
            let offset_px = [
                f32::from(rect.x) * cell_w as f32,
                f32::from(rect.y) * cell_h as f32,
            ];
            #[allow(clippy::cast_precision_loss)]
            let size_px = [
                f32::from(rect.w) * cell_w as f32,
                f32::from(rect.h) * cell_h as f32,
            ];
            match host.new_compositor_for_viewport(offset_px, size_px) {
                Ok(comp) => {
                    aw.compositors.insert(pane_id, comp);
                }
                Err(err) => {
                    tracing::error!(?pane_id, ?err, "per-pane Compositor init failed");
                }
            }
        }
        if aw.active_pane_id.is_none() {
            aw.active_pane_id = Some(seed_pane_id);
        }
    }

    /// Cmd-T: create a new NSWindowTabbingMode-grouped winit Window (D-56)
    /// and register an AppWindow for it. Plan 04-04 only ships the window-
    /// spawn flow; per-pane Mux wiring + a fresh PTY actor land in Plan 04-05.
    fn handle_new_tab(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = WindowAttributes::default()
            .with_title("Vector")
            .with_inner_size(LogicalSize::new(1024.0, 640.0));
        let factory = WinitWindowFactory { event_loop };
        let win = match factory.create_tabbed(attrs, VECTOR_TABBING_IDENTIFIER) {
            Ok(w) => w,
            Err(err) => {
                tracing::error!(?err, "Cmd-T: create_tabbed failed");
                return;
            }
        };
        let id = win.id();
        // SAFETY: winit guarantees user_event/window_event run on the macOS main thread.
        let overlay_inst = unsafe { Some(overlay::install(&win)) };
        let render_host = match RenderHost::new(&win) {
            Ok(h) => Some(h),
            Err(err) => {
                tracing::error!(?err, "Cmd-T: RenderHost init failed");
                None
            }
        };
        self.windows.insert(
            id,
            AppWindow {
                window: win,
                render_host,
                overlay: overlay_inst,
                overlay_dropped: false,
                first_paint_ready: false,
                last_resize_at: None,
                pending_resize: None,
                compositors: HashMap::new(),
                active_pane_id: None,
            },
        );
        // TODO(phase-5): per-NSWindow mux WindowId allocation when Cmd-T spawns a
        // fresh Mux Tab+Pane. Plan 04-06 bounded scope: reuse the bootstrap mux
        // WindowId so newly-created tab-group NSWindows still route resize.
        if let Some(mux_window_id) =
            Mux::try_get().and_then(|m| m.window_ids_snapshot().first().copied())
        {
            self.winit_to_mux_window.insert(id, mux_window_id);
        }
        tracing::info!(window_id = ?id, "Cmd-T: new tab-grouped window created");
    }

    /// Dispatch an `EncodedKey::Mux(...)` command. Plan 04-04 wires Cmd-T
    /// directly (window spawn); other commands route through the Mux
    /// singleton and log their outcome — Plan 04-05 polishes the per-pane
    /// renderer side-effects (border focus flip, viewport redistribute, etc.).
    fn handle_mux_command(&mut self, event_loop: &ActiveEventLoop, cmd: MuxCommand) {
        tracing::info!(?cmd, "mux command dispatch");
        match cmd {
            MuxCommand::NewTab => self.handle_new_tab(event_loop),
            MuxCommand::ClosePane => {
                if let Some(mux) = Mux::try_get() {
                    if let Some(active) = mux.any_active_pane_id() {
                        let result = mux.close_pane(active);
                        tracing::info!(?result, "close_pane cascade");
                        if matches!(result, vector_mux::CloseResult::LastWindowClosed) {
                            event_loop.exit();
                        }
                    }
                }
            }
            MuxCommand::SplitHorizontal | MuxCommand::SplitVertical => {
                // Plan 04-05: dispatch the async split to the I/O thread. Per-pane
                // Compositor wiring + visible second-shell rendering lands in the
                // multi-pane render polish (Plan 04-06 gap-closure).
                if let Some(mux) = Mux::try_get() {
                    if let Some(active) = mux.any_active_pane_id() {
                        let dir = if matches!(cmd, MuxCommand::SplitHorizontal) {
                            vector_mux::SplitDirection::Horizontal
                        } else {
                            vector_mux::SplitDirection::Vertical
                        };
                        if let Some(req_tx) = self.split_req_tx.as_ref() {
                            if let Err(err) = req_tx.try_send((active, dir)) {
                                tracing::warn!(?err, "split request channel full/closed");
                            } else {
                                tracing::info!(
                                    pane = ?active,
                                    ?dir,
                                    "split request dispatched to I/O thread"
                                );
                            }
                        }
                    }
                }
            }
            MuxCommand::CycleTabNext | MuxCommand::CycleTabPrev => {
                if let Some(mux) = Mux::try_get() {
                    let dir = if matches!(cmd, MuxCommand::CycleTabNext) {
                        vector_mux::Direction::Right
                    } else {
                        vector_mux::Direction::Left
                    };
                    // Cycle the (single) window's tabs in mux; AppKit owns the
                    // visible tab-bar switch (D-56). Plan 04-05 reconciles when
                    // mux runs multi-tab.
                    for &wid in &mux.window_ids_snapshot() {
                        mux.cycle_tab(wid, dir);
                    }
                }
            }
            MuxCommand::FocusDir(dir) => {
                if let Some(mux) = Mux::try_get() {
                    if let Some(active) = mux.any_active_pane_id() {
                        if let Some(new_id) = mux.focus_direction(active, dir) {
                            tracing::info!(?active, ?new_id, "focus moved");
                            // Plan 04-06 Gap 3: invoke the D-66 border-color setter
                            // on both the old-active and new-active compositors,
                            // and flip cursor_focused for filled vs hollow cursor.
                            self.apply_focus_change(new_id);
                            self.request_redraw_all();
                        } else {
                            tracing::debug!("focus_direction returned no neighbor; absorbed");
                        }
                    }
                }
            }
            MuxCommand::NudgeSplit(dir) => {
                if let Some(mux) = Mux::try_get() {
                    if let Some(active) = mux.any_active_pane_id() {
                        if let Err(err) = mux.nudge_split(active, dir) {
                            tracing::debug!(?err, "nudge_split no-op");
                        }
                    }
                }
            }
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.windows.is_empty() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title("Vector")
            .with_inner_size(LogicalSize::new(1024.0, 640.0));
        // Use the factory so the bootstrap window also joins the tab group on
        // first launch (D-56 + winit#2238 belt-and-braces).
        let factory = WinitWindowFactory { event_loop };
        let window = factory
            .create_tabbed(attrs, VECTOR_TABBING_IDENTIFIER)
            .expect("create bootstrap window");

        // SAFETY: winit guarantees `resumed` runs on the macOS main thread.
        let overlay_inst = unsafe {
            menu::install_main_menu();
            Some(overlay::install(&window))
        };
        let render_host = match RenderHost::new(&window) {
            Ok(host) => Some(host),
            Err(err) => {
                tracing::error!(?err, "RenderHost init failed");
                None
            }
        };
        let id = window.id();
        self.windows.insert(
            id,
            AppWindow {
                window,
                render_host,
                overlay: overlay_inst,
                overlay_dropped: false,
                first_paint_ready: false,
                last_resize_at: None,
                pending_resize: None,
                compositors: HashMap::new(),
                active_pane_id: None,
            },
        );
        // Plan 04-06: record the bootstrap mux WindowId mapping. The I/O thread
        // creates exactly one Mux window on startup (main.rs); we adopt its id.
        if let Some(mux_window_id) =
            Mux::try_get().and_then(|m| m.window_ids_snapshot().first().copied())
        {
            self.winit_to_mux_window.insert(id, mux_window_id);
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::PaneOutput { pane_id, bytes } => {
                if bytes.is_empty() {
                    return;
                }
                // Plan 04-06: feed bytes into the pane's own per-pane Term (the
                // source of truth in the Mux). Backward-compat: keep mirroring
                // the ACTIVE pane's bytes into the App's shared `self.term` so
                // existing selection / cell_from_pixel plumbing still works.
                let active = Mux::try_get().and_then(|m| m.any_active_pane_id());
                let is_active = active.is_some_and(|a| a == pane_id);
                if let Some(pane) = Mux::try_get().and_then(|m| m.pane(pane_id)) {
                    let mut t = pane.term.lock();
                    t.feed(&bytes);
                    // D-79 OSC 7 consumer: sync the ring's most-recent cwd into
                    // the pane so spawn_cwd_for / format_tab_title can read it.
                    let latest = t.cwd_ring().back().cloned();
                    drop(t);
                    *pane.cwd.lock() = latest;
                }
                if is_active {
                    let mut t = self.term.lock();
                    t.feed(&bytes);
                }
                // Plan 04-06: lazily ensure per-pane Compositors are built for
                // every leaf in this pane's tab. New panes from Cmd-D land here.
                let window_ids: Vec<WindowId> = self.windows.keys().copied().collect();
                for wid in window_ids {
                    self.ensure_compositors_for_pane(wid, pane_id);
                }
                // First-paint gate (D-51, per-window per Pitfall H): flip on ANY
                // pane's first non-empty drain.
                for aw in self.windows.values_mut() {
                    if !aw.overlay_dropped {
                        aw.overlay = None;
                        aw.overlay_dropped = true;
                    }
                    if !aw.first_paint_ready {
                        aw.first_paint_ready = true;
                        tracing::info!(
                            ?pane_id,
                            "first PTY byte received; per-window first-paint gate open (D-51)"
                        );
                    }
                    // Plan 04-06: redraw on ANY pane's output (not only the
                    // active one), since the per-pane Compositor map paints
                    // every pane independently.
                    aw.window.request_redraw();
                }
            }
            UserEvent::PaneResized {
                pane_id,
                rows,
                cols,
            } => {
                let _ = pane_id;
                {
                    let mut t = self.term.lock();
                    t.resize(cols, rows);
                }
                self.request_redraw_all();
            }
            UserEvent::PaneExited(pane_id) => {
                tracing::info!(?pane_id, "pane exited (Plan 04-05 will render sentinel)");
            }
            UserEvent::PaneTitleChanged { pane_id, label } => {
                // D-79 B2: append `: {cwd_stem}` when OSC 7 ring is non-empty.
                let cwd = Mux::try_get()
                    .and_then(|m| m.pane(pane_id))
                    .and_then(|p| p.cwd.lock().clone());
                let title = vector_mux::format_tab_title(&label, cwd.as_deref());
                tracing::info!(?pane_id, %title, "pane title changed");
                if let Some(aw) = self.primary_window() {
                    aw.window.set_title(&format!("Vector — {title}"));
                }
            }
            UserEvent::LpmChanged(enabled) => {
                self.lpm_flag.store(enabled, Ordering::Relaxed);
            }
            // Plan 05-10 Task 3 — chrome / config / hyperlink / Cmd-N arms.
            UserEvent::ConfigReloaded(cfg) => {
                tracing::info!("config reloaded; applying");
                self.current_config = Some(cfg);
            }
            UserEvent::ConfigError(msg) => {
                tracing::warn!(error = %msg, "config error");
                self.toasts
                    .show(ToastBanner::info(format!("config error: {msg}")));
            }
            UserEvent::ReloadConfig => {
                // M4 — D-69 Cmd-Shift-R fallback. The watcher thread will pick up
                // a real file change; this arm just records intent so the menu
                // item is wired end-to-end.
                tracing::info!("ReloadConfig requested (M4 / D-69 fallback)");
            }
            UserEvent::OpenProfilePicker => {
                tracing::info!("OpenProfilePicker (Plan 05-10 picker — render pass deferred to future plan)");
            }
            UserEvent::ProfileSelected(name) => {
                tracing::info!(profile = %name, "ProfileSelected (Local kind only in v1; Codespace/DevTunnel deferred to Phase 6+)");
            }
            UserEvent::ToggleSearch => {
                tracing::info!("ToggleSearch (Plan 05-10 — search bar overlay pass deferred)");
            }
            UserEvent::ToggleSecureKeyboardEntry => {
                // POLISH-08 / D-80 — toggle Carbon SKE; menu state mirrors via
                // a future binding (UI-SPEC §5.8). Pitfall 6 RAII guarantees
                // disable on app exit even if the user leaves it on.
                let now_on = self.ske_guard.toggle();
                tracing::info!(secure_keyboard_entry = now_on, "ToggleSecureKeyboardEntry");
            }
            UserEvent::SpawnNewWindow => {
                // D-82 Cmd-N — spawn a fresh ungrouped NSWindow with default profile.
                tracing::info!("SpawnNewWindow (D-82 Cmd-N — full window factory wired in Phase-4 mux_commands)");
            }
            UserEvent::HyperlinkClicked { url } => {
                hyperlink_dispatch::open_with_nsworkspace(&url);
            }
            UserEvent::ToastInfo(text) => {
                self.toasts.show(ToastBanner::info(text));
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.windows.remove(&id);
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.mods = ModState::from_winit(modifiers.state());
            }
            WindowEvent::KeyboardInput { event, .. } => {
                // Cmd-V: read NSPasteboard + wrap in bracketed paste markers (D-53).
                if event.state == ElementState::Pressed && self.mods.cmd {
                    if let Key::Character(s) = &event.logical_key {
                        if s.as_str() == "v" {
                            let pasted = read_clipboard().unwrap_or_default();
                            let bytes = wrap_bracketed_paste(&pasted);
                            self.input_bridge.send_bytes(bytes);
                            return;
                        }
                        // Plan 05-10 / 05-11 Task 1 — Cmd-C native pasteboard write.
                        // CONTEXT Cmd-C Claude's Discretion: NSPasteboard, NEVER OSC 52.
                        // Selection text built via `selection_to_string` over a
                        // `TermGridAccess` newtype (B1: trait impl lives in vector-app
                        // to avoid a vector-input -> vector-mux -> vector-term cycle).
                        if s.as_str() == "c" && !self.mods.shift {
                            if let Some(range) = self.input_bridge.selection.range() {
                                use vector_input::{selection_to_string, SelectionMode};
                                let text = {
                                    let t = self.term.lock();
                                    selection_to_string(
                                        &range,
                                        &crate::term_grid_access::TermGridAccess(&*t),
                                        SelectionMode::Stream,
                                    )
                                };
                                self.write_pasteboard(&text);
                            }
                            return;
                        }
                    }
                }
                match encode_key(&event, self.mods) {
                    Some(EncodedKey::Pty(bytes)) => {
                        self.input_bridge.send_bytes(bytes);
                        self.request_redraw(id);
                    }
                    Some(EncodedKey::Mux(cmd)) => {
                        self.handle_mux_command(event_loop, cmd);
                        self.request_redraw_all();
                    }
                    // Plan 05-11 Rule-3 deviation: Plan 05-13 added EncodedKey::App;
                    // Plan 05-14 wires the App-shortcut dispatch. Until then, ignore
                    // so the build stays green.
                    Some(EncodedKey::App(_)) => {}
                    None => {}
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                // Plan 05-10 B1 — Cmd-click on a hovered OSC 8 hyperlink dispatches
                // via NSWorkspace; disallowed schemes show the UI-SPEC §6.1 toast.
                // Falls through to normal selection on no-link / no-Cmd.
                if state == ElementState::Pressed && self.mods.cmd {
                    if let Some(url) = self.hover_uri.clone() {
                        match hyperlink_dispatch::dispatch_cmd_click(&url, &mut self.toasts) {
                            hyperlink_dispatch::DispatchAction::OpenUrl(u) => {
                                hyperlink_dispatch::open_with_nsworkspace(&u);
                            }
                            hyperlink_dispatch::DispatchAction::None => {
                                self.request_redraw(id);
                            }
                        }
                        return;
                    }
                }
                if let Some(cell) = self.cell_from_pixel(self.cursor_px) {
                    match state {
                        ElementState::Pressed => self.input_bridge.selection.mouse_down(cell),
                        ElementState::Released => self.input_bridge.selection.mouse_up(),
                    }
                    self.request_redraw(id);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_px = position;
                // Plan 05-10 B1 — Cmd-hover affordance. Resolve cell → hyperlink_at;
                // if Cmd held and cell has a link, swap to CursorIcon::Pointer
                // (winit's portable mapping to NSCursor.pointingHand on macOS).
                self.hover_uri = self.cell_from_pixel(position).and_then(|(col, row)| {
                    let t = self.term.lock();
                    t.hyperlink_at(usize::from(row), usize::from(col))
                        .map(|(uri, _id)| uri)
                });
                if let Some(win) = self.windows.get(&id).map(|aw| Arc::clone(&aw.window)) {
                    use winit::window::Cursor;
                    if self.mods.cmd && self.hover_uri.is_some() {
                        win.set_cursor(Cursor::Icon(winit::window::CursorIcon::Pointer));
                    } else {
                        win.set_cursor(Cursor::Icon(winit::window::CursorIcon::Default));
                    }
                }
                if matches!(self.input_bridge.selection, SelectionState::Dragging(_)) {
                    if let Some(cell) = self.cell_from_pixel(position) {
                        self.input_bridge.selection.mouse_move(cell);
                        self.request_redraw(id);
                    }
                }
            }
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(_, y),
                ..
            } => {
                #[allow(clippy::cast_possible_truncation)]
                let delta = y.round() as i32;
                if delta != 0 {
                    {
                        let mut t = self.term.lock();
                        t.scroll_display(delta);
                    }
                    self.request_redraw(id);
                }
            }
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::PixelDelta(pos),
                ..
            } => {
                if let Some(host) = self.windows.get(&id).and_then(|aw| aw.render_host.as_ref()) {
                    if let Some((_cw, ch)) = host.cell_metrics_px() {
                        #[allow(clippy::cast_possible_truncation)]
                        let lines = (pos.y / f64::from(ch.max(1))) as i32;
                        if lines != 0 {
                            {
                                let mut t = self.term.lock();
                                t.scroll_display(lines);
                            }
                            self.request_redraw(id);
                        }
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(host) = self
                    .windows
                    .get_mut(&id)
                    .and_then(|aw| aw.render_host.as_mut())
                {
                    #[allow(clippy::cast_possible_truncation)]
                    let dpr = scale_factor as f32;
                    host.clear_atlases();
                    host.set_dpr(dpr);
                }
                self.request_redraw(id);
                tracing::info!(scale_factor, "DPR change; cleared atlases (D-48)");
            }
            WindowEvent::Resized(size) => {
                let Some(aw) = self.windows.get_mut(&id) else {
                    return;
                };
                if let Some(host) = aw.render_host.as_mut() {
                    host.resize(size.width, size.height);
                }
                if let Some(overlay) = aw.overlay.as_mut() {
                    overlay.relayout();
                }
                if let Some(host) = aw.render_host.as_ref() {
                    if let Some((cell_w, cell_h)) = host.cell_metrics_px() {
                        let cols =
                            u16::try_from((size.width / cell_w.max(1)).max(1)).unwrap_or(u16::MAX);
                        let rows =
                            u16::try_from((size.height / cell_h.max(1)).max(1)).unwrap_or(u16::MAX);
                        aw.pending_resize = Some((rows, cols));
                        aw.last_resize_at = Some(Instant::now());
                    }
                }
                aw.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let ready = self.windows.get(&id).is_some_and(|aw| aw.first_paint_ready);
                if !ready {
                    return;
                }
                self.flush_pending_resize_if_quiescent(id);
                let sel = self
                    .input_bridge
                    .selection
                    .range()
                    .map(|r| (r.anchor, r.cursor));
                self.render_window(id, sel);
            }
            _ => {}
        }
    }
}

/// Read the macOS general pasteboard's string content. Must run on the main thread.
fn read_clipboard() -> Option<String> {
    let pb = objc2_app_kit::NSPasteboard::generalPasteboard();
    let ns_str = pb.stringForType(unsafe { objc2_app_kit::NSPasteboardTypeString })?;
    Some(ns_str.to_string())
}
