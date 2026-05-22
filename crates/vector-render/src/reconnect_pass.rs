//! Plan 09-04 — ReconnectPass wgpu pipeline (UI-SPEC §S1).
//!
//! Pane-top inline status bar shown during a transport reconnect. Structurally
//! cloned from `toast_pass.rs` (single mode, no kind branch), but kept as a
//! distinct pipeline because Toast is window-scoped + auto-dismissed and
//! Reconnect is pane-scoped + persistent until success.
//!
//! Text glyphs composite via the caller's CellPipeline (Plan 09-05 wires it).
//! This pass owns only the 24 px bar background.

#![allow(
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::too_many_arguments
)]

use crate::chrome_quad::ChromeQuadPipeline;
use wgpu::{Device, Queue, RenderPass, TextureFormat};

pub const RECONNECT_BAR_HEIGHT_PX: u32 = 24;
pub const RECONNECT_FADE_IN_MS: u32 = 120;
pub const RECONNECT_FADE_OUT_MS: u32 = 200;
pub const RECONNECT_DEBOUNCE_MS: u32 = 250;

pub struct ReconnectLayout {
    pub height_px: u32,
    pub fade_in_ms: u32,
    pub fade_out_ms: u32,
    pub bg_rgba: [f32; 4],
}

/// Default layout — dark-mode `chrome.surface` at α=0.9 (UI-SPEC §Color row 1).
/// Light-mode swap is the caller's responsibility (passed via `bg_rgba` arg to `update`).
#[must_use]
pub fn reconnect_layout() -> ReconnectLayout {
    ReconnectLayout {
        height_px: RECONNECT_BAR_HEIGHT_PX,
        fade_in_ms: RECONNECT_FADE_IN_MS,
        fade_out_ms: RECONNECT_FADE_OUT_MS,
        bg_rgba: [0.110, 0.110, 0.118, 0.90],
    }
}

/// Reconnect-tuned fade curve. Mirrors `toast_pass::alpha_at` shape but uses
/// `RECONNECT_FADE_IN_MS` / `RECONNECT_FADE_OUT_MS`. Reduce Motion collapses to instant.
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
    if elapsed_ms < RECONNECT_FADE_IN_MS {
        elapsed_ms as f32 / RECONNECT_FADE_IN_MS as f32
    } else if elapsed_ms < total_visible_ms.saturating_sub(RECONNECT_FADE_OUT_MS) {
        1.0
    } else if elapsed_ms < total_visible_ms {
        (total_visible_ms - elapsed_ms) as f32 / RECONNECT_FADE_OUT_MS as f32
    } else {
        0.0
    }
}

/// Pane-top status bar background. Single quad; caller composites glyph row over it.
pub struct ReconnectPass {
    chrome: ChromeQuadPipeline,
}

impl ReconnectPass {
    pub fn new(device: &Device, surface_format: TextureFormat) -> Self {
        Self {
            chrome: ChromeQuadPipeline::new(device, surface_format),
        }
    }

    /// `pane_rect_px` = `(x, y, width, height)` of the pane viewport in surface px.
    /// Bar paints at the TOP of the pane rect with height `RECONNECT_BAR_HEIGHT_PX`.
    pub fn update(
        &self,
        queue: &Queue,
        pane_rect_px: (u32, u32, u32, u32),
        surface_size_px: (u32, u32),
        alpha: f32,
        bg_rgba: [f32; 4],
    ) {
        let (x, y, w, _h) = pane_rect_px;
        let rect = [
            x as f32,
            y as f32,
            w as f32,
            RECONNECT_BAR_HEIGHT_PX as f32,
        ];
        let rgba = [bg_rgba[0], bg_rgba[1], bg_rgba[2], bg_rgba[3] * alpha];
        let (sw, sh) = surface_size_px;
        self.chrome
            .update_quad(queue, rect, rgba, [sw as f32, sh as f32]);
    }

    pub fn draw<'a>(&'a self, rpass: &mut RenderPass<'a>) {
        self.chrome.draw(rpass);
    }
}

/// UI-SPEC §Copywriting: status-bar string for a reconnecting pane.
/// `content_cells` is the pane viewport width in cells.
/// Returns `None` when `content_cells < 18` (UI-SPEC: "hide bar entirely").
#[must_use]
pub fn format_reconnect_text(profile: &str, attempt: u32, content_cells: u16) -> Option<String> {
    if content_cells < 18 {
        return None;
    }
    let attempt_text = if attempt >= 10 {
        String::from("9+")
    } else {
        attempt.to_string()
    };
    let suffix = format!(" (attempt {attempt_text})");
    if content_cells < 28 {
        return Some(format!("Reconnecting\u{2026}{suffix}"));
    }
    let profile_render = if content_cells < 40 && profile.chars().count() > 16 {
        let chars: Vec<char> = profile.chars().collect();
        let first: String = chars.iter().take(8).collect();
        let last: String = chars
            .iter()
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{first}\u{2026}{last}")
    } else {
        profile.to_string()
    };
    Some(format!("Reconnecting to {profile_render}\u{2026}{suffix}"))
}

#[cfg(test)]
mod constants_tests {
    use super::*;
    #[test]
    fn constants_match_ui_spec() {
        assert_eq!(RECONNECT_BAR_HEIGHT_PX, 24);
        assert_eq!(RECONNECT_FADE_IN_MS, 120);
        assert_eq!(RECONNECT_FADE_OUT_MS, 200);
        assert_eq!(RECONNECT_DEBOUNCE_MS, 250);
    }
}
