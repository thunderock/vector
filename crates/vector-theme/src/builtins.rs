//! Vector Light + Vector Dark builtin palettes (UI-SPEC §9.1, locked).

// Canonical 24-bit hex color literals — splitter underscores hurt readability here.
#![allow(clippy::unreadable_literal)]

use crate::palette::{ChromePalette, Palette, Rgb, Rgba};

const ANSI_XTERM: [Rgb; 16] = [
    Rgb::from_hex(0x000000),
    Rgb::from_hex(0xcd0000),
    Rgb::from_hex(0x00cd00),
    Rgb::from_hex(0xcdcd00),
    Rgb::from_hex(0x0000ee),
    Rgb::from_hex(0xcd00cd),
    Rgb::from_hex(0x00cdcd),
    Rgb::from_hex(0xe5e5e5),
    Rgb::from_hex(0x7f7f7f),
    Rgb::from_hex(0xff0000),
    Rgb::from_hex(0x00ff00),
    Rgb::from_hex(0xffff00),
    Rgb::from_hex(0x5c5cff),
    Rgb::from_hex(0xff00ff),
    Rgb::from_hex(0x00ffff),
    Rgb::from_hex(0xffffff),
];

pub fn vector_dark() -> Palette {
    Palette {
        ansi: ANSI_XTERM,
        fg: Rgb::from_hex(0xffffff),
        bg: Rgb::from_hex(0x0d1117),
        cursor: Rgb::from_hex(0xc9d1d9),
        selection: Rgb::from_hex(0x264f78),
        bold: Rgb::from_hex(0xffffff),
        chrome: ChromePalette {
            surface: Rgba::from_hex_alpha(0x1c1c1e, 0xe6),
            divider: Rgb::from_hex(0x3a3a3c),
            button: Rgb::from_hex(0x2c2c2e),
            button_hover: Rgb::from_hex(0x3a3a3c),
            selection: Rgba::from_hex_alpha(0x0a84ff, 0x33),
            search_highlight: Rgb::from_hex(0xffd60a),
            warning: Rgb::from_hex(0xffd60a),
            danger_subtle: Rgb::from_hex(0xff453a),
            link: Rgb::from_hex(0x0a84ff),
            fg_muted: Rgb::from_hex(0x8e8e93),
        },
    }
}

pub fn vector_light() -> Palette {
    Palette {
        ansi: ANSI_XTERM,
        fg: Rgb::from_hex(0x1d1d1f),
        bg: Rgb::from_hex(0xffffff),
        cursor: Rgb::from_hex(0x5a5a5f),
        selection: Rgb::from_hex(0xb3d4ff),
        bold: Rgb::from_hex(0x000000),
        chrome: ChromePalette {
            surface: Rgba::from_hex_alpha(0xf4f4f5, 0xe6),
            divider: Rgb::from_hex(0xd1d1d6),
            button: Rgb::from_hex(0xffffff),
            button_hover: Rgb::from_hex(0xe5e5ea),
            selection: Rgba::from_hex_alpha(0x007aff, 0x22),
            search_highlight: Rgb::from_hex(0xff9500),
            warning: Rgb::from_hex(0xff9500),
            danger_subtle: Rgb::from_hex(0xff3b30),
            link: Rgb::from_hex(0x007aff),
            fg_muted: Rgb::from_hex(0x8e8e93),
        },
    }
}
