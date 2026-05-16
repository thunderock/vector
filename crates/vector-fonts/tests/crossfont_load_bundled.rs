//! Confirms vector-fonts can locate + load the bundled JetBrains Mono TTF (D-41).

use vector_fonts::FontStack;

#[test]
fn loads_bundled_jetbrains_mono_and_rasterizes_a() {
    let stack = FontStack::load_bundled(1.0, 14.0).expect("load_bundled");
    let glyph = stack.rasterize('A').expect("rasterize A");
    assert!(
        glyph.width > 0,
        "glyph width must be > 0; got {}",
        glyph.width
    );
    assert!(
        glyph.height > 0,
        "glyph height must be > 0; got {}",
        glyph.height
    );
    assert!(
        matches!(glyph.bitmap, vector_fonts::BitmapKind::Mono(_)),
        "ASCII 'A' must be Mono bitmap"
    );
}
