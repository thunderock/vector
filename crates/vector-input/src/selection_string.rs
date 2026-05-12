//! POLISH-05 / D-53 / D-54 carry — selection-to-string extraction for Cmd-C.
//!
//! Pitfall 8: cells flagged WIDE_CHAR_SPACER are skipped (right-half of a wide CJK char
//! that already emitted its glyph). Trailing whitespace per row stripped (CONTEXT.md
//! Claude's Discretion). Rectangular joins rows with `\n`; stream preserves grid newlines.

use crate::selection::SelectionRange;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    Stream,
    Rectangular,
}

pub trait GridAccess {
    fn cell_char(&self, row: usize, col: usize) -> Option<char>;
    /// True if the cell is a WIDE_CHAR_SPACER (right-half of a wide char, per Pitfall 8).
    fn cell_is_wide_spacer(&self, row: usize, col: usize) -> bool;
    fn cols(&self) -> usize;
}

/// Walks `range` over `grid`, yielding a UTF-8 string suitable for the clipboard.
/// Skips wide-char spacers (Pitfall 8). Strips trailing whitespace per row. Honors `mode`.
#[must_use]
#[allow(clippy::trivially_copy_pass_by_ref)] // contract: matches plan signature
pub fn selection_to_string<G: GridAccess>(
    range: &SelectionRange,
    grid: &G,
    mode: SelectionMode,
) -> String {
    let mut out = String::new();
    let pairs = iter_rows(range, mode, grid.cols());
    let last_idx = pairs.len().saturating_sub(1);
    for (i, (row_idx, col_range)) in pairs.into_iter().enumerate() {
        let mut row_str = String::new();
        for col in col_range {
            if grid.cell_is_wide_spacer(row_idx, col) {
                continue;
            }
            if let Some(c) = grid.cell_char(row_idx, col) {
                row_str.push(c);
            }
        }
        out.push_str(row_str.trim_end());
        if i != last_idx {
            out.push('\n');
        }
    }
    out
}

/// Yields (row, col_range) pairs for the selection. Stream: anchor row gets
/// [anchor_col, cols), middle rows [0, cols), last row [0, cursor_col+1).
/// Rectangular: every row gets the same [min_col, max_col+1).
#[allow(clippy::trivially_copy_pass_by_ref)]
fn iter_rows(
    range: &SelectionRange,
    mode: SelectionMode,
    cols: usize,
) -> Vec<(usize, std::ops::Range<usize>)> {
    // SelectionRange stores anchor/cursor as (col, row) per Plan 03-04 D-54.
    let (a_col, a_row) = (range.anchor.0 as usize, range.anchor.1 as usize);
    let (c_col, c_row) = (range.cursor.0 as usize, range.cursor.1 as usize);
    let (r0, c0, r1, c1) = if (a_row, a_col) <= (c_row, c_col) {
        (a_row, a_col, c_row, c_col)
    } else {
        (c_row, c_col, a_row, a_col)
    };
    let mut pairs = Vec::new();
    match mode {
        SelectionMode::Stream => {
            if r0 == r1 {
                pairs.push((r0, c0..(c1 + 1).min(cols)));
            } else {
                pairs.push((r0, c0..cols));
                for r in (r0 + 1)..r1 {
                    pairs.push((r, 0..cols));
                }
                pairs.push((r1, 0..(c1 + 1).min(cols)));
            }
        }
        SelectionMode::Rectangular => {
            let lo = c0.min(c1);
            let hi = c0.max(c1) + 1;
            for r in r0..=r1 {
                pairs.push((r, lo..hi.min(cols)));
            }
        }
    }
    pairs
}
