use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tokio::sync::mpsc;
use vector_input::{encode_key, wrap_bracketed_paste, ModState, SelectionState};
use vector_term::Term;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::Key;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::{input_bridge::InputBridge, menu, overlay, render_host::RenderHost, UserEvent};

/// Window size threshold for debouncing `Term::resize` (D-49).
const RESIZE_DEBOUNCE: Duration = Duration::from_millis(50);

pub struct App {
    window: Option<Arc<Window>>,
    overlay: Option<overlay::Overlay>,
    overlay_dropped: bool,
    term: Arc<Mutex<Term>>,
    render_host: Option<RenderHost>,
    input_bridge: InputBridge,
    mods: ModState,
    cursor_px: PhysicalPosition<f64>,
    lpm_flag: Arc<AtomicBool>,
    first_paint_ready: bool,
    last_resize_at: Option<Instant>,
    pending_resize: Option<(u16, u16)>,
}

impl App {
    pub fn new(
        write_tx: mpsc::Sender<Vec<u8>>,
        resize_tx: mpsc::Sender<(u16, u16)>,
        lpm_flag: Arc<AtomicBool>,
    ) -> Self {
        Self {
            window: None,
            overlay: None,
            overlay_dropped: false,
            term: Arc::new(Mutex::new(Term::new(80, 24, 10_000))),
            render_host: None,
            input_bridge: InputBridge::new(write_tx, resize_tx),
            mods: ModState::default(),
            cursor_px: PhysicalPosition::new(0.0, 0.0),
            lpm_flag,
            first_paint_ready: false,
            last_resize_at: None,
            pending_resize: None,
        }
    }

    fn cell_from_pixel(&self, px: PhysicalPosition<f64>) -> Option<(u16, u16)> {
        let host = self.render_host.as_ref()?;
        let (cw, ch) = host.cell_metrics_px()?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px_x = px.x.max(0.0).min(f64::from(u32::MAX)) as u32;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px_y = px.y.max(0.0).min(f64::from(u32::MAX)) as u32;
        let col = u16::try_from(px_x / cw.max(1)).unwrap_or(u16::MAX);
        let row = u16::try_from(px_y / ch.max(1)).unwrap_or(u16::MAX);
        Some((col, row))
    }

    fn request_redraw(&self) {
        if let Some(w) = self.window.as_ref() {
            w.request_redraw();
        }
    }

    /// D-49 debounce: if a pending resize is ≥ 50 ms old, flush it now.
    fn flush_pending_resize_if_quiescent(&mut self) {
        if let (Some(at), Some((rows, cols))) = (self.last_resize_at, self.pending_resize) {
            if at.elapsed() >= RESIZE_DEBOUNCE {
                self.input_bridge.send_resize(rows, cols);
                self.pending_resize = None;
                self.last_resize_at = None;
            }
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title("Vector")
            .with_inner_size(LogicalSize::new(1024.0, 640.0));
        let window = Arc::new(event_loop.create_window(attrs).expect("create_window"));

        // SAFETY: winit guarantees `resumed` runs on the macOS main thread.
        unsafe {
            menu::install_main_menu();
            self.overlay = Some(overlay::install(&window));
        }
        match RenderHost::new(&window) {
            Ok(host) => self.render_host = Some(host),
            Err(err) => tracing::error!(?err, "RenderHost init failed"),
        }
        self.window = Some(window);
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::PtyOutput(bytes) => {
                if bytes.is_empty() {
                    return;
                }
                {
                    let mut t = self.term.lock();
                    t.feed(&bytes);
                }
                if !self.overlay_dropped {
                    self.overlay = None;
                    self.overlay_dropped = true;
                }
                // D-51: first non-empty drain flips the first-paint gate.
                if !self.first_paint_ready {
                    self.first_paint_ready = true;
                    tracing::info!("first PTY byte received; first-paint gate open (D-51)");
                }
                self.request_redraw();
            }
            UserEvent::Resized { rows, cols } => {
                {
                    let mut t = self.term.lock();
                    t.resize(cols, rows);
                }
                self.request_redraw();
            }
            UserEvent::LpmChanged(enabled) => {
                self.lpm_flag.store(enabled, Ordering::Relaxed);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::ModifiersChanged(modifiers) => {
                self.mods = ModState::from_winit(modifiers.state());
            }
            WindowEvent::KeyboardInput { event, .. } => {
                // Cmd-V: read NSPasteboard + wrap in bracketed paste markers (D-53).
                // Cmd-C deferred to Phase 5 per D-53.
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
                if let Some(bytes) = encode_key(&event, self.mods) {
                    self.input_bridge.send_bytes(bytes);
                    self.request_redraw();
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
                    self.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_px = position;
                if matches!(self.input_bridge.selection, SelectionState::Dragging(_)) {
                    if let Some(cell) = self.cell_from_pixel(position) {
                        self.input_bridge.selection.mouse_move(cell);
                        self.request_redraw();
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
                    self.request_redraw();
                }
            }
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::PixelDelta(pos),
                ..
            } => {
                if let Some(host) = self.render_host.as_ref() {
                    if let Some((_cw, ch)) = host.cell_metrics_px() {
                        #[allow(clippy::cast_possible_truncation)]
                        let lines = (pos.y / f64::from(ch.max(1))) as i32;
                        if lines != 0 {
                            {
                                let mut t = self.term.lock();
                                t.scroll_display(lines);
                            }
                            self.request_redraw();
                        }
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(host) = self.render_host.as_mut() {
                    #[allow(clippy::cast_possible_truncation)]
                    let dpr = scale_factor as f32;
                    host.clear_atlases();
                    host.set_dpr(dpr);
                }
                self.request_redraw();
                tracing::info!(scale_factor, "DPR change; cleared atlases (D-48)");
            }
            WindowEvent::Resized(size) => {
                // wgpu surface reconfigures on every event (cheap); Term::resize debounces 50ms.
                if let Some(host) = self.render_host.as_mut() {
                    host.resize(size.width, size.height);
                }
                if let Some(overlay) = self.overlay.as_mut() {
                    overlay.relayout();
                }
                if let Some(host) = self.render_host.as_ref() {
                    if let Some((cell_w, cell_h)) = host.cell_metrics_px() {
                        let cols =
                            u16::try_from((size.width / cell_w.max(1)).max(1)).unwrap_or(u16::MAX);
                        let rows =
                            u16::try_from((size.height / cell_h.max(1)).max(1)).unwrap_or(u16::MAX);
                        self.pending_resize = Some((rows, cols));
                        self.last_resize_at = Some(Instant::now());
                    }
                }
                self.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                // D-51: gate first paint until shell + PTY + font + dirty row ready.
                if !self.first_paint_ready {
                    return;
                }
                // D-49: flush pending Term::resize if quiescent.
                self.flush_pending_resize_if_quiescent();
                if let Some(host) = self.render_host.as_mut() {
                    let sel = self
                        .input_bridge
                        .selection
                        .range()
                        .map(|r| (r.anchor, r.cursor));
                    let mut t = self.term.lock();
                    if let Err(err) = host.render(&mut t, sel) {
                        tracing::warn!(?err, "render failed");
                    }
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
