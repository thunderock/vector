// Block-cursor pipeline. Plan 03-03 (RENDER-05). Plan 03-05 adds blink.
// Plan 04-04: per-pane viewport offset + cursor_focused (filled vs hollow outline).

struct CursorUniforms {
    window_size_px: vec2<f32>,
    cell_size_px: vec2<f32>,
    cursor_cell: vec2<u32>,
    viewport_offset_px: vec2<f32>,
    cursor_color: vec4<f32>,
    cursor_focused: u32,
    _pad0: u32,
    _pad1: vec2<u32>,
}

@group(0) @binding(0) var<uniform> u: CursorUniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) frag_color: vec4<f32>,
    @location(1) frag_local: vec2<f32>,
    @location(2) @interpolate(flat) frag_focused: u32,
}

@vertex
fn vs_main(@location(0) vertex_pos: vec2<f32>) -> VertexOutput {
    let cell_origin = vec2<f32>(f32(u.cursor_cell.x), f32(u.cursor_cell.y)) * u.cell_size_px;
    let local_px = cell_origin + vertex_pos * u.cell_size_px;
    let pos_px = u.viewport_offset_px + local_px;
    let ndc = vec2<f32>(
        (pos_px.x / u.window_size_px.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / u.window_size_px.y) * 2.0,
    );
    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc, 0.0, 1.0);
    out.frag_color = u.cursor_color;
    out.frag_local = vertex_pos * u.cell_size_px;
    out.frag_focused = u.cursor_focused;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (in.frag_focused != 0u) {
        return in.frag_color;
    }
    // Inactive pane (Plan 04-04): 1-px stroke outline around the cell rect.
    let stroke = 1.0;
    let dl = in.frag_local.x;
    let dr = u.cell_size_px.x - in.frag_local.x;
    let dt = in.frag_local.y;
    let db = u.cell_size_px.y - in.frag_local.y;
    let dmin = min(min(dl, dr), min(dt, db));
    if (dmin < stroke) {
        return in.frag_color;
    }
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}
