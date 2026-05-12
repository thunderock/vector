// Shared chrome render pass shader (M1-v2 refactor). Used by SearchBarPass /
// ToastPass / PickerPass via the shared ChromeQuadPipeline helper.
//
// Uniforms layout (std140, 16-byte aligned):
//   rect_px      — [x, y, w, h] in screen pixels (origin = top-left of surface)
//   color_rgba   — fragment color (premultiplied or straight per ALPHA_BLENDING)
//   surface_size — [width_px, height_px] of the swapchain texture
//   _pad         — 8-byte tail to round struct to 48 bytes (multiple of 16)

struct ChromeQuadUniform {
    rect_px:      vec4<f32>,
    color_rgba:   vec4<f32>,
    surface_size: vec2<f32>,
    _pad:         vec2<f32>,
};
@group(0) @binding(0) var<uniform> u: ChromeQuadUniform;

struct VsOut { @builtin(position) clip: vec4<f32> };

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    // 6-vertex triangle list covering the rect.
    // Vertex order: (0,0) (1,0) (0,1) (1,0) (1,1) (0,1).
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0), vec2<f32>(0.0, 1.0),
    );
    let c = corners[vi];
    let px = vec2<f32>(
        u.rect_px.x + c.x * u.rect_px.z,
        u.rect_px.y + c.y * u.rect_px.w,
    );
    // Px (top-left origin) → NDC. NDC y is +1 at top, -1 at bottom.
    let ndc_x = (px.x / u.surface_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (px.y / u.surface_size.y) * 2.0;
    var out: VsOut;
    out.clip = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return u.color_rgba;
}
