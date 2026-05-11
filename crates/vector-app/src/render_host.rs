//! Owns the wgpu surface + lazy compositor. Plan 03-03 wires the cell + cursor pipelines.

use std::sync::Arc;

use anyhow::Result;
use vector_fonts::FontStack;
use vector_render::{Compositor, CompositorError, RenderContext};
use vector_term::Term;
use winit::window::Window;

pub struct RenderHost {
    ctx: RenderContext,
    compositor: Option<Compositor>,
    compositor_failed: bool,
}

impl RenderHost {
    pub fn new(window: &Arc<Window>) -> Result<Self> {
        Ok(Self {
            ctx: RenderContext::new(window)?,
            compositor: None,
            compositor_failed: false,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.ctx.resize(width, height);
        if let Some(comp) = self.compositor.as_mut() {
            let cols = (width / comp.cell_width_px()).max(1);
            let rows = (height / comp.cell_height_px()).max(1);
            let cols = u16::try_from(cols).unwrap_or(u16::MAX);
            let rows = u16::try_from(rows).unwrap_or(u16::MAX);
            comp.resize(&self.ctx, cols, rows);
        }
    }

    /// (cell_width_px, cell_height_px) once the compositor is initialized. None before then.
    pub fn cell_metrics_px(&self) -> Option<(u32, u32)> {
        self.compositor
            .as_ref()
            .map(|c| (c.cell_width_px(), c.cell_height_px()))
    }

    /// xterm-256 dark default — used as a fallback before the compositor exists or if its init failed.
    pub fn render_clear_default(&self) -> Result<()> {
        self.ctx.render_clear(&[0.06, 0.06, 0.06, 1.0])
    }

    fn ensure_compositor(&mut self) {
        if self.compositor.is_some() || self.compositor_failed {
            return;
        }
        match FontStack::load_bundled(1.0, 14.0).and_then(|fs| Compositor::new(&self.ctx, fs)) {
            Ok(c) => self.compositor = Some(c),
            Err(err) => {
                tracing::error!(?err, "compositor init failed; falling back to clear color");
                self.compositor_failed = true;
            }
        }
    }

    /// Render via Compositor if available, else clear-color fallback.
    pub fn render(
        &mut self,
        term: &mut Term,
        selection: Option<((u16, u16), (u16, u16))>,
    ) -> Result<()> {
        self.ensure_compositor();
        let Some(comp) = self.compositor.as_mut() else {
            return self.render_clear_default();
        };
        match comp.render(&self.ctx, term, selection) {
            // Outdated/Lost: surface was reconfigured by Compositor::render; retry next redraw.
            Ok(()) | Err(CompositorError::Outdated | CompositorError::Lost) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("compositor render: {err}")),
        }
    }
}
