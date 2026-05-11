//! Confirms wgpu can find a Metal adapter on macOS without a surface.
//! Runs on CI macos-14 runners (no display required). Plan 03-01, RENDER-01.

#[test]
fn metal_adapter_available() {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = wgpu::Backends::METAL;
    let instance = wgpu::Instance::new(desc);
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("no Metal adapter found");
    let info = adapter.get_info();
    assert_eq!(
        info.backend,
        wgpu::Backend::Metal,
        "adapter backend must be Metal"
    );
}
