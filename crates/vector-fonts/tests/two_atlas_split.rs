//! ASCII -> Mono and emoji 🦀 -> Color via the CoreText fallback (RENDER-04).

use vector_fonts::{BitmapKind, FontStack};

#[test]
#[cfg(target_os = "macos")]
fn ascii_is_mono_emoji_is_color() {
    let stack = FontStack::load_bundled(1.0, 14.0).expect("load_bundled");
    let ascii = stack.rasterize('A').expect("rasterize A");
    assert!(
        matches!(ascii.bitmap, BitmapKind::Mono(_)),
        "'A' must be Mono"
    );
    let emoji = stack.rasterize('\u{1F980}').expect("rasterize crab emoji");
    assert!(
        matches!(emoji.bitmap, BitmapKind::Color(_)),
        "crab emoji must fall through to Apple Color Emoji as Color"
    );
}
