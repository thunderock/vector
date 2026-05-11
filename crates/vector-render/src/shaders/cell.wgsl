// Cell pipeline shader. Plan 03-03: cell-grid composite with per-cell selected bit.

struct Uniforms {
    viewport_size_px: vec2<f32>,
    cell_size_px: vec2<f32>,
    selection_tint: vec4<f32>,
}

@group(0) @binding(0) var mono_atlas: texture_2d<f32>;
@group(0) @binding(1) var color_atlas: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;
@group(0) @binding(3) var<uniform> u: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) frag_uv: vec2<f32>,
    @location(1) frag_fg: vec4<f32>,
    @location(2) frag_bg: vec4<f32>,
    @location(3) @interpolate(flat) frag_atlas_kind: u32,
    @location(4) @interpolate(flat) frag_selected: u32,
}

@vertex
fn vs_main(
    @location(0) vertex_pos: vec2<f32>,
    @location(1) cell_pos: vec2<u32>,
    @location(2) fg: vec4<f32>,
    @location(3) bg: vec4<f32>,
    @location(4) uv_rect: vec4<f32>,
    @location(5) atlas_kind: u32,
    @location(6) selected: u32,
    @location(7) flags: u32,
) -> VertexOutput {
    let cell_px = vec2<f32>(f32(cell_pos.x), f32(cell_pos.y)) * u.cell_size_px;
    let pos_px = cell_px + vertex_pos * u.cell_size_px;
    let ndc = vec2<f32>(
        (pos_px.x / u.viewport_size_px.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / u.viewport_size_px.y) * 2.0,
    );
    var out: VertexOutput;
    out.clip_position = vec4<f32>(ndc, 0.0, 1.0);
    out.frag_uv = mix(uv_rect.xy, uv_rect.zw, vertex_pos);
    // Inverse flag (bit 0): swap fg/bg.
    if ((flags & 1u) != 0u) {
        out.frag_fg = bg;
        out.frag_bg = fg;
    } else {
        out.frag_fg = fg;
        out.frag_bg = bg;
    }
    out.frag_atlas_kind = atlas_kind;
    out.frag_selected = selected;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var out: vec4<f32>;
    if (in.frag_atlas_kind == 0u) {
        // Mono RGB alphamask, multiply by fg.
        let s = textureSample(mono_atlas, samp, in.frag_uv);
        let cov = max(s.r, max(s.g, s.b));
        let glyph_rgb = in.frag_fg.rgb * s.rgb;
        out = vec4<f32>(mix(in.frag_bg.rgb, glyph_rgb, cov), 1.0);
    } else if (in.frag_atlas_kind == 1u) {
        let s = textureSample(color_atlas, samp, in.frag_uv);
        out = vec4<f32>(mix(in.frag_bg.rgb, s.rgb, s.a), 1.0);
    } else {
        out = vec4<f32>(in.frag_bg.rgb, 1.0);
    }
    if (in.frag_selected == 1u) {
        out = vec4<f32>(mix(out.rgb, u.selection_tint.rgb, u.selection_tint.a), 1.0);
    }
    return out;
}
