//! Block-cursor pipeline. Plan 03-03 Task 2 (RENDER-05). Plan 03-05 adds blink.

#![allow(clippy::too_many_lines, clippy::default_trait_access)]

use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BlendState, Buffer, BufferDescriptor, BufferUsages,
    ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState,
    PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, TextureFormat,
    VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

/// Mirror of cursor.wgsl `CursorUniforms`. Plan 04-04 adds per-pane viewport offset
/// + cursor_focused (filled vs hollow outline). Layout:
///   0  window_size_px      vec2 (8)
///   8  cell_size_px        vec2 (8)
///   16 cursor_cell         vec2<u32> (8)
///   24 viewport_offset_px  vec2<f32> (8)
///   32 cursor_color        vec4 (16) — must be 16-aligned
///   48 cursor_focused      u32 (4)
///   52 _pad0               u32 (4)
///   56 _pad1               vec2<u32> (8) → total 64
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
#[allow(clippy::pub_underscore_fields)]
struct CursorUniforms {
    window_size_px: [f32; 2],
    cell_size_px: [f32; 2],
    cursor_cell: [u32; 2],
    viewport_offset_px: [f32; 2],
    cursor_color: [f32; 4],
    cursor_focused: u32,
    _pad0: u32,
    _pad1: [u32; 2],
}

/// Placeholder for future instanced cursor variants (bar, underline). Block cursor only in v1.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CursorInstance {
    pub cell_pos: [u32; 2],
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

pub struct CursorPipeline {
    pipeline: RenderPipeline,
    bind_group: BindGroup,
    vertex_buf: Buffer,
    index_buf: Buffer,
    uniform_buf: Buffer,
    _bgl: BindGroupLayout,
}

impl CursorPipeline {
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("cursor-shader"),
            source: ShaderSource::Wgsl(include_str!("shaders/cursor.wgsl").into()),
        });
        let uniform_buf = device.create_buffer(&BufferDescriptor {
            label: Some("cursor-uniforms"),
            size: size_of::<CursorUniforms>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("cursor-bgl"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("cursor-bg"),
            layout: &bgl,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("cursor-pl"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let vertex_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("cursor-vbuf"),
            contents: bytemuck::cast_slice(&QUAD_VERTICES),
            usage: BufferUsages::VERTEX,
        });
        let index_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("cursor-ibuf"),
            contents: bytemuck::cast_slice(&QUAD_INDICES),
            usage: BufferUsages::INDEX,
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("cursor-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[VertexBufferLayout {
                    array_stride: size_of::<QuadVertex>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[VertexAttribute {
                        shader_location: 0,
                        offset: 0,
                        format: VertexFormat::Float32x2,
                    }],
                }],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    // Alpha blend so the hollow-cursor stroke composites over the cell
                    // pass without zeroing transparent interior pixels.
                    blend: Some(BlendState::ALPHA_BLENDING),
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
            vertex_buf,
            index_buf,
            uniform_buf,
            _bgl: bgl,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &self,
        queue: &Queue,
        cursor_cell: [u32; 2],
        cell_size_px: [f32; 2],
        window_size_px: [f32; 2],
        viewport_offset_px: [f32; 2],
        cursor_color: [f32; 4],
        cursor_focused: bool,
    ) {
        let u = CursorUniforms {
            window_size_px,
            cell_size_px,
            cursor_cell,
            viewport_offset_px,
            cursor_color,
            cursor_focused: u32::from(cursor_focused),
            _pad0: 0,
            _pad1: [0, 0],
        };
        queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&u));
    }

    pub fn draw<'a>(&'a self, rpass: &mut RenderPass<'a>) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        rpass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
        rpass.draw_indexed(0..6, 0, 0..1);
    }
}
