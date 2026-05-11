use std::sync::Arc;

use parking_lot::Mutex;
use vector_term::Term;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::{menu, overlay, render_host::RenderHost, UserEvent};

pub struct App {
    window: Option<Arc<Window>>,
    overlay: Option<overlay::Overlay>,
    overlay_dropped: bool,
    term: Arc<Mutex<Term>>,
    render_host: Option<RenderHost>,
}

impl App {
    pub fn new() -> Self {
        Self {
            window: None,
            overlay: None,
            overlay_dropped: false,
            term: Arc::new(Mutex::new(Term::new(80, 24, 10_000))),
            render_host: None,
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
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
                if let Some(w) = self.window.as_ref() {
                    w.request_redraw();
                }
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(host) = self.render_host.as_mut() {
                    host.resize(size.width, size.height);
                }
                if let Some(overlay) = self.overlay.as_mut() {
                    overlay.relayout();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(host) = self.render_host.as_mut() {
                    if let Err(err) = host.render_clear_default() {
                        tracing::warn!(?err, "render_clear failed");
                    }
                }
            }
            _ => {}
        }
    }
}
