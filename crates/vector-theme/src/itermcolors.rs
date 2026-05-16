//! `.itermcolors` plist importer.
//!
//! **UI-SPEC §9.2 contract:** `.itermcolors` overrides GRID colors only.
//! `Palette.chrome` stays from the caller's appearance baseline (Vector Light/Dark)
//! and is NEVER touched by the imported plist. Implementation: we seed the result
//! palette from `vector_dark()` so its `.chrome` field carries the baseline; the
//! `vector-app` config-apply pipeline replaces `result.chrome` with the active
//! appearance's chrome before use.

use crate::{
    builtins::vector_dark,
    error::ThemeError,
    palette::{Palette, Rgb},
};
use plist::Value;

pub fn parse_itermcolors(bytes: &[u8]) -> Result<Palette, ThemeError> {
    let value: Value = plist::from_bytes(bytes)?;
    let dict = value.as_dictionary().ok_or(ThemeError::NotADict)?;

    let mut palette = vector_dark();
    let mut ansi: [Rgb; 16] = palette.ansi;

    for (key, v) in dict {
        let d = v
            .as_dictionary()
            .ok_or_else(|| ThemeError::Field(key.clone()))?;
        let rgb = read_rgb(d).map_err(|_| ThemeError::Field(key.clone()))?;
        match key.as_str() {
            k if k.starts_with("Ansi ") && k.ends_with(" Color") => {
                let n_str = k.trim_start_matches("Ansi ").trim_end_matches(" Color");
                if let Ok(n) = n_str.parse::<usize>() {
                    if n < 16 {
                        ansi[n] = rgb;
                    }
                } else {
                    tracing::warn!(key = %k, "malformed Ansi key, ignored");
                }
            }
            "Foreground Color" => palette.fg = rgb,
            "Background Color" => palette.bg = rgb,
            "Cursor Color" => palette.cursor = rgb,
            "Selection Color" => palette.selection = rgb,
            "Bold Color" => palette.bold = rgb,
            // UI-SPEC §9.2: ignore any key claiming to set chrome colors.
            "Cursor Text Color"
            | "Selected Text Color"
            | "Tab Color"
            | "Underline Color"
            | "Link Color"
            | "Badge Color" => {
                tracing::debug!(key = %key, "iTerm key not used in Vector (chrome contract)");
            }
            other => tracing::warn!(key = %other, "unknown .itermcolors key, ignored"),
        }
    }
    palette.ansi = ansi;
    Ok(palette)
}

fn read_rgb(d: &plist::Dictionary) -> Result<Rgb, ThemeError> {
    let r = d
        .get("Red Component")
        .and_then(Value::as_real)
        .unwrap_or(0.0);
    let g = d
        .get("Green Component")
        .and_then(Value::as_real)
        .unwrap_or(0.0);
    let b = d
        .get("Blue Component")
        .and_then(Value::as_real)
        .unwrap_or(0.0);
    // Pitfall: legacy schemes have values > 1 (sRGB extended). Clamp.
    Ok(Rgb {
        r: f_to_u8(r),
        g: f_to_u8(g),
        b: f_to_u8(b),
    })
}

/// Clamped `[0, 1]` sRGB component → 0–255 byte. Truncation/sign-loss are
/// impossible after clamp.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn f_to_u8(v: f64) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}
