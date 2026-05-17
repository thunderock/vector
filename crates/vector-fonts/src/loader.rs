//! FontStack: crossfont 0.9 CoreText rasterizer over bundled JetBrains Mono (D-40, D-41, D-50).

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use crossfont::{
    BitmapBuffer, FontDesc, FontKey, GlyphKey, Metrics, Rasterize, Rasterizer, Size, Slant, Style,
    Weight,
};
use parking_lot::Mutex;

use crate::glyph::{BitmapKind, RasterizedGlyph};

#[derive(Debug, Clone, Copy)]
pub struct CellMetrics {
    pub width_px: u32,
    pub height_px: u32,
    pub baseline: i32,
}

pub struct FontStack {
    rasterizer: Arc<Mutex<Rasterizer>>,
    font_key: FontKey,
    size: Size,
    pub cell_metrics: CellMetrics,
    // POLISH-02 D-69: runtime ligature toggle (Pattern 5). Default true — JetBrains Mono
    // ligatures render via CoreText shaping at glyph-lookup time; the toggle gates a
    // future contiguous-run shaper path (deferred per 05-RESEARCH Pattern 5).
    ligatures_enabled: bool,
}

impl FontStack {
    /// Load the bundled JetBrains Mono Regular face at the given device-pixel ratio + point size.
    /// `dpr` is reserved for future per-DPR rasterizers — crossfont 0.9's CoreText backend takes
    /// no DPR at construction; we pre-multiply into `size_pt` so the CoreText pixel grid lines up.
    pub fn load_bundled(dpr: f32, size_pt: f32) -> Result<Self> {
        if locate_bundled_font().is_none() {
            tracing::warn!(
                "JetBrains Mono not found in bundle; will fall back to system monospace"
            );
        }
        let mut rasterizer = Rasterizer::new().context("Rasterizer::new")?;
        let size = Size::new(size_pt * dpr.max(1.0));
        // Try JetBrains Mono first; fall back to Menlo so the terminal is never
        // blank just because the font file is missing from the .app bundle.
        let font_key = load_font_with_fallback(&mut rasterizer, size)?;
        let metrics: Metrics = rasterizer.metrics(font_key, size).context("font metrics")?;
        let cell_metrics = CellMetrics {
            width_px: f_to_u32(metrics.average_advance),
            height_px: f_to_u32(metrics.line_height),
            baseline: f_to_i32(f64::from(metrics.descent)),
        };
        Ok(Self {
            rasterizer: Arc::new(Mutex::new(rasterizer)),
            font_key,
            size,
            cell_metrics,
            ligatures_enabled: true,
        })
    }

    /// POLISH-02 D-69 ligature toggle (Pattern 5). Runtime no-op for v1 — CoreText
    /// shapes JetBrains Mono ligatures at glyph lookup unconditionally; the toggle
    /// is plumbed for the deferred contiguous-run shaper path.
    pub fn set_ligatures(&mut self, on: bool) {
        self.ligatures_enabled = on;
    }

    #[must_use]
    pub fn ligatures_enabled(&self) -> bool {
        self.ligatures_enabled
    }

    pub fn rasterize(&self, character: char) -> Result<RasterizedGlyph> {
        let key = GlyphKey {
            character,
            font_key: self.font_key,
            size: self.size,
        };
        let mut r = self.rasterizer.lock();
        let g = r
            .get_glyph(key)
            .map_err(|e| anyhow!("get_glyph({character:?}): {e:?}"))?;
        let bitmap = match g.buffer {
            BitmapBuffer::Rgb(b) => BitmapKind::Mono(b),
            BitmapBuffer::Rgba(b) => BitmapKind::Color(b),
        };
        Ok(RasterizedGlyph {
            character,
            width: u32::try_from(g.width.max(0)).unwrap_or(0),
            height: u32::try_from(g.height.max(0)).unwrap_or(0),
            top: g.top,
            left: g.left,
            advance_x: g.advance.0,
            bitmap,
        })
    }
}

fn load_font_with_fallback(rasterizer: &mut Rasterizer, size: Size) -> Result<FontKey> {
    let preferred = FontDesc::new(
        "JetBrains Mono",
        Style::Description {
            slant: Slant::Normal,
            weight: Weight::Normal,
        },
    );
    rasterizer.load_font(&preferred, size).or_else(|_| {
        tracing::warn!("JetBrains Mono unavailable; falling back to Menlo");
        let fallback = FontDesc::new(
            "Menlo",
            Style::Description {
                slant: Slant::Normal,
                weight: Weight::Normal,
            },
        );
        rasterizer
            .load_font(&fallback, size)
            .context("load_font: neither JetBrains Mono nor Menlo available")
    })
}

#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
fn f_to_u32(v: f64) -> u32 {
    v.round().clamp(1.0, f64::from(u32::MAX)) as u32
}

#[allow(clippy::cast_possible_truncation)]
fn f_to_i32(v: f64) -> i32 {
    v.round().clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

/// Best-effort lookup — returns `None` instead of erroring so a missing font
/// never breaks rendering. Checks two cargo-bundle layouts plus the dev path.
fn locate_bundled_font() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            if let Some(grandparent) = parent.parent() {
                // cargo-bundle 0.10 preserves the source `resources/` prefix.
                let with_prefix = grandparent
                    .join("Resources")
                    .join("resources")
                    .join("Fonts")
                    .join("JetBrainsMono-Regular.ttf");
                if with_prefix.exists() {
                    return Some(with_prefix);
                }
                let flat = grandparent
                    .join("Resources")
                    .join("Fonts")
                    .join("JetBrainsMono-Regular.ttf");
                if flat.exists() {
                    return Some(flat);
                }
            }
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dev_path = manifest
        .parent()?
        .join("vector-app")
        .join("resources")
        .join("Fonts")
        .join("JetBrainsMono-Regular.ttf");
    if dev_path.exists() {
        return Some(dev_path);
    }
    None
}
