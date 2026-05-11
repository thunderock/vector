//! wgpu pipeline + glyph atlas + damage tracking. Phase 3.

mod atlas;
mod pipeline;

pub use atlas::{Atlas, AtlasSlot, GlyphKey};
pub use pipeline::RenderContext;
