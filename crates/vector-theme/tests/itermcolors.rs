use vector_theme::{parse_itermcolors, vector_dark, Rgb};

const FIXTURE: &[u8] = include_bytes!("fixtures/Solarized-Dark.itermcolors");

#[test]
fn parses_full_scheme() {
    let palette = parse_itermcolors(FIXTURE).expect("Solarized-Dark.itermcolors parses");
    // Solarized base03 background = #002b36 (0.0, 0.168, 0.211) → clamp + 255 = (0, 43, 54)
    assert_eq!(palette.bg, Rgb::new(0, 43, 54));
    // Foreground = base0 = #839496 (0.514, 0.580, 0.588) → (131, 148, 150)
    assert_eq!(palette.fg, Rgb::new(131, 148, 150));
    // Ansi 0 = base02 = #073642 (0.027, 0.211, 0.258) → (7, 54, 66)
    assert_eq!(palette.ansi[0], Rgb::new(7, 54, 66));

    // UI-SPEC §9.2: chrome is NOT overridden by .itermcolors — it stays from the resolver baseline.
    assert_eq!(
        palette.chrome.search_highlight,
        vector_dark().chrome.search_highlight,
        "chrome MUST NOT be overridden by .itermcolors (UI-SPEC §9.2)"
    );
    assert_eq!(
        palette.chrome.surface,
        vector_dark().chrome.surface,
        "chrome.surface MUST NOT be overridden by .itermcolors (UI-SPEC §9.2)"
    );
}

#[test]
fn unknown_key_warns() {
    // The fixture contains a `Bogus Color` key; parse must succeed (skip + warn).
    let palette = parse_itermcolors(FIXTURE).expect("must not fail on unknown key");
    // Sanity: the parse still produced the known fields.
    assert_eq!(palette.bg, Rgb::new(0, 43, 54));
}
