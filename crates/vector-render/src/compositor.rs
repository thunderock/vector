//! Grid → quads compositor consuming `vector_term::Term::damage()`. Plan 03-03 (RENDER-01/04/05).

#![allow(
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::items_after_statements
)]

use std::mem::size_of;

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::vte::ansi::{Color, NamedColor, Rgb};
use anyhow::Result;
use vector_fonts::{CellMetrics, FontStack};
use vector_term::{Term, TermDamage};

use crate::atlas::{Atlas, AtlasSlot, GlyphKey};
use crate::cell_pipeline::{CellInstance, CellPipeline, Uniforms as CellUniforms};
use crate::cursor_pipeline::CursorPipeline;
use crate::pipeline::RenderContext;

/// Recoverable surface acquisition states. `Outdated`/`Lost` mean we reconfigured the surface
/// and the caller should retry; `Timeout` is transient; `Validation` is fatal (logged by caller).
/// Replaces wgpu 28's removed `SurfaceError` for our render path. D-11 / Open Question #4.
#[derive(Debug, thiserror::Error)]
pub enum CompositorError {
    #[error("surface outdated; reconfigured")]
    Outdated,
    #[error("surface lost; reconfigured")]
    Lost,
    #[error("surface acquire timeout")]
    Timeout,
    #[error("surface validation error")]
    Validation,
}

/// xterm-ish translucent blue. Final value Claude's discretion (D-54 selection tint).
const SELECTION_TINT: [f32; 4] = [0.27, 0.48, 0.78, 0.40];
/// xterm-256 default dark background.
const DEFAULT_BG: [f32; 4] = [0.06, 0.06, 0.06, 1.0];
/// Light gray foreground.
const DEFAULT_FG: [f32; 4] = [0.85, 0.85, 0.85, 1.0];
/// Block-cursor color. Plan 03-05 may promote to a theme uniform; blink also lands there.
const CURSOR_COLOR: [f32; 4] = [0.85, 0.85, 0.85, 1.0];
/// D-66 active-pane border default thickness (px).
pub const DEFAULT_BORDER_WIDTH_PX: f32 = 2.0;

pub struct Compositor {
    cell_pipeline: CellPipeline,
    cursor_pipeline: CursorPipeline,
    atlas: Atlas,
    font_stack: FontStack,
    cell_metrics: CellMetrics,
    palette_256: [[f32; 4]; 256],
    default_fg: [f32; 4],
    default_bg: [f32; 4],
    selection_tint: [f32; 4],
    cursor_color: [f32; 4],
    surface_format: wgpu::TextureFormat,
    /// Full window surface dimensions; used for NDC conversion.
    window_size_px: [f32; 2],
    /// Per-pane viewport offset within the window (Plan 04-04).
    viewport_offset_px: [f32; 2],
    /// Per-pane viewport size; may equal window_size_px (single-pane mode).
    viewport_size_px: [f32; 2],
    /// Active-pane border color (D-66). Alpha 0 disables the border.
    border_color: [f32; 4],
    border_width_px: f32,
    /// false → hollow/outline cursor (inactive pane); true → filled rect.
    cursor_focused: bool,
    instance_scratch: Vec<CellInstance>,
}

impl Compositor {
    pub fn new(render_ctx: &RenderContext, font_stack: FontStack) -> Result<Self> {
        Self::new_with(
            &render_ctx.device,
            &render_ctx.queue,
            render_ctx.config.format,
            render_ctx.config.width,
            render_ctx.config.height,
            font_stack,
        )
    }

    /// Build a Compositor against a raw device + queue + surface format. Plan 03-03 tests use
    /// `RenderContext::new_offscreen` to get the device/queue pair without a window.
    /// Plan 04-04: viewport offset defaults to (0,0) and viewport size to (width,height) —
    /// single-pane behavior. Per-pane callers use `set_viewport`.
    pub fn new_with(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        font_stack: FontStack,
    ) -> Result<Self> {
        let cell_metrics = font_stack.cell_metrics;
        let atlas = Atlas::new(device);
        let cell_pipeline = CellPipeline::new(
            device,
            surface_format,
            atlas.mono_view(),
            atlas.color_view(),
            16_000,
        );
        let cursor_pipeline = CursorPipeline::new(device, surface_format);
        let size = [width as f32, height as f32];
        let palette_256 = xterm_256_palette();
        let me = Self {
            cell_pipeline,
            cursor_pipeline,
            atlas,
            font_stack,
            cell_metrics,
            palette_256,
            default_fg: DEFAULT_FG,
            default_bg: DEFAULT_BG,
            selection_tint: SELECTION_TINT,
            cursor_color: CURSOR_COLOR,
            surface_format,
            window_size_px: size,
            viewport_offset_px: [0.0, 0.0],
            viewport_size_px: size,
            border_color: [0.0, 0.0, 0.0, 0.0],
            border_width_px: DEFAULT_BORDER_WIDTH_PX,
            cursor_focused: true,
            instance_scratch: Vec::new(),
        };
        me.write_cell_uniforms(queue);
        Ok(me)
    }

    /// Build a Compositor with explicit window dimensions, viewport offset, and viewport size.
    /// Plan 04-04 per-pane callers use this form.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_viewport(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        window_width: u32,
        window_height: u32,
        viewport_offset_px: [f32; 2],
        viewport_size_px: [f32; 2],
        font_stack: FontStack,
    ) -> Result<Self> {
        let mut c = Self::new_with(
            device,
            queue,
            surface_format,
            window_width,
            window_height,
            font_stack,
        )?;
        c.viewport_offset_px = viewport_offset_px;
        c.viewport_size_px = viewport_size_px;
        c.write_cell_uniforms(queue);
        Ok(c)
    }

    fn current_uniforms(&self) -> CellUniforms {
        CellUniforms {
            window_size_px: self.window_size_px,
            cell_size_px: [
                self.cell_metrics.width_px as f32,
                self.cell_metrics.height_px as f32,
            ],
            selection_tint: self.selection_tint,
            border_color: self.border_color,
            viewport_offset_px: self.viewport_offset_px,
            viewport_size_px: self.viewport_size_px,
            border_width_px: self.border_width_px,
            _pad0: 0.0,
            _pad1: [0.0, 0.0],
        }
    }

    fn write_cell_uniforms(&self, queue: &wgpu::Queue) {
        let u = self.current_uniforms();
        self.cell_pipeline.update_uniforms(queue, &u);
    }

    /// Plan 04-04: set per-pane viewport offset + size, re-upload uniforms.
    pub fn set_viewport(&mut self, queue: &wgpu::Queue, offset_px: [f32; 2], size_px: [f32; 2]) {
        self.viewport_offset_px = offset_px;
        self.viewport_size_px = size_px;
        self.write_cell_uniforms(queue);
    }

    /// Plan 04-04 (D-66): set active-pane border color. Alpha 0 disables.
    pub fn set_border_color(&mut self, queue: &wgpu::Queue, color: [f32; 4]) {
        self.border_color = color;
        self.write_cell_uniforms(queue);
    }

    /// Plan 04-04: focused pane → filled cursor (true); inactive → hollow outline (false).
    pub fn set_cursor_focused(&mut self, focused: bool) {
        self.cursor_focused = focused;
    }

    #[must_use]
    pub fn viewport_offset_px(&self) -> [f32; 2] {
        self.viewport_offset_px
    }

    #[must_use]
    pub fn viewport_size_px(&self) -> [f32; 2] {
        self.viewport_size_px
    }

    #[must_use]
    pub fn border_color(&self) -> [f32; 4] {
        self.border_color
    }

    pub fn cell_width_px(&self) -> u32 {
        self.cell_metrics.width_px.max(1)
    }

    pub fn cell_height_px(&self) -> u32 {
        self.cell_metrics.height_px.max(1)
    }

    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_format
    }

    /// Plan 03-05's ScaleFactorChanged hook (D-48). Public access path for plan 03-05.
    pub fn atlas_mut(&mut self) -> &mut Atlas {
        &mut self.atlas
    }

    /// D-48: clear both atlases on DPR change; lazy re-rasterize on next frame.
    pub fn clear_atlases(&mut self) {
        self.atlas.clear_all();
    }

    /// Record the DPR bucket for future glyph re-rasterization. Cell metrics are
    /// already in pixel units; this is a forward hook for Plan 04+ multi-DPR.
    pub fn set_dpr(&mut self, _dpr: f32) {}

    pub fn resize(&mut self, render_ctx: &RenderContext, cols: u16, rows: u16) {
        let new_size = [
            render_ctx.config.width as f32,
            render_ctx.config.height as f32,
        ];
        self.window_size_px = new_size;
        // Single-pane callers keep viewport == window; multi-pane callers will
        // call set_viewport explicitly after this.
        if self.viewport_offset_px == [0.0, 0.0] {
            self.viewport_size_px = new_size;
        }
        let needed = usize::from(cols) * usize::from(rows);
        self.cell_pipeline
            .ensure_capacity(&render_ctx.device, needed);
        self.write_cell_uniforms(&render_ctx.queue);
    }

    /// Render one frame to the wgpu surface. Selection is wired from day one; Plan 03-03 tests
    /// pass None; Plan 03-04's selection state machine will populate it. Single-pane callers
    /// keep the surface clear color (LoadOp::Clear). Plan 04-04 multi-pane callers use
    /// `render_into_view` for finer control over surface acquisition + LoadOp.
    pub fn render(
        &mut self,
        render_ctx: &RenderContext,
        term: &mut Term,
        selection: Option<((u16, u16), (u16, u16))>,
    ) -> Result<(), CompositorError> {
        self.prepare_frame(render_ctx, term, selection);
        let frame = match render_ctx.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated => {
                render_ctx
                    .surface
                    .configure(&render_ctx.device, &render_ctx.config);
                return Err(CompositorError::Outdated);
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                render_ctx
                    .surface
                    .configure(&render_ctx.device, &render_ctx.config);
                return Err(CompositorError::Lost);
            }
            wgpu::CurrentSurfaceTexture::Timeout => return Err(CompositorError::Timeout),
            wgpu::CurrentSurfaceTexture::Occluded => return Ok(()),
            wgpu::CurrentSurfaceTexture::Validation => {
                tracing::error!("surface validation error");
                return Err(CompositorError::Validation);
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let load_op = wgpu::LoadOp::Clear(wgpu::Color {
            r: f64::from(self.default_bg[0]),
            g: f64::from(self.default_bg[1]),
            b: f64::from(self.default_bg[2]),
            a: 1.0,
        });
        self.encode_passes_with(&render_ctx.device, &render_ctx.queue, &view, load_op);
        frame.present();
        Ok(())
    }

    /// Plan 04-04: render this compositor's pane into the provided view + queue + device.
    /// The caller acquires/presents the surface texture and orchestrates LoadOp across panes.
    /// First pane per frame passes `LoadOp::Clear`; subsequent panes pass `LoadOp::Load`.
    #[allow(clippy::too_many_arguments)]
    pub fn render_into_view(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        window_width: u32,
        window_height: u32,
        term: &mut Term,
        selection: Option<((u16, u16), (u16, u16))>,
        load_op: wgpu::LoadOp<wgpu::Color>,
    ) -> anyhow::Result<()> {
        self.prepare_frame_raw(device, queue, window_width, window_height, term, selection);
        self.encode_passes_with(device, queue, view, load_op);
        Ok(())
    }

    /// Render to an internally-owned offscreen Rgba8Unorm texture and read back pixel bytes.
    /// Used by Plan 03-03 Task 2 pixel-snapshot tests. Does NOT acquire the surface — tests can
    /// build the compositor against a `RenderContext` with any (or no real) surface.
    pub fn render_offscreen(
        &mut self,
        render_ctx: &RenderContext,
        term: &mut Term,
        selection: Option<((u16, u16), (u16, u16))>,
    ) -> anyhow::Result<OffscreenFrame> {
        self.render_offscreen_with(
            &render_ctx.device,
            &render_ctx.queue,
            render_ctx.config.width,
            render_ctx.config.height,
            term,
            selection,
        )
    }

    /// Surface-free variant of `render_offscreen`. Lets headless tests build a Device + Queue
    /// (via `Adapter::request_device`) and run the compositor without instantiating a window.
    pub fn render_offscreen_with(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        term: &mut Term,
        selection: Option<((u16, u16), (u16, u16))>,
    ) -> anyhow::Result<OffscreenFrame> {
        let width = width.max(1);
        let height = height.max(1);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("compositor-offscreen"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.surface_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.prepare_frame_raw(device, queue, width, height, term, selection);
        let load_op = wgpu::LoadOp::Clear(wgpu::Color {
            r: f64::from(self.default_bg[0]),
            g: f64::from(self.default_bg[1]),
            b: f64::from(self.default_bg[2]),
            a: 1.0,
        });
        self.encode_passes_with(device, queue, &view, load_op);

        // Copy out via padded staging buffer (256-byte row alignment per wgpu spec).
        let bytes_per_pixel: u32 = 4;
        let unpadded_bpr = width * bytes_per_pixel;
        let align: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bpr = unpadded_bpr.div_ceil(align) * align;
        let buf_size = u64::from(padded_bpr) * u64::from(height);
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("offscreen-staging"),
            size: buf_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("offscreen-copy"),
        });
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bpr),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(enc.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| anyhow::anyhow!("device poll: {e:?}"))?;
        rx.recv()
            .map_err(|e| anyhow::anyhow!("map_async channel: {e}"))?
            .map_err(|e| anyhow::anyhow!("map_async: {e:?}"))?;
        let data = slice.get_mapped_range();

        // De-pad rows.
        let mut pixels = Vec::with_capacity((unpadded_bpr * height) as usize);
        for row in 0..height {
            let off = (row * padded_bpr) as usize;
            pixels.extend_from_slice(&data[off..off + unpadded_bpr as usize]);
        }
        drop(data);
        staging.unmap();
        Ok(OffscreenFrame {
            width,
            height,
            pixels,
            format: self.surface_format,
        })
    }

    fn prepare_frame(
        &mut self,
        render_ctx: &RenderContext,
        term: &mut Term,
        selection: Option<((u16, u16), (u16, u16))>,
    ) {
        self.prepare_frame_raw(
            &render_ctx.device,
            &render_ctx.queue,
            render_ctx.config.width,
            render_ctx.config.height,
            term,
            selection,
        );
    }

    fn prepare_frame_raw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        term: &mut Term,
        selection: Option<((u16, u16), (u16, u16))>,
    ) {
        let (cols, rows) = term.dims();
        let cursor = term.cursor();
        let window_size = [width as f32, height as f32];
        #[allow(clippy::float_cmp)]
        let window_changed =
            window_size[0] != self.window_size_px[0] || window_size[1] != self.window_size_px[1];
        if window_changed {
            self.window_size_px = window_size;
            // Single-pane case keeps viewport == window.
            if self.viewport_offset_px == [0.0, 0.0] {
                self.viewport_size_px = window_size;
            }
            self.write_cell_uniforms(queue);
        }
        let needed = usize::from(cols) * usize::from(rows);
        self.cell_pipeline.ensure_capacity(device, needed);

        // Snapshot damage + reset; full rebuild for Plan 03-03 simplicity.
        let _damage_rows: Vec<(u16, u16, u16)> = match term.damage() {
            TermDamage::Full => (0..rows)
                .map(|r| (r, 0u16, cols.saturating_sub(1)))
                .collect(),
            TermDamage::Partial(iter) => iter
                .map(|b| {
                    (
                        u16::try_from(b.line).unwrap_or(u16::MAX),
                        u16::try_from(b.left).unwrap_or(u16::MAX),
                        u16::try_from(b.right).unwrap_or(u16::MAX),
                    )
                })
                .collect(),
        };
        term.reset_damage();

        self.instance_scratch.clear();
        self.instance_scratch
            .reserve(usize::from(cols) * usize::from(rows));
        let grid = term.grid();
        let _ = grid.total_lines();
        for r in 0..rows {
            for c in 0..cols {
                let point = Point::new(Line(i32::from(r)), Column(usize::from(c)));
                let cell = &grid[point];
                if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                    // Pitfall 4 — wide-char continuation; lead cell paints the glyph.
                    continue;
                }
                let inverse = cell.flags.contains(Flags::INVERSE);
                let bold = cell.flags.contains(Flags::BOLD);
                let mut flags = 0u32;
                if inverse {
                    flags |= 1;
                }
                if bold {
                    flags |= 2;
                }
                let fg =
                    color_to_rgba(cell.fg, &self.palette_256, self.default_fg, self.default_bg);
                let bg =
                    color_to_rgba(cell.bg, &self.palette_256, self.default_fg, self.default_bg);
                let (atlas_kind, uv) = if cell.c == ' ' || cell.c == '\0' {
                    (2u32, [0.0; 4])
                } else {
                    match self.font_stack.rasterize(cell.c) {
                        Ok(glyph) => {
                            let key = GlyphKey {
                                character: cell.c,
                                dpr_bucket: 1,
                            };
                            match self.atlas.slot_for(queue, key, &glyph) {
                                AtlasSlot::Mono { uv, .. } => (0u32, uv),
                                AtlasSlot::Color { uv, .. } => (1u32, uv),
                                AtlasSlot::Fallback => (2u32, [0.0; 4]),
                            }
                        }
                        Err(_) => (2u32, [0.0; 4]),
                    }
                };
                let selected = u32::from(is_cell_selected(selection, c, r));
                self.instance_scratch.push(CellInstance {
                    cell_pos: [u32::from(c), u32::from(r)],
                    fg,
                    bg,
                    uv,
                    atlas_kind,
                    selected,
                    flags,
                    _pad: 0,
                });
            }
        }
        self.cell_pipeline
            .upload_instances(queue, &self.instance_scratch, 0);
        self.cursor_pipeline.update(
            queue,
            [u32::from(cursor.0), u32::from(cursor.1)],
            [
                self.cell_metrics.width_px as f32,
                self.cell_metrics.height_px as f32,
            ],
            self.window_size_px,
            self.viewport_offset_px,
            self.cursor_color,
            self.cursor_focused,
        );
    }

    fn encode_passes_with(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        cell_load: wgpu::LoadOp<wgpu::Color>,
    ) {
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("compositor-encoder"),
        });
        {
            let mut rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("cell-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: cell_load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let count = u32::try_from(self.instance_scratch.len()).unwrap_or(u32::MAX);
            self.cell_pipeline.draw(&mut rpass, count);
        }
        {
            let mut rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("cursor-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.cursor_pipeline.draw(&mut rpass);
        }
        queue.submit(Some(enc.finish()));
    }
}

/// Read-back result from `Compositor::render_offscreen`.
#[derive(Debug, Clone)]
pub struct OffscreenFrame {
    pub width: u32,
    pub height: u32,
    /// Tightly-packed pixels in the surface format (typically `Bgra8UnormSrgb` or `Rgba8Unorm`).
    pub pixels: Vec<u8>,
    pub format: wgpu::TextureFormat,
}

/// Row-major selection test. Mirrors `vector_input::SelectionRange::cells` — intentional
/// duplicate to avoid a vector-render → vector-input dep edge. Selection covers the partial
/// first row (anchor→EOL), full intervening rows, and partial last row (BOL→cursor).
fn is_cell_selected(selection: Option<((u16, u16), (u16, u16))>, col: u16, row: u16) -> bool {
    let Some(((a_col, a_row), (b_col, b_row))) = selection else {
        return false;
    };
    // Normalize so (start_row, start_col) <= (end_row, end_col) in row-major order.
    let (start_row, start_col, end_row, end_col) = if (a_row, a_col) <= (b_row, b_col) {
        (a_row, a_col, b_row, b_col)
    } else {
        (b_row, b_col, a_row, a_col)
    };
    if row < start_row || row > end_row {
        return false;
    }
    if start_row == end_row {
        col >= start_col && col <= end_col
    } else if row == start_row {
        col >= start_col
    } else if row == end_row {
        col <= end_col
    } else {
        true
    }
}

/// Resolve an alacritty `Color` into linear-ish [r,g,b,a] floats. Plan 03-03 uses sRGB-as-linear
/// (no gamma correction); Plan 03-05 may revisit once we have a theme uniform.
pub(crate) fn color_to_rgba(
    c: Color,
    palette: &[[f32; 4]; 256],
    default_fg: [f32; 4],
    default_bg: [f32; 4],
) -> [f32; 4] {
    match c {
        Color::Named(n) => match n {
            NamedColor::Foreground | NamedColor::BrightForeground | NamedColor::DimForeground => {
                default_fg
            }
            NamedColor::Background => default_bg,
            NamedColor::Cursor => default_fg,
            other => {
                let idx = other as usize;
                if idx < 256 {
                    palette[idx]
                } else {
                    default_fg
                }
            }
        },
        Color::Spec(Rgb { r, g, b }) => [
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            1.0,
        ],
        Color::Indexed(i) => palette[i as usize],
    }
}

/// xterm 256-color palette: 16 ANSI + 6×6×6 cube + 24 grayscale ramp.
/// Source: xterm 256-color palette
/// (https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit; verified against xterm sources).
pub(crate) fn xterm_256_palette() -> [[f32; 4]; 256] {
    let mut out = [[0.0f32; 4]; 256];
    // 16 ANSI base colors (xterm defaults).
    const ANSI: [[u8; 3]; 16] = [
        [0x00, 0x00, 0x00],
        [0xcd, 0x00, 0x00],
        [0x00, 0xcd, 0x00],
        [0xcd, 0xcd, 0x00],
        [0x00, 0x00, 0xee],
        [0xcd, 0x00, 0xcd],
        [0x00, 0xcd, 0xcd],
        [0xe5, 0xe5, 0xe5],
        [0x7f, 0x7f, 0x7f],
        [0xff, 0x00, 0x00],
        [0x00, 0xff, 0x00],
        [0xff, 0xff, 0x00],
        [0x5c, 0x5c, 0xff],
        [0xff, 0x00, 0xff],
        [0x00, 0xff, 0xff],
        [0xff, 0xff, 0xff],
    ];
    for (i, rgb) in ANSI.iter().enumerate() {
        out[i] = [
            f32::from(rgb[0]) / 255.0,
            f32::from(rgb[1]) / 255.0,
            f32::from(rgb[2]) / 255.0,
            1.0,
        ];
    }
    // 6×6×6 cube starting at index 16.
    const CUBE_STEPS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    for r in 0..6u8 {
        for g in 0..6u8 {
            for b in 0..6u8 {
                let idx = 16 + 36 * usize::from(r) + 6 * usize::from(g) + usize::from(b);
                out[idx] = [
                    f32::from(CUBE_STEPS[r as usize]) / 255.0,
                    f32::from(CUBE_STEPS[g as usize]) / 255.0,
                    f32::from(CUBE_STEPS[b as usize]) / 255.0,
                    1.0,
                ];
            }
        }
    }
    // 24-step grayscale ramp starting at index 232.
    for i in 0..24u8 {
        let raw = 8u32 + 10 * u32::from(i);
        let clamped = u8::try_from(raw.min(255)).unwrap_or(255);
        let v = f32::from(clamped) / 255.0;
        out[232 + usize::from(i)] = [v, v, v, 1.0];
    }
    out
}

// CellInstance is 72 bytes (8+16+16+16+4+4+4+4) — divisible by 8, naga accepts the layout.
// WGSL needs each instance attribute aligned to its component size; locations are u32x2 / f32x4
// / u32 — all within naga's relaxed instance-stride rules at 72 bytes/instance.
const _: () = assert!(size_of::<CellInstance>() == 72);
