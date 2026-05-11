//! Owns the wgpu surface + clear-color default. Plan 03-03 extends with the cell compositor.

use std::sync::Arc;

use anyhow::Result;
use vector_render::RenderContext;
use winit::window::Window;

pub struct RenderHost {
    ctx: RenderContext,
}

impl RenderHost {
    pub fn new(window: &Arc<Window>) -> Result<Self> {
        Ok(Self {
            ctx: RenderContext::new(window)?,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.ctx.resize(width, height);
    }

    /// xterm-256 dark default; Plan 03-05 promotes to a theme uniform.
    pub fn render_clear_default(&self) -> Result<()> {
        self.ctx.render_clear(&[0.06, 0.06, 0.06, 1.0])
    }
}
