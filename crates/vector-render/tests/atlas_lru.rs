//! wgpu Metal integration: tiny atlas + many glyphs forces LRU eviction.

use vector_fonts::FontStack;
use vector_render::{Atlas, GlyphKey};

#[test]
fn lru_evicts_oldest_glyph_when_atlas_fills() {
    let mut idesc = wgpu::InstanceDescriptor::new_without_display_handle();
    idesc.backends = wgpu::Backends::METAL;
    let instance = wgpu::Instance::new(idesc);
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("Metal adapter");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        label: Some("atlas-lru-test-device"),
        memory_hints: wgpu::MemoryHints::Performance,
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        trace: wgpu::Trace::Off,
    }))
    .expect("device");

    // 64×64 atlas guarantees overflow: ASCII glyphs at 14pt are ~9×17 px;
    // even tight packing tops out at ~24 glyphs before forcing eviction.
    let mut atlas = Atlas::new_with_dims(&device, 64, 64);
    let stack = FontStack::load_bundled(1.0, 14.0).expect("font stack");

    let chars: Vec<char> = ('!'..='~').collect();
    let mut keys = Vec::new();
    for c in &chars {
        let glyph = stack.rasterize(*c).expect("rasterize");
        let key = GlyphKey {
            character: *c,
            dpr_bucket: 1,
        };
        atlas.slot_for(&queue, key, &glyph);
        keys.push(key);
    }
    assert!(
        !atlas.contains(keys[0]),
        "atlas LRU did not evict oldest glyph ('!')"
    );
    assert!(
        atlas.contains(keys[chars.len() - 1]),
        "most-recent glyph ('~') must still be resident"
    );
}
