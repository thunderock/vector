//! Bitmap kind + RasterizedGlyph wrapper (D-50 + research finding #1).

/// Bitmap kind per glyph.
/// `BitmapKind::Mono` = 3-channel RGB alphamask (CoreText grayscale AA, D-50).
/// `BitmapKind::Color` = 4-channel premultiplied RGBA (emoji fallback).
#[derive(Debug, Clone)]
pub enum BitmapKind {
    Mono(Vec<u8>),
    Color(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct RasterizedGlyph {
    pub character: char,
    pub width: u32,
    pub height: u32,
    pub top: i32,
    pub left: i32,
    pub advance_x: i32,
    pub bitmap: BitmapKind,
}
