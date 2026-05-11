//! Plan 03-03 Task 1 plumbing smoke: 24-bit SGR populates Color::Spec(Rgb).

use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::vte::ansi::{Color, Rgb};

#[test]
fn truecolor_sgr_lands_as_rgb_spec() {
    let mut term = vector_term::Term::new(20, 4, 100);
    term.feed(b"\x1b[38;2;255;128;0mZ\x1b[0m");
    let grid = term.grid();
    let cell = &grid[Point::new(Line(0), Column(0))];
    assert_eq!(cell.c, 'Z');
    assert_eq!(
        cell.fg,
        Color::Spec(Rgb {
            r: 255,
            g: 128,
            b: 0,
        })
    );
}
