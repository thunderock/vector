//! M2 (Plan 05-10 Task 2) — ToastBanner wgpu render pass (UI-SPEC §5.4).
//! 36 px Info / 56 px Action. Fades 120 ms in / 200 ms out (instant under Reduce Motion).

#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::too_many_arguments
)]

use crate::chrome_quad::ChromeQuadPipeline;
use wgpu::{Device, Queue, RenderPass, TextureFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastModeKind {
    Info,
    Action,
}

pub const TOAST_INFO_HEIGHT_PX: u32 = 36;
pub const TOAST_ACTION_HEIGHT_PX: u32 = 56;
pub const TOAST_FADE_IN_MS: u32 = 120;
pub const TOAST_FADE_OUT_MS: u32 = 200;

pub struct ToastLayout {
    pub height_px: u32,
    pub fade_in_ms: u32,
    pub fade_out_ms: u32,
    pub bg_rgba: [f32; 4],
}

#[must_use]
pub fn toast_layout(mode: ToastModeKind) -> ToastLayout {
    let (height_px, bg_rgba) = match mode {
        ToastModeKind::Info => (TOAST_INFO_HEIGHT_PX, [0.11_f32, 0.11, 0.12, 0.95]),
        ToastModeKind::Action => (TOAST_ACTION_HEIGHT_PX, [0.13_f32, 0.11, 0.15, 0.97]),
    };
    ToastLayout {
        height_px,
        fade_in_ms: TOAST_FADE_IN_MS,
        fade_out_ms: TOAST_FADE_OUT_MS,
        bg_rgba,
    }
}

/// Piecewise alpha curve; Reduce Motion collapses to instant on/off.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn alpha_at(elapsed_ms: u32, total_visible_ms: u32, reduce_motion: bool) -> f32 {
    if reduce_motion {
        return if elapsed_ms < total_visible_ms {
            1.0
        } else {
            0.0
        };
    }
    if elapsed_ms < TOAST_FADE_IN_MS {
        elapsed_ms as f32 / TOAST_FADE_IN_MS as f32
    } else if elapsed_ms < total_visible_ms.saturating_sub(TOAST_FADE_OUT_MS) {
        1.0
    } else if elapsed_ms < total_visible_ms {
        (total_visible_ms - elapsed_ms) as f32 / TOAST_FADE_OUT_MS as f32
    } else {
        0.0
    }
}

pub struct ToastPass {
    chrome: ChromeQuadPipeline,
}

impl ToastPass {
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        Self {
            chrome: ChromeQuadPipeline::new(device, surface_format),
        }
    }

    /// `top_y_px` is the surface-px Y of the toast's TOP edge.
    pub fn update(
        &self,
        queue: &Queue,
        top_y_px: f32,
        content_w_px: f32,
        mode: ToastModeKind,
        alpha: f32,
        surface_w_px: f32,
        surface_h_px: f32,
    ) {
        let l = toast_layout(mode);
        let rgba = [
            l.bg_rgba[0],
            l.bg_rgba[1],
            l.bg_rgba[2],
            l.bg_rgba[3] * alpha,
        ];
        let rect = [0.0, top_y_px, content_w_px, l.height_px as f32];
        self.chrome
            .update_quad(queue, rect, rgba, [surface_w_px, surface_h_px]);
    }

    pub fn draw<'a>(&'a self, rpass: &mut RenderPass<'a>) {
        self.chrome.draw(rpass);
    }
}
