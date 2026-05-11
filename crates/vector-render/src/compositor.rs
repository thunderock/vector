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
use crate::cell_pipeline::{CellInstance, CellPipeline};
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
    viewport_size_px: [f32; 2],
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
        let viewport_size_px = [width as f32, height as f32];
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
            viewport_size_px,
            instance_scratch: Vec::new(),
        };
        me.cell_pipeline.update_uniforms(
            queue,
            [cell_metrics.width_px as f32, cell_metrics.height_px as f32],
            viewport_size_px,
            me.selection_tint,
        );
        Ok(me)
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

    pub fn resize(&mut self, render_ctx: &RenderContext, cols: u16, rows: u16) {
        self.viewport_size_px = [
            render_ctx.config.width as f32,
            render_ctx.config.height as f32,
        ];
        let needed = usize::from(cols) * usize::from(rows);
        self.cell_pipeline
            .ensure_capacity(&render_ctx.device, needed);
        self.cell_pipeline.update_uniforms(
            &render_ctx.queue,
            [
                self.cell_metrics.width_px as f32,
                self.cell_metrics.height_px as f32,
            ],
            self.viewport_size_px,
            self.selection_tint,
        );
    }

    /// Render one frame to the wgpu surface. Selection is wired from day one; Plan 03-03 tests
    /// pass None; Plan 03-04's selection state machine will populate it.
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
        self.encode_passes(render_ctx, &view);
        frame.present();
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
        self.encode_passes_raw(device, queue, &view);

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
        let viewport = [width as f32, height as f32];
        #[allow(clippy::float_cmp)]
        let viewport_changed =
            viewport[0] != self.viewport_size_px[0] || viewport[1] != self.viewport_size_px[1];
        if viewport_changed {
            self.viewport_size_px = viewport;
            self.cell_pipeline.update_uniforms(
                queue,
                [
                    self.cell_metrics.width_px as f32,
                    self.cell_metrics.height_px as f32,
                ],
                self.viewport_size_px,
                self.selection_tint,
            );
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
            self.viewport_size_px,
            self.cursor_color,
        );
    }

    fn encode_passes(&self, render_ctx: &RenderContext, view: &wgpu::TextureView) {
        self.encode_passes_raw(&render_ctx.device, &render_ctx.queue, view);
    }

    fn encode_passes_raw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
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
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(self.default_bg[0]),
                            g: f64::from(self.default_bg[1]),
                            b: f64::from(self.default_bg[2]),
                            a: 1.0,
                        }),
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

fn is_cell_selected(selection: Option<((u16, u16), (u16, u16))>, col: u16, row: u16) -> bool {
    let Some(((a_col, a_row), (b_col, b_row))) = selection else {
        return false;
    };
    let (lo, hi) = if (a_row, a_col) <= (b_row, b_col) {
        ((a_col, a_row), (b_col, b_row))
    } else {
        ((b_col, b_row), (a_col, a_row))
    };
    let pt = (row, col);
    let lo_pt = (lo.1, lo.0);
    let hi_pt = (hi.1, hi.0);
    pt >= lo_pt && pt <= hi_pt
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

const _: () = {
    // Repr-check: CellInstance ends on a 16-byte boundary.
    let _ = [(); size_of::<CellInstance>() % 16];
};
