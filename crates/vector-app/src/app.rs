use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tokio::sync::mpsc;
use vector_input::{
    encode_key, wrap_bracketed_paste, EncodedKey, ModState, MuxCommand, SelectionState,
};
use vector_mux::Mux;
use vector_term::Term;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::Key;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::input_bridge::InputBridge;
use crate::mux_commands::{self, WindowFactory, WinitWindowFactory, VECTOR_TABBING_IDENTIFIER};
use crate::overlay::Overlay;
use crate::render_host::RenderHost;
use crate::{menu, overlay, UserEvent};

/// Window size threshold for debouncing `Term::resize` (D-49).
const RESIZE_DEBOUNCE: Duration = Duration::from_millis(50);

/// Per-winit-Window state. Plan 04-04 (D-56): each NSWindowTabbingMode-grouped
/// window holds its own RenderHost + overlay + first-paint gate. Multi-pane
/// rendering inside a window remains Plan 04-05 polish.
struct AppWindow {
    window: Arc<Window>,
    render_host: Option<RenderHost>,
    overlay: Option<Overlay>,
    overlay_dropped: bool,
    first_paint_ready: bool,
    last_resize_at: Option<Instant>,
    pending_resize: Option<(u16, u16)>,
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
        }
    }

    fn primary_window(&self) -> Option<&AppWindow> {
        self.windows.values().next()
    }

    fn primary_window_mut(&mut self) -> Option<&mut AppWindow> {
        self.windows.values_mut().next()
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

    /// D-49 debounce: if a pending resize is ≥ 50 ms old on the given window, flush it now.
    fn flush_pending_resize_if_quiescent(&mut self, id: WindowId) {
        let Some(aw) = self.windows.get_mut(&id) else {
            return;
        };
        if let (Some(at), Some((rows, cols))) = (aw.last_resize_at, aw.pending_resize) {
            if at.elapsed() >= RESIZE_DEBOUNCE {
                self.input_bridge.send_resize(rows, cols);
                aw.pending_resize = None;
                aw.last_resize_at = None;
            }
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
            },
        );
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
                tracing::info!(
                    "{} — Plan 04-05 wires the per-pane Compositor + redistribute",
                    mux_commands::describe(cmd)
                );
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
            MuxCommand::FocusDir(_) | MuxCommand::NudgeSplit(_) => {
                tracing::info!(
                    "{} — Plan 04-05 wires multi-pane focus/nudge UI",
                    mux_commands::describe(cmd)
                );
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
            },
        );
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::PaneOutput { pane_id, bytes } => {
                // Plan 04-04 shim: still single-Term per process; Plan 04-05
                // routes per-pane bytes into the per-pane Term inside the Mux.
                let _ = pane_id;
                if bytes.is_empty() {
                    return;
                }
                {
                    let mut t = self.term.lock();
                    t.feed(&bytes);
                }
                if let Some(aw) = self.primary_window_mut() {
                    if !aw.overlay_dropped {
                        aw.overlay = None;
                        aw.overlay_dropped = true;
                    }
                    if !aw.first_paint_ready {
                        aw.first_paint_ready = true;
                        tracing::info!("first PTY byte received; first-paint gate open (D-51)");
                    }
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
                tracing::info!(?pane_id, %label, "pane title changed");
                // D-57: surface the title on the primary window. Multi-window
                // disambiguation (which window holds which pane) is Plan 04-05.
                if let Some(aw) = self.primary_window() {
                    aw.window.set_title(&format!("Vector — {label}"));
                }
            }
            UserEvent::LpmChanged(enabled) => {
                self.lpm_flag.store(enabled, Ordering::Relaxed);
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
                    None => {}
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
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
                let Some(host) = self
                    .windows
                    .get_mut(&id)
                    .and_then(|aw| aw.render_host.as_mut())
                else {
                    return;
                };
                let mut t = self.term.lock();
                if let Err(err) = host.render(&mut t, sel) {
                    tracing::warn!(?err, "render failed");
                }
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
