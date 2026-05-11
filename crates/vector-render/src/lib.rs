//! wgpu pipeline + glyph atlas + damage tracking. Phase 3.

mod atlas;
mod cell_pipeline;
mod compositor;
mod cursor_pipeline;
mod pipeline;

pub use atlas::{Atlas, AtlasSlot, GlyphKey};
pub use cell_pipeline::CellInstance;
pub use compositor::{Compositor, CompositorError, OffscreenFrame};
pub use cursor_pipeline::{CursorInstance, CursorPipeline};
pub use pipeline::{Offscreen, RenderContext};
