use vector_config::Appearance;
use vector_theme::resolve_palette;

#[test]
fn dark_light_flip() {
    // explicit override
    assert_eq!(
        resolve_palette(Appearance::Dark, false).bg,
        vector_theme::vector_dark().bg
    );
    assert_eq!(
        resolve_palette(Appearance::Light, true).bg,
        vector_theme::vector_light().bg
    );
    // system follow
    assert_eq!(
        resolve_palette(Appearance::System, true).bg,
        vector_theme::vector_dark().bg
    );
    assert_eq!(
        resolve_palette(Appearance::System, false).bg,
        vector_theme::vector_light().bg
    );
}
