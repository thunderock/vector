use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::{menu, overlay, UserEvent};

pub struct App {
    window: Option<Window>,
    overlay: Option<overlay::Overlay>,
}

impl App {
    pub fn new() -> Self {
        Self {
            window: None,
            overlay: None,
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
        let window = event_loop.create_window(attrs).expect("create_window");

        // SAFETY: winit guarantees `resumed` runs on the macOS main thread.
        unsafe {
            menu::install_main_menu();
            self.overlay = Some(overlay::install(&window));
        }
        self.window = Some(window);
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Tick(n) => {
                if let Some(window) = self.window.as_ref() {
                    window.set_title(&format!("Vector \u{2014} tick {n}"));
                }
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_size) => {
                if let Some(overlay) = self.overlay.as_mut() {
                    overlay.relayout();
                }
            }
            _ => {}
        }
    }
}
