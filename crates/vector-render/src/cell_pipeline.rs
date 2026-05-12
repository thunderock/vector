//! Cell pipeline: one quad per cell, instanced. Plan 03-03 (RENDER-01/04).

#![allow(clippy::too_many_lines, clippy::default_trait_access, dead_code)]

use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendState, Buffer, BufferDescriptor,
    BufferUsages, ColorTargetState, ColorWrites, Device, FragmentState, MipmapFilterMode,
    MultisampleState, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, TextureFormat, TextureSampleType,
    TextureView, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexState, VertexStepMode,
};

/// One quad per terminal cell. Repr-C, Pod for `queue.write_buffer`. 72 bytes per instance.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
#[allow(clippy::pub_underscore_fields)]
pub struct CellInstance {
    /// (col, row) — viewport-relative.
    pub cell_pos: [u32; 2],
    pub fg: [f32; 4],
    pub bg: [f32; 4],
    /// (u0, v0, u1, v1) inside the bound atlas.
    pub uv: [f32; 4],
    /// 0 = Mono, 1 = Color, 2 = Empty/bg-only.
    pub atlas_kind: u32,
    /// 0 or 1; Plan 03-04 populates from the selection state machine.
    pub selected: u32,
    /// Bit 0: inverse. Bit 1: bold (reserved). Others reserved.
    pub flags: u32,
    pub _pad: u32,
}

/// CPU-side mirror of cell.wgsl's `Uniforms`. Plan 04-04 added per-pane viewport
/// offset/size + border color/width (D-66). Layout matches WGSL std140-ish: vec4
/// fields are 16-byte aligned; explicit padding keeps total a multiple of 16.
///
/// Byte offsets (must match cell.wgsl):
///   0  window_size_px      vec2 (8 B)
///   8  cell_size_px        vec2 (8 B)
///   16 selection_tint      vec4 (16 B)
///   32 border_color        vec4 (16 B)
///   48 viewport_offset_px  vec2 (8 B)
///   56 viewport_size_px    vec2 (8 B)
///   64 border_width_px     f32  (4 B)
///   68 _pad0               f32  (4 B)
///   72 _pad1               vec2 (8 B) → total 80, aligned to 16
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
#[allow(clippy::pub_underscore_fields)]
pub struct Uniforms {
    pub window_size_px: [f32; 2],
    pub cell_size_px: [f32; 2],
    pub selection_tint: [f32; 4],
    pub border_color: [f32; 4],
    pub viewport_offset_px: [f32; 2],
    pub viewport_size_px: [f32; 2],
    pub border_width_px: f32,
    pub _pad0: f32,
    pub _pad1: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct QuadVertex {
    pos: [f32; 2],
}

const QUAD_VERTICES: [QuadVertex; 4] = [
    QuadVertex { pos: [0.0, 0.0] },
    QuadVertex { pos: [1.0, 0.0] },
    QuadVertex { pos: [0.0, 1.0] },
    QuadVertex { pos: [1.0, 1.0] },
];
const QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 1, 3];

pub struct CellPipeline {
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    sampler: Sampler,
    instance_buf: Buffer,
    instance_capacity: usize,
    vertex_buf: Buffer,
    index_buf: Buffer,
    uniform_buf: Buffer,
    pub(crate) bind_group_layout: BindGroupLayout,
}

impl CellPipeline {
    pub fn new(
        device: &Device,
        surface_format: TextureFormat,
        mono_view: &TextureView,
        color_view: &TextureView,
        initial_capacity: usize,
    ) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("cell-shader"),
            source: ShaderSource::Wgsl(include_str!("shaders/cell.wgsl").into()),
        });
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("cell-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let uniform_buf = device.create_buffer(&BufferDescriptor {
            label: Some("cell-uniforms"),
            size: size_of::<Uniforms>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("cell-bgl"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("cell-bg"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(mono_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(color_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: uniform_buf.as_entire_binding(),
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("cell-pl"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let vertex_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("cell-quad-vbuf"),
            contents: bytemuck::cast_slice(&QUAD_VERTICES),
            usage: BufferUsages::VERTEX,
        });
        let index_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("cell-quad-ibuf"),
            contents: bytemuck::cast_slice(&QUAD_INDICES),
            usage: BufferUsages::INDEX,
        });
        let instance_capacity = initial_capacity.max(1);
        let instance_buf = device.create_buffer(&BufferDescriptor {
            label: Some("cell-instance-buf"),
            size: (instance_capacity * size_of::<CellInstance>()) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("cell-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    VertexBufferLayout {
                        array_stride: size_of::<QuadVertex>() as u64,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &[VertexAttribute {
                            shader_location: 0,
                            offset: 0,
                            format: VertexFormat::Float32x2,
                        }],
                    },
                    instance_buffer_layout(),
                ],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pipeline,
            bind_group,
            sampler,
            instance_buf,
            instance_capacity,
            vertex_buf,
            index_buf,
            uniform_buf,
            bind_group_layout,
        }
    }

    /// Rebind to a new atlas view pair (Plan 03-05 DPR change clears + reloads).
    pub fn rebind_atlas(
        &mut self,
        device: &Device,
        mono_view: &TextureView,
        color_view: &TextureView,
    ) {
        self.bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("cell-bg"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(mono_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(color_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&self.sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: self.uniform_buf.as_entire_binding(),
                },
            ],
        });
    }

    pub fn ensure_capacity(&mut self, device: &Device, needed: usize) {
        if needed <= self.instance_capacity {
            return;
        }
        let new_cap = needed.next_power_of_two().max(self.instance_capacity * 2);
        self.instance_buf = device.create_buffer(&BufferDescriptor {
            label: Some("cell-instance-buf"),
            size: (new_cap * size_of::<CellInstance>()) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.instance_capacity = new_cap;
    }

    pub fn upload_instances(&self, queue: &Queue, instances: &[CellInstance], offset_cells: usize) {
        if instances.is_empty() {
            return;
        }
        let byte_offset = (offset_cells * size_of::<CellInstance>()) as u64;
        queue.write_buffer(
            &self.instance_buf,
            byte_offset,
            bytemuck::cast_slice(instances),
        );
    }

    pub fn update_uniforms(&self, queue: &Queue, uniforms: &Uniforms) {
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(uniforms));
    }

    pub fn draw<'a>(&'a self, rpass: &mut RenderPass<'a>, instance_count: u32) {
        if instance_count == 0 {
            return;
        }
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        rpass.set_vertex_buffer(1, self.instance_buf.slice(..));
        rpass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
        rpass.draw_indexed(0..6, 0, 0..instance_count);
    }
}

fn instance_buffer_layout() -> VertexBufferLayout<'static> {
    // 8 attributes: cell_pos(u32x2), fg(f32x4), bg(f32x4), uv(f32x4),
    // atlas_kind(u32), selected(u32), flags(u32), _pad(u32 — unused in shader)
    const ATTRS: &[VertexAttribute] = &[
        VertexAttribute {
            shader_location: 1,
            offset: 0,
            format: VertexFormat::Uint32x2,
        },
        VertexAttribute {
            shader_location: 2,
            offset: 8,
            format: VertexFormat::Float32x4,
        },
        VertexAttribute {
            shader_location: 3,
            offset: 24,
            format: VertexFormat::Float32x4,
        },
        VertexAttribute {
            shader_location: 4,
            offset: 40,
            format: VertexFormat::Float32x4,
        },
        VertexAttribute {
            shader_location: 5,
            offset: 56,
            format: VertexFormat::Uint32,
        },
        VertexAttribute {
            shader_location: 6,
            offset: 60,
            format: VertexFormat::Uint32,
        },
        VertexAttribute {
            shader_location: 7,
            offset: 64,
            format: VertexFormat::Uint32,
        },
    ];
    VertexBufferLayout {
        array_stride: size_of::<CellInstance>() as u64,
        step_mode: VertexStepMode::Instance,
        attributes: ATTRS,
    }
}
