//! wgpu Metal surface + clear-color frame. Phase 3 Plan 03-01 (D-45, RENDER-01).

use std::sync::Arc;

use anyhow::{anyhow, Result};
use wgpu::{
    Adapter, CompositeAlphaMode, CurrentSurfaceTexture, Device, ExperimentalFeatures, Instance,
    InstanceDescriptor, Limits, MemoryHints, PowerPreference, PresentMode, Queue,
    RequestAdapterOptions, Surface, SurfaceConfiguration, TextureUsages, Trace,
};
use winit::window::Window;

/// wgpu Metal surface + device/queue, configured for PresentMode::Fifo (D-45).
pub struct RenderContext {
    _instance: Instance,
    _adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
}

impl RenderContext {
    pub fn new(window: &Arc<Window>) -> Result<Self> {
        let mut desc = InstanceDescriptor::new_without_display_handle();
        desc.backends = wgpu::Backends::METAL;
        let instance = Instance::new(desc);
        // Arc<Window> takes the surface to 'static — wgpu 29 owns the handle.
        let surface = instance.create_surface(window.clone())?;
        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .map_err(|e| anyhow!("no wgpu adapter: {e}"))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: Limits::default(),
                label: Some("vector-render-device"),
                memory_hints: MemoryHints::Performance,
                experimental_features: ExperimentalFeatures::disabled(),
                trace: Trace::Off,
            }))?;
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats[0];
        let size = window.inner_size();
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        Ok(Self {
            _instance: instance,
            _adapter: adapter,
            device,
            queue,
            surface,
            config,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    /// Acquire-clear-present. Suboptimal/Outdated/Lost are recoverable and logged; we skip the
    /// frame and let the next RedrawRequested retry. Validation surfaces as anyhow::Error.
    pub fn render_clear(&self, color: &[f64; 4]) -> Result<()> {
        let frame = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(t) | CurrentSurfaceTexture::Suboptimal(t) => t,
            CurrentSurfaceTexture::Timeout
            | CurrentSurfaceTexture::Occluded
            | CurrentSurfaceTexture::Outdated
            | CurrentSurfaceTexture::Lost => {
                tracing::debug!("surface frame unavailable; skipping");
                return Ok(());
            }
            CurrentSurfaceTexture::Validation => {
                return Err(anyhow!("surface validation error"));
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("clear-encoder"),
            });
        {
            let _rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: color[0],
                            g: color[1],
                            b: color[2],
                            a: color[3],
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        self.queue.submit(Some(enc.finish()));
        frame.present();
        Ok(())
    }
}
