//! POLISH-07 / D-75 / UI-SPEC §5.1 — tint stripe quad geometry.
//! Phase 8 / D-17 — Microsoft brand-blue constant for DevTunnel pane tint.

#![allow(clippy::float_cmp)]

use vector_render::tint_stripe::{TintStripePipeline, GITHUB_PURPLE, MICROSOFT_BLUE};

#[test]
fn geometry() {
    let quad = TintStripePipeline::quad_for(1200);
    // 6 vertices forming two triangles covering [0..1200] × [0..28].
    assert_eq!(quad.len(), 6);
    let xs: Vec<f32> = quad.iter().map(|v| v[0]).collect();
    let ys: Vec<f32> = quad.iter().map(|v| v[1]).collect();
    assert!(
        xs.iter().all(|&x| (0.0..=1200.0).contains(&x)),
        "x in [0, content_width]"
    );
    assert!(
        ys.iter().all(|&y| (0.0..=28.0).contains(&y)),
        "y in [0, 28] per UI-SPEC §5.1"
    );
    assert!(xs.contains(&1200.0));
    assert!(xs.contains(&0.0));
    assert!(ys.contains(&28.0));
}

#[test]
fn microsoft_blue_is_0078d4() {
    // #0078d4 → R=0x00=0.0, G=0x78=120/255=0.4706, B=0xd4=212/255=0.8314, A=1.0
    let expected: [f32; 4] = [0.0, 0.471, 0.831, 1.0];
    for (i, (a, b)) in MICROSOFT_BLUE.iter().zip(expected.iter()).enumerate() {
        assert!(
            (a - b).abs() < 0.01,
            "channel {i}: got {a}, expected ~{b} (#0078d4)"
        );
    }
}

#[test]
fn github_purple_is_7a3aaf() {
    // Phase 6 legacy — Codespaces dormant in v1 but constant stays exported.
    let expected: [f32; 4] = [0.478, 0.227, 0.686, 1.0];
    for (i, (a, b)) in GITHUB_PURPLE.iter().zip(expected.iter()).enumerate() {
        assert!(
            (a - b).abs() < 0.01,
            "channel {i}: got {a}, expected ~{b} (#7a3aaf)"
        );
    }
}
