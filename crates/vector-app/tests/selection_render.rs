//! Plan 03-04 Task 2: pure-Rust contract tests for SelectionState transitions
//! and SelectionRange::cells. Tracks RENDER-05 + D-54.

use vector_input::{SelectionRange, SelectionState};

#[test]
fn mouse_down_enters_dragging() {
    let mut s = SelectionState::default();
    s.mouse_down((5, 3));
    assert!(matches!(s, SelectionState::Dragging(_)));
    let r = s.range().unwrap();
    assert_eq!(r.anchor, (5, 3));
    assert_eq!(r.cursor, (5, 3));
}

#[test]
fn mouse_move_updates_cursor_only() {
    let mut s = SelectionState::default();
    s.mouse_down((5, 3));
    s.mouse_move((10, 3));
    let r = s.range().unwrap();
    assert_eq!(r.anchor, (5, 3));
    assert_eq!(r.cursor, (10, 3));
}

#[test]
fn mouse_up_transitions_to_selected() {
    let mut s = SelectionState::default();
    s.mouse_down((5, 3));
    s.mouse_move((10, 3));
    s.mouse_up();
    assert!(matches!(s, SelectionState::Selected(_)));
}

#[test]
fn single_row_cells_left_to_right() {
    let r = SelectionRange {
        anchor: (2, 1),
        cursor: (5, 1),
    };
    let cells = r.cells(80);
    assert_eq!(cells, vec![(2, 1), (3, 1), (4, 1), (5, 1)]);
}

#[test]
fn anchor_after_cursor_normalizes() {
    let r = SelectionRange {
        anchor: (5, 1),
        cursor: (2, 1),
    };
    let cells = r.cells(80);
    assert_eq!(cells, vec![(2, 1), (3, 1), (4, 1), (5, 1)]);
}

#[test]
fn multi_row_includes_partial_endpoints_and_full_middle() {
    let r = SelectionRange {
        anchor: (3, 0),
        cursor: (1, 2),
    };
    let cells = r.cells(5);
    // Row 0 from col 3 → 4 (partial); row 1 full 0..=4; row 2 from 0 → 1.
    assert_eq!(
        cells,
        vec![
            (3, 0),
            (4, 0),
            (0, 1),
            (1, 1),
            (2, 1),
            (3, 1),
            (4, 1),
            (0, 2),
            (1, 2),
        ]
    );
}
