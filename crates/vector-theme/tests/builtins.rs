#![allow(clippy::unreadable_literal)]

use vector_theme::{vector_dark, vector_light};

#[test]
fn builtins_loadable() {
    let d = vector_dark();
    assert_eq!(d.bg, vector_theme::Rgb::new(0x0d, 0x11, 0x17));
    assert_eq!(
        d.chrome.surface.a, 0xe6,
        "chrome.surface alpha must be 230 (UI-SPEC §9.1)"
    );

    let l = vector_light();
    assert_eq!(l.bg, vector_theme::Rgb::new(0xff, 0xff, 0xff));
    assert_eq!(
        l.chrome.search_highlight,
        vector_theme::Rgb::from_hex(0xff9500),
        "Light search highlight is orange (UI-SPEC §9.1)"
    );

    assert_eq!(
        d.chrome.search_highlight,
        vector_theme::Rgb::from_hex(0xffd60a),
        "Dark search highlight is yellow (UI-SPEC §9.1)"
    );
}
