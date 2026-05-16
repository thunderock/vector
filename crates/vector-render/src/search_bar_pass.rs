//! M1 (Plan 05-10 Task 2) — SearchBar wgpu render pass (UI-SPEC §5.2).
//! 32 px height background bar. Child glyphs (query text, counter, "aA", arrows,
//! close "×") composite via the existing CellPipeline glyph atlas (Plan 03-02).

#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::too_many_arguments
)]

use crate::chrome_quad::ChromeQuadPipeline;
use wgpu::{Device, Queue, RenderPass, TextureFormat};

pub const SEARCH_BAR_HEIGHT_PX: u32 = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub struct SearchBarLayout {
    pub height_px: u32,
    pub bg_rgba: [f32; 4],
    pub query_field: Rect,
    pub smart_case_indicator: Rect,
    pub prev_arrow: Rect,
    pub next_arrow: Rect,
    pub counter: Rect,
    pub close_btn: Rect,
}

/// UI-SPEC §5.2: 32 px tall; right-aligned chrome
/// `[smart_case 24][prev 24][next 24][counter 48][close 24]`; query field flexes;
/// 4 px spacing per UI-SPEC §2.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn search_bar_layout(content_width: u32, no_match: bool) -> SearchBarLayout {
    let h = SEARCH_BAR_HEIGHT_PX as f32;
    let right_widths = [24.0_f32, 24.0, 24.0, 48.0, 24.0];
    let spacing = 4.0_f32;
    let reserved: f32 = right_widths.iter().sum::<f32>() + spacing * (right_widths.len() as f32);
    let query_w = (content_width as f32 - reserved - spacing).max(0.0);

    let mut x = spacing;
    let query_field = Rect {
        x,
        y: 0.0,
        w: query_w,
        h,
    };
    x += query_w + spacing;
    let smart_case_indicator = Rect {
        x,
        y: 0.0,
        w: 24.0,
        h,
    };
    x += 24.0 + spacing;
    let prev_arrow = Rect {
        x,
        y: 0.0,
        w: 24.0,
        h,
    };
    x += 24.0 + spacing;
    let next_arrow = Rect {
        x,
        y: 0.0,
        w: 24.0,
        h,
    };
    x += 24.0 + spacing;
    let counter = Rect {
        x,
        y: 0.0,
        w: 48.0,
        h,
    };
    x += 48.0 + spacing;
    let close_btn = Rect {
        x,
        y: 0.0,
        w: 24.0,
        h,
    };

    let baseline = [0.11_f32, 0.11, 0.12, 0.92];
    let bg_rgba = if no_match {
        let warning = [0.78_f32, 0.20, 0.18, 0.20];
        [
            baseline[0] * 0.8 + warning[0] * 0.2,
            baseline[1] * 0.8 + warning[1] * 0.2,
            baseline[2] * 0.8 + warning[2] * 0.2,
            baseline[3],
        ]
    } else {
        baseline
    };

    SearchBarLayout {
        height_px: SEARCH_BAR_HEIGHT_PX,
        bg_rgba,
        query_field,
        smart_case_indicator,
        prev_arrow,
        next_arrow,
        counter,
        close_btn,
    }
}

pub struct SearchBarPass {
    chrome: ChromeQuadPipeline,
}

impl SearchBarPass {
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        Self {
            chrome: ChromeQuadPipeline::new(device, surface_format),
        }
    }

    /// Position the bar; `pane_bottom_y_px` is the surface-px Y of the bar's TOP edge.
    pub fn update_for_pane(
        &self,
        queue: &Queue,
        pane_bottom_y_px: f32,
        content_width_px: f32,
        surface_w_px: f32,
        surface_h_px: f32,
        bg_rgba: [f32; 4],
    ) {
        let rect = [
            0.0,
            pane_bottom_y_px,
            content_width_px,
            SEARCH_BAR_HEIGHT_PX as f32,
        ];
        self.chrome
            .update_quad(queue, rect, bg_rgba, [surface_w_px, surface_h_px]);
    }

    pub fn draw<'a>(&'a self, rpass: &mut RenderPass<'a>) {
        self.chrome.draw(rpass);
    }
}
