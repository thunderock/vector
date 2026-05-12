//! POLISH-07 / D-75 / UI-SPEC §5.1 — tint stripe quad geometry.

use vector_render::tint_stripe::TintStripePipeline;

#[test]
fn geometry() {
    let quad = TintStripePipeline::quad_for(1200);
    // 6 vertices forming two triangles covering [0..1200] × [0..28].
    assert_eq!(quad.len(), 6);
    let xs: Vec<f32> = quad.iter().map(|v| v[0]).collect();
    let ys: Vec<f32> = quad.iter().map(|v| v[1]).collect();
    assert!(xs.iter().all(|&x| (0.0..=1200.0).contains(&x)), "x in [0, content_width]");
    assert!(ys.iter().all(|&y| (0.0..=28.0).contains(&y)), "y in [0, 28] per UI-SPEC §5.1");
    assert!(xs.iter().any(|&x| x == 1200.0));
    assert!(xs.iter().any(|&x| x == 0.0));
    assert!(ys.iter().any(|&y| y == 28.0));
}
