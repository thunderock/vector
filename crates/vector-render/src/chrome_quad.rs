//! M1-v2 (Plan 05-10 Task 2) — shared chrome quad pipeline. One wgpu RenderPipeline +
//! uniform-driven screen-px rect + rgba color. Reused by SearchBarPass / ToastPass /
//! PickerPass — each pass becomes ~30 LOC of glue around a `ChromeQuadPipeline` field.
//!
//! Pattern mirrors `tint_stripe.rs::TintStripePipeline` (Plan 05-08 B4 fix); divergences:
//! (a) shader source (`chrome_quad.wgsl`), (b) larger uniform with rect + surface size,
//! (c) vertex shader synthesizes verts from `vertex_index` (no vertex buffer).

#![allow(
    clippy::default_trait_access,
    clippy::similar_names,
    clippy::pub_underscore_fields
)]

use bytemuck::{Pod, Zeroable};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendState, Buffer, BufferBindingType, BufferUsages,
    ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState,
    PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, TextureFormat,
    VertexState,
};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ChromeQuadUniform {
    pub rect_px: [f32; 4],
    pub color_rgba: [f32; 4],
    pub surface_size: [f32; 2],
    pub _pad: [f32; 2],
}

pub struct ChromeQuadPipeline {
    pipeline: RenderPipeline,
    ubuf: Buffer,
    bind_group: BindGroup,
}

impl ChromeQuadPipeline {
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("chrome_quad.wgsl"),
            source: ShaderSource::Wgsl(include_str!("shaders/chrome_quad.wgsl").into()),
        });
        let bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("chrome_quad.bgl"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("chrome_quad.layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("chrome_quad.pipeline"),
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
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
        let ubuf = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("chrome_quad.ubuf"),
            contents: bytemuck::bytes_of(&ChromeQuadUniform {
                rect_px: [0.0; 4],
                color_rgba: [0.0; 4],
                surface_size: [1.0, 1.0],
                _pad: [0.0; 2],
            }),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("chrome_quad.bg"),
            layout: &bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: ubuf.as_entire_binding(),
            }],
        });
        Self {
            pipeline,
            ubuf,
            bind_group,
        }
    }

    pub fn update_quad(
        &self,
        queue: &Queue,
        rect_px: [f32; 4],
        color_rgba: [f32; 4],
        surface_size: [f32; 2],
    ) {
        let u = ChromeQuadUniform {
            rect_px,
            color_rgba,
            surface_size,
            _pad: [0.0; 2],
        };
        queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));
    }

    pub fn draw<'a>(&'a self, rpass: &mut RenderPass<'a>) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.draw(0..6, 0..1);
    }
}
