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
}

impl FontStack {
    /// Load the bundled JetBrains Mono Regular face at the given device-pixel ratio + point size.
    /// `dpr` is reserved for future per-DPR rasterizers — crossfont 0.9's CoreText backend takes
    /// no DPR at construction; we pre-multiply into `size_pt` so the CoreText pixel grid lines up.
    pub fn load_bundled(dpr: f32, size_pt: f32) -> Result<Self> {
        let _ttf_path = locate_bundled_font()?;
        let mut rasterizer = Rasterizer::new().context("Rasterizer::new")?;
        let desc = FontDesc::new(
            "JetBrains Mono",
            Style::Description {
                slant: Slant::Normal,
                weight: Weight::Normal,
            },
        );
        let size = Size::new(size_pt * dpr.max(1.0));
        let font_key = rasterizer
            .load_font(&desc, size)
            .context("load_font JetBrains Mono")?;
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
        })
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

#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
fn f_to_u32(v: f64) -> u32 {
    v.round().clamp(1.0, f64::from(u32::MAX)) as u32
}

#[allow(clippy::cast_possible_truncation)]
fn f_to_i32(v: f64) -> i32 {
    v.round().clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

/// Bundle path first (`Vector.app/Contents/Resources/Fonts/`), dev workspace path second.
fn locate_bundled_font() -> Result<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            if let Some(grandparent) = parent.parent() {
                let bundled = grandparent
                    .join("Resources")
                    .join("Fonts")
                    .join("JetBrainsMono-Regular.ttf");
                if bundled.exists() {
                    return Ok(bundled);
                }
            }
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dev_path = manifest
        .parent()
        .ok_or_else(|| anyhow!("CARGO_MANIFEST_DIR has no parent"))?
        .join("vector-app")
        .join("resources")
        .join("Fonts")
        .join("JetBrainsMono-Regular.ttf");
    if dev_path.exists() {
        return Ok(dev_path);
    }
    Err(anyhow!(
        "JetBrains Mono not found at bundle or dev path: {dev_path:?}"
    ))
}
