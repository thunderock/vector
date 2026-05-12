//! D-72 appearance resolver. Pure-Rust — no AppKit linkage; vector-app reads
//! `NSApplication.effectiveAppearance` and passes `system_is_dark` here.

use crate::{
    builtins::{vector_dark, vector_light},
    palette::Palette,
};
use vector_config::Appearance;

pub fn resolve_palette(appearance: Appearance, system_is_dark: bool) -> Palette {
    match appearance {
        Appearance::Dark => vector_dark(),
        Appearance::Light => vector_light(),
        Appearance::System => {
            if system_is_dark {
                vector_dark()
            } else {
                vector_light()
            }
        }
    }
}
