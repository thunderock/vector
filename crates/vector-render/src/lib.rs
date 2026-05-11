//! wgpu pipeline + glyph atlas + damage tracking. Phase 3.

mod atlas;
mod cell_pipeline;
mod compositor;
mod pipeline;

pub use atlas::{Atlas, AtlasSlot, GlyphKey};
pub use cell_pipeline::CellInstance;
pub use compositor::Compositor;
pub use pipeline::RenderContext;
