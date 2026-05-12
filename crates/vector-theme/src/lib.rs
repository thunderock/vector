//! vector-theme — Phase 5 palette, builtins, .itermcolors, appearance resolver (POLISH-03).

pub mod appearance;
pub mod builtins;
pub mod error;
pub mod itermcolors;
pub mod palette;

pub use appearance::resolve_palette;
pub use builtins::{vector_dark, vector_light};
pub use error::ThemeError;
pub use itermcolors::parse_itermcolors;
pub use palette::{ChromePalette, Palette, Rgb, Rgba};
