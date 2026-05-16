//! Palette + ChromePalette + Rgb/Rgba — UI-SPEC §9.1 contract.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xff) as u8,
            g: ((hex >> 8) & 0xff) as u8,
            b: (hex & 0xff) as u8,
        }
    }
}

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    pub const fn from_hex_alpha(hex: u32, a: u8) -> Self {
        Self {
            r: ((hex >> 16) & 0xff) as u8,
            g: ((hex >> 8) & 0xff) as u8,
            b: (hex & 0xff) as u8,
            a,
        }
    }
}

/// Grid (cell) colors + chrome surface tokens. `.itermcolors` overlay overrides
/// only the grid fields, NEVER `chrome` (UI-SPEC §9.2).
#[derive(Debug, Clone)]
pub struct Palette {
    pub ansi: [Rgb; 16],
    pub fg: Rgb,
    pub bg: Rgb,
    pub cursor: Rgb,
    pub selection: Rgb,
    pub bold: Rgb,
    pub chrome: ChromePalette,
}

/// Chrome surface colors per UI-SPEC §9.1. `.itermcolors` does NOT touch these.
#[derive(Debug, Clone, Copy)]
pub struct ChromePalette {
    pub surface: Rgba,
    pub divider: Rgb,
    pub button: Rgb,
    pub button_hover: Rgb,
    pub selection: Rgba,
    pub search_highlight: Rgb,
    pub warning: Rgb,
    pub danger_subtle: Rgb,
    pub link: Rgb,
    pub fg_muted: Rgb,
}
