//! CORE-02: 24-bit truecolor + 256-color SGR.

use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::vte::ansi::Color;
use vector_term::Term;

#[test]
fn sgr_truecolor_24bit_foreground() {
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"\x1b[38;2;255;128;0mX\x1b[0m");
    let cell = &term.grid()[Point::new(Line(0), Column(0))];
    match cell.fg {
        Color::Spec(rgb) => {
            assert_eq!(rgb.r, 255);
            assert_eq!(rgb.g, 128);
            assert_eq!(rgb.b, 0);
        }
        other => panic!("expected truecolor RGB foreground, got {other:?}"),
    }
}

#[test]
fn sgr_truecolor_24bit_background() {
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"\x1b[48;2;0;128;255mX\x1b[0m");
    let cell = &term.grid()[Point::new(Line(0), Column(0))];
    match cell.bg {
        Color::Spec(rgb) => {
            assert_eq!(rgb.r, 0);
            assert_eq!(rgb.g, 128);
            assert_eq!(rgb.b, 255);
        }
        other => panic!("expected truecolor RGB background, got {other:?}"),
    }
}

#[test]
fn sgr_256_color_indexed_foreground() {
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"\x1b[38;5;196mX\x1b[0m");
    let cell = &term.grid()[Point::new(Line(0), Column(0))];
    match cell.fg {
        Color::Indexed(n) => assert_eq!(n, 196),
        other => panic!("expected 256-color indexed foreground, got {other:?}"),
    }
}
