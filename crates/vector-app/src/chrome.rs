//! Plan 05-16: Chrome pipelines owned by AppWindow (HIGH-2 reviewer fix).
//!
//! These four wgpu pipelines render the window chrome surfaces: tint stripe,
//! search bar background, toast banner, profile picker scrim+modal.
//!
//! They live HERE (not inside RenderHost) so that `app.windows[id].chrome_pipelines`
//! and `app.windows[id].render_host` can be borrowed independently in
//! `App::render_window` without triggering a wgpu double-mutable-borrow on
//! RenderHost's surface/compositor.

use vector_render::{PickerPass, SearchBarPass, TintStripePipeline, ToastPass};

pub struct ChromePipelines {
    pub tint: TintStripePipeline,
    pub search_bar: SearchBarPass,
    pub toast: ToastPass,
    pub picker: PickerPass,
}

impl ChromePipelines {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        Self {
            tint: TintStripePipeline::new(device, format),
            search_bar: SearchBarPass::new(device, format),
            toast: ToastPass::new(device, format),
            picker: PickerPass::new(device, format),
        }
    }
}
