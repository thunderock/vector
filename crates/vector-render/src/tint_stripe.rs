//! POLISH-07 / D-75 / UI-SPEC §5.1 — title-bar tint stripe pipeline.
//! 1 pipeline, 1 quad (6 verts, 2 tris), 1 vec4 color uniform.

#![allow(
    clippy::default_trait_access,
    clippy::cast_precision_loss,
    clippy::similar_names
)]

use bytemuck::{Pod, Zeroable};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendState, Buffer, BufferBindingType, BufferUsages,
    ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState,
    PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, TextureFormat,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Tint {
    rgba: [f32; 4],
}

pub struct TintStripePipeline {
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    quad_vertices: Buffer,
    color_uniform: Buffer,
    current_color: Option<[f32; 4]>,
}

impl TintStripePipeline {
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("tint_stripe.wgsl"),
            source: ShaderSource::Wgsl(include_str!("shaders/tint_stripe.wgsl").into()),
        });
        let bg_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("tint_stripe.bgl"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("tint_stripe.layout"),
            bind_group_layouts: &[Some(&bg_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("tint_stripe.pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[VertexBufferLayout {
                    array_stride: 8,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[VertexAttribute {
                        format: VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    }],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        // Safe default; caller updates via update_quad() once surface size is known.
        let initial_ndc = ndc_quad(800, 600);
        let quad_vertices = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("tint_stripe.quad"),
            contents: bytemuck::cast_slice(&initial_ndc),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        let color_uniform = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("tint_stripe.color"),
            contents: bytemuck::bytes_of(&Tint { rgba: [0.0; 4] }),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("tint_stripe.bg"),
            layout: &bg_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: color_uniform.as_entire_binding(),
            }],
        });
        Self {
            pipeline,
            bind_group,
            quad_vertices,
            color_uniform,
            current_color: None,
        }
    }

    pub fn set_color(&mut self, queue: &Queue, rgba: Option<[f32; 4]>) {
        self.current_color = rgba;
        let payload = Tint {
            rgba: rgba.unwrap_or([0.0; 4]),
        };
        queue.write_buffer(&self.color_uniform, 0, bytemuck::bytes_of(&payload));
    }

    /// Update the quad to span [0..surface_w] × [0..28] of the current surface.
    pub fn update_quad(&self, queue: &Queue, surface_w_px: u32, surface_h_px: u32) {
        let verts = ndc_quad(surface_w_px, surface_h_px);
        queue.write_buffer(&self.quad_vertices, 0, bytemuck::cast_slice(&verts));
    }

    pub fn draw<'a>(&'a self, pass: &mut RenderPass<'a>) {
        if self.current_color.is_none() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.quad_vertices.slice(..));
        pass.draw(0..6, 0..1);
    }

    /// Screen-px geometry helper (used by geometry tests + render-pass debugging).
    /// Returns 6 verts spanning `[0..content_width_px] × [0..28]`.
    #[must_use]
    pub fn quad_for(content_width_px: u32) -> [[f32; 2]; 6] {
        let w = content_width_px as f32;
        let h = 28.0;
        [
            [0.0, 0.0], [w, 0.0], [0.0, h],
            [w, 0.0],  [w, h],   [0.0, h],
        ]
    }
}

/// Convert screen-px rect `[0..surface_w, 0..28]` (top of surface) into NDC verts.
/// NDC: x in [-1, 1] maps to [0, surface_w]; y in [1, -1] maps to [0, surface_h].
/// Surface width is implicit (full surface) so we span x ∈ [-1, 1].
fn ndc_quad(_surface_w_px: u32, surface_h_px: u32) -> [[f32; 2]; 6] {
    let h_ndc = (28.0 / surface_h_px.max(1) as f32) * 2.0;
    let x0 = -1.0;
    let x1 = 1.0;
    let y0 = 1.0;
    let y1 = 1.0 - h_ndc;
    [
        [x0, y0], [x1, y0], [x0, y1],
        [x1, y0], [x1, y1], [x0, y1],
    ]
}
