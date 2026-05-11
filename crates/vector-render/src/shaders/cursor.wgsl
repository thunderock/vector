// Block-cursor pipeline. Plan 03-03 (RENDER-05). Always-on block cursor; blink → Plan 03-05.

struct CursorUniforms {
    viewport_size_px: vec2<f32>,
    cell_size_px: vec2<f32>,
    cursor_cell: vec2<u32>,
    cursor_color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> u: CursorUniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) frag_color: vec4<f32>,
}

@vertex
fn vs_main(@location(0) vertex_pos: vec2<f32>) -> VertexOutput {
    let cell_origin = vec2<f32>(f32(u.cursor_cell.x), f32(u.cursor_cell.y)) * u.cell_size_px;
    let pos_px = cell_origin + vertex_pos * u.cell_size_px;
    let ndc = vec2<f32>(
        (pos_px.x / u.viewport_size_px.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / u.viewport_size_px.y) * 2.0,
    );
    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc, 0.0, 1.0);
    out.frag_color = u.cursor_color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.frag_color;
}
