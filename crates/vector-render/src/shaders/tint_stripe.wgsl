// POLISH-07 / D-75 / UI-SPEC §5.1 — title-bar tint stripe.
// One quad, one solid color uniform. NDC verts come from CPU-side ndc_quad().

struct Uniforms { rgba: vec4<f32> };
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
};

@vertex
fn vs_main(@location(0) pos: vec2<f32>) -> VsOut {
    var out: VsOut;
    out.clip = vec4<f32>(pos, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return u.rgba;
}
