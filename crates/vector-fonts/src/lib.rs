//! Font discovery + rasterization. Phase 3: crossfont 0.9 + CoreText (D-40, D-50).

mod glyph;
mod loader;
mod width;

pub use glyph::{BitmapKind, RasterizedGlyph};
pub use loader::{CellMetrics, FontStack};
pub use width::cell_width;
