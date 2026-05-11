//! Mono glyphs return 3-channel "RGB alphamask" bitmaps (D-50; research finding #1).

use vector_fonts::{BitmapKind, FontStack};

#[test]
fn mono_bitmap_is_three_channel_per_pixel() {
    let stack = FontStack::load_bundled(1.0, 14.0).expect("load_bundled");
    let glyph = stack.rasterize('M').expect("rasterize M");
    match glyph.bitmap {
        BitmapKind::Mono(b) => {
            let expected = glyph.width as usize * glyph.height as usize * 3;
            assert_eq!(b.len(), expected, "mono bitmap len == w*h*3");
        }
        BitmapKind::Color(_) => panic!("ASCII 'M' must rasterize as Mono, not Color"),
    }
}
