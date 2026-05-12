//! M2 (Plan 05-10 Task 2) — ProfilePicker wgpu render pass (UI-SPEC §5.3).
//! Two draws per frame: scrim (full content rect, dim) then modal (centered rect).

#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::too_many_arguments
)]

use crate::chrome_quad::ChromeQuadPipeline;
use wgpu::{Device, Queue, RenderPass, TextureFormat};

pub const PICKER_ROW_HEIGHT_PX: u32 = 28;
pub const PICKER_INPUT_ROW_HEIGHT_PX: u32 = 32;
pub const PICKER_MAX_VISIBLE_ROWS: u32 = 8;
pub const PICKER_MIN_WIDTH_PX: u32 = 280;
pub const PICKER_MAX_WIDTH_PX: u32 = 480;

pub struct PickerLayout {
    pub width_px: u32,
    pub height_px: u32,
    pub x: f32,
    pub y: f32,
}

/// UI-SPEC §5.3: width = clamp(longest_label_px + 48, 280, 480); 25% from top of content.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn picker_layout(
    longest_label_px: u32,
    visible_rows: u32,
    content_width_px: u32,
    content_height_px: u32,
) -> PickerLayout {
    let width_px = (longest_label_px + 48).clamp(PICKER_MIN_WIDTH_PX, PICKER_MAX_WIDTH_PX);
    let row_count = visible_rows.min(PICKER_MAX_VISIBLE_ROWS);
    let height_px = PICKER_INPUT_ROW_HEIGHT_PX + row_count * PICKER_ROW_HEIGHT_PX;
    let x = (content_width_px.saturating_sub(width_px) / 2) as f32;
    let y = (content_height_px / 4) as f32;
    PickerLayout {
        width_px,
        height_px,
        x,
        y,
    }
}

pub struct PickerPass {
    chrome: ChromeQuadPipeline,
}

impl PickerPass {
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        Self {
            chrome: ChromeQuadPipeline::new(device, surface_format),
        }
    }

    /// Update + draw the scrim quad (full content rect, low-alpha black).
    pub fn draw_scrim<'a>(
        &'a self,
        queue: &Queue,
        content_w_px: f32,
        content_h_px: f32,
        surface_w_px: f32,
        surface_h_px: f32,
        rpass: &mut RenderPass<'a>,
    ) {
        self.chrome.update_quad(
            queue,
            [0.0, 0.0, content_w_px, content_h_px],
            [0.0, 0.0, 0.0, 0.45],
            [surface_w_px, surface_h_px],
        );
        self.chrome.draw(rpass);
    }

    /// Update + draw the modal quad (centered rect).
    pub fn draw_modal<'a>(
        &'a self,
        queue: &Queue,
        layout: &PickerLayout,
        bg_rgba: [f32; 4],
        surface_w_px: f32,
        surface_h_px: f32,
        rpass: &mut RenderPass<'a>,
    ) {
        self.chrome.update_quad(
            queue,
            [
                layout.x,
                layout.y,
                layout.width_px as f32,
                layout.height_px as f32,
            ],
            bg_rgba,
            [surface_w_px, surface_h_px],
        );
        self.chrome.draw(rpass);
    }
}
