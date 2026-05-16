//! Plan 03-03 Task 1 plumbing smoke: feeding 'X' lands a non-empty cell.

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};

#[test]
fn feeding_single_char_writes_to_grid() {
    let mut term = vector_term::Term::new(8, 4, 100);
    term.feed(b"X");
    let grid = term.grid();
    let cell = &grid[Point::new(Line(0), Column(0))];
    assert_eq!(cell.c, 'X');
    let _total = grid.total_lines();
}
