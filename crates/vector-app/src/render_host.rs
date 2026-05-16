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
    dpr: f32,
}

/// Surface frame handle yielded by `RenderHost::with_frame`. Plan 04-06: per-pane
/// render loop acquires the surface once, then iterates compositors against `view`
/// with chained LoadOps; caller calls `present()` after the last pane is encoded.
pub struct AcquiredFrame {
    frame: wgpu::SurfaceTexture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl AcquiredFrame {
    pub fn present(self) {
        self.frame.present();
    }
}

impl RenderHost {
    pub fn new(window: &Arc<Window>) -> Result<Self> {
        #[allow(clippy::cast_possible_truncation)]
        let dpr = window.scale_factor() as f32;
        Ok(Self {
            ctx: RenderContext::new(window)?,
            compositor: None,
            compositor_failed: false,
            dpr,
        })
    }

    /// D-48: clear both atlases; next frame lazy-rasterizes glyphs at the new DPR.
    pub fn clear_atlases(&mut self) {
        if let Some(comp) = self.compositor.as_mut() {
            comp.clear_atlases();
        }
    }

    /// Record the current device-pixel ratio; future re-rasterization uses this bucket.
    pub fn set_dpr(&mut self, dpr: f32) {
        self.dpr = dpr.max(1.0);
    }

    #[allow(dead_code)]
    pub fn dpr(&self) -> f32 {
        self.dpr
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

    /// Plan 04-06: read access to the underlying wgpu queue. Per-pane border-color
    /// updates from the App's MuxCommand::FocusDir handler call `Compositor::set_border_color`
    /// which needs the queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.ctx.queue
    }

    /// Plan 04-06: read access to device for lazy per-pane Compositor construction.
    pub fn device(&self) -> &wgpu::Device {
        &self.ctx.device
    }

    /// Plan 04-06: surface format for `Compositor::new_with_viewport`.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.ctx.config.format
    }

    /// Plan 04-06: surface dimensions (width, height) in physical pixels.
    pub fn surface_size(&self) -> (u32, u32) {
        (self.ctx.config.width, self.ctx.config.height)
    }

    /// Plan 04-06: acquire the next surface frame. Returns `Ok(None)` when the
    /// surface is Occluded/Timeout/Outdated/Lost (caller skips the frame and the
    /// surface auto-reconfigures for Outdated/Lost).
    pub fn acquire_frame(&mut self) -> Result<Option<AcquiredFrame>> {
        let frame = match self.ctx.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.ctx
                    .surface
                    .configure(&self.ctx.device, &self.ctx.config);
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(anyhow::anyhow!("surface validation error"));
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Some(AcquiredFrame {
            frame,
            view,
            width: self.ctx.config.width,
            height: self.ctx.config.height,
        }))
    }

    /// Plan 04-06: build a fresh Compositor against this host's device + surface
    /// format. Used to populate the per-pane `compositors` map lazily.
    pub fn new_compositor_for_viewport(
        &self,
        offset_px: [f32; 2],
        size_px: [f32; 2],
    ) -> Result<Compositor> {
        let fs = FontStack::load_bundled(self.dpr, 14.0)?;
        Compositor::new_with_viewport(
            &self.ctx.device,
            &self.ctx.queue,
            self.ctx.config.format,
            self.ctx.config.width,
            self.ctx.config.height,
            offset_px,
            size_px,
            fs,
        )
    }
}
