//! Shared headless render harness for snapshot tests. No winit window — `RenderContext::
//! new_offscreen` builds device + queue directly via Metal adapter request.

#![allow(dead_code, clippy::missing_panics_doc)]

use vector_fonts::FontStack;
use vector_render::{Compositor, Offscreen};

/// Probes for a Metal adapter; returns None on hosts without one (Linux dev shells).
/// The returned compositor renders via `render_offscreen_with(&ctx.device, &ctx.queue, w, h, ...)`.
pub fn build_compositor(width: u32, height: u32) -> Option<(Compositor, Offscreen)> {
    let ctx = vector_render::RenderContext::new_offscreen(width, height).ok()?;
    let font_stack = FontStack::load_bundled(1.0, 14.0).ok()?;
    let comp = Compositor::new_with(
        &ctx.device,
        &ctx.queue,
        ctx.format,
        ctx.width,
        ctx.height,
        font_stack,
    )
    .ok()?;
    Some((comp, ctx))
}

/// Return (r_index, g_index, b_index) for a 4-byte pixel in the given surface format.
pub fn channel_indices(format: wgpu::TextureFormat) -> (usize, usize, usize) {
    match format {
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => (2, 1, 0),
        _ => (0, 1, 2),
    }
}
