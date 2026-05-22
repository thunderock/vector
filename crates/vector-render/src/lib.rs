//! wgpu pipeline + glyph atlas + damage tracking. Phase 3.

mod atlas;
mod cell_pipeline;
pub mod chrome_quad;
mod compositor;
mod cursor_pipeline;
pub mod picker_pass;
mod pipeline;
pub mod reconnect_pass;
pub mod search_bar_pass;
pub mod tint_stripe;
pub mod toast_pass;

pub use atlas::{Atlas, AtlasSlot, GlyphKey};
pub use cell_pipeline::CellInstance;
pub use chrome_quad::{ChromeQuadPipeline, ChromeQuadUniform};
pub use compositor::{Compositor, CompositorError, OffscreenFrame};
pub use cursor_pipeline::{CursorInstance, CursorPipeline};
pub use picker_pass::{picker_layout, PickerLayout, PickerPass, PICKER_ROW_HEIGHT_PX};
pub use pipeline::{Offscreen, RenderContext};
pub use reconnect_pass::{
    alpha_at as reconnect_alpha_at, format_reconnect_text, reconnect_layout, ReconnectLayout,
    ReconnectPass, RECONNECT_BAR_HEIGHT_PX, RECONNECT_DEBOUNCE_MS, RECONNECT_FADE_IN_MS,
    RECONNECT_FADE_OUT_MS,
};
pub use search_bar_pass::{
    search_bar_layout, SearchBarLayout, SearchBarPass, SEARCH_BAR_HEIGHT_PX,
};
pub use tint_stripe::TintStripePipeline;
pub use toast_pass::{
    alpha_at, toast_layout, ToastLayout, ToastModeKind, ToastPass, TOAST_ACTION_HEIGHT_PX,
    TOAST_FADE_IN_MS, TOAST_FADE_OUT_MS, TOAST_INFO_HEIGHT_PX,
};
