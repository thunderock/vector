//! POLISH-05 / D-53 / D-54 carry — selection-to-string extraction for Cmd-C.

use vector_input::{selection_to_string, GridAccess, SelectionMode, SelectionRange};

struct MockGrid {
    rows: Vec<Vec<(Option<char>, bool)>>,
}

impl GridAccess for MockGrid {
    fn cell_char(&self, row: usize, col: usize) -> Option<char> {
        self.rows
            .get(row)
            .and_then(|r| r.get(col))
            .and_then(|c| c.0)
    }
    fn cell_is_wide_spacer(&self, row: usize, col: usize) -> bool {
        self.rows
            .get(row)
            .and_then(|r| r.get(col))
            .is_some_and(|c| c.1)
    }
    fn cols(&self) -> usize {
        self.rows.first().map_or(0, Vec::len)
    }
}

#[test]
fn wide_chars_collapse() {
    // Row: "你好" — each wide char takes 2 cells: char + WIDE_CHAR_SPACER.
    let grid = MockGrid {
        rows: vec![vec![
            (Some('你'), false),
            (None, true),
            (Some('好'), false),
            (None, true),
        ]],
    };
    let range = SelectionRange {
        anchor: (0, 0),
        cursor: (3, 0),
    };
    let s = selection_to_string(&range, &grid, SelectionMode::Stream);
    assert_eq!(s, "你好", "Pitfall 8: spacers MUST be skipped, got {s:?}");
}

#[test]
fn trailing_ws_stripped() {
    let grid = MockGrid {
        rows: vec![vec![
            (Some('h'), false),
            (Some('e'), false),
            (Some('l'), false),
            (Some('l'), false),
            (Some('o'), false),
            (Some(' '), false),
            (Some(' '), false),
            (Some(' '), false),
        ]],
    };
    let range = SelectionRange {
        anchor: (0, 0),
        cursor: (7, 0),
    };
    let s = selection_to_string(&range, &grid, SelectionMode::Stream);
    assert_eq!(s, "hello", "trailing whitespace MUST be stripped per line");
}

#[test]
fn rect_uses_newline() {
    let grid = MockGrid {
        rows: vec![
            vec![(Some('a'), false), (Some('b'), false)],
            vec![(Some('c'), false), (Some('d'), false)],
        ],
    };
    let range = SelectionRange {
        anchor: (0, 0),
        cursor: (1, 1),
    };
    let s = selection_to_string(&range, &grid, SelectionMode::Rectangular);
    assert_eq!(s, "ab\ncd");
}
