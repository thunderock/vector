//! POLISH-02 ligature toggle + Nerd Font fallback smoke (Pattern 5 in 05-RESEARCH).

use vector_fonts::FontStack;

#[test]
fn ligature_glyph_present() {
    let mut fs = FontStack::load_bundled(1.0, 14.0).expect("bundled JetBrains Mono loads");
    fs.set_ligatures(true);
    assert!(fs.ligatures_enabled());
    for c in ['>', '-', '=', '<'] {
        let _ = fs
            .rasterize(c)
            .unwrap_or_else(|e| panic!("ligature-participant glyph {c:?} rasterizes: {e:?}"));
    }
}

#[test]
fn ligature_toggle_off() {
    let mut fs = FontStack::load_bundled(1.0, 14.0).unwrap();
    fs.set_ligatures(false);
    assert!(!fs.ligatures_enabled(), "toggle read-back works");
    // Smoke: rasterization still works regardless of toggle (Pattern 5).
    let _ = fs.rasterize('>').unwrap();
}

#[test]
fn nerd_font_codepoint_renders() {
    let fs = FontStack::load_bundled(1.0, 14.0).unwrap();
    // U+E0A0 = Powerline branch icon. CoreText falls back if no Nerd Font installed;
    // acceptance is "rasterization does not fail" — SOMETHING came back.
    let result = fs.rasterize('\u{E0A0}');
    assert!(
        result.is_ok(),
        "U+E0A0 must rasterize (CoreText fallback chain finds SOMETHING); got {:?}",
        result.err()
    );
}
