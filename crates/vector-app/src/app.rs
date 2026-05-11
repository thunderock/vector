use std::sync::Arc;

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

pub struct App {
    window: Option<Arc<Window>>,
    overlay: Option<overlay::Overlay>,
    overlay_dropped: bool,
    term: Arc<Mutex<Term>>,
    render_host: Option<RenderHost>,
    input_bridge: InputBridge,
    mods: ModState,
    cursor_px: PhysicalPosition<f64>,
}

impl App {
    pub fn new(write_tx: mpsc::Sender<Vec<u8>>, resize_tx: mpsc::Sender<(u16, u16)>) -> Self {
        Self {
            window: None,
            overlay: None,
            overlay_dropped: false,
            term: Arc::new(Mutex::new(Term::new(80, 24, 10_000))),
            render_host: None,
            input_bridge: InputBridge::new(write_tx, resize_tx),
            mods: ModState::default(),
            cursor_px: PhysicalPosition::new(0.0, 0.0),
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
            UserEvent::Tick(_) => {}
            UserEvent::PtyOutput(bytes) => {
                {
                    let mut t = self.term.lock();
                    t.feed(&bytes);
                }
                if !self.overlay_dropped {
                    self.overlay = None;
                    self.overlay_dropped = true;
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
        }
    }

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
                // Plan 03-05 ratifies scrollback wiring; vector-term doesn't expose scroll_display yet.
                tracing::debug!(y_lines = y, "scrollback offset deferred to Plan 03-05");
            }
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::PixelDelta(pos),
                ..
            } => {
                tracing::debug!(y_px = pos.y, "scrollback offset deferred to Plan 03-05");
            }
            WindowEvent::Resized(size) => {
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
                        self.input_bridge.send_resize(rows, cols);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
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
