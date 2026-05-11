//! Cell-coordinate selection state machine. D-54 / RENDER-05.

/// Cell-coordinate selection range. anchor is the down-press cell; cursor is the live drag cell.
/// Ordering is normalized in `cells()` — no requirement that anchor <= cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionRange {
    pub anchor: (u16, u16),
    pub cursor: (u16, u16),
}

impl SelectionRange {
    /// Enumerate cells touched by the selection, in row-major order.
    /// Multi-row selections include full rows between anchor and cursor (inclusive endpoints).
    #[must_use]
    pub fn cells(&self, cols: u16) -> Vec<(u16, u16)> {
        let (a_col, a_row) = self.anchor;
        let (c_col, c_row) = self.cursor;
        let (start_row, start_col, end_row, end_col) = if (a_row, a_col) <= (c_row, c_col) {
            (a_row, a_col, c_row, c_col)
        } else {
            (c_row, c_col, a_row, a_col)
        };
        let last_col = cols.saturating_sub(1);
        let mut out = Vec::new();
        if start_row == end_row {
            for col in start_col..=end_col.min(last_col) {
                out.push((col, start_row));
            }
        } else {
            for col in start_col..=last_col {
                out.push((col, start_row));
            }
            for row in (start_row + 1)..end_row {
                for col in 0..=last_col {
                    out.push((col, row));
                }
            }
            for col in 0..=end_col.min(last_col) {
                out.push((col, end_row));
            }
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionState {
    #[default]
    Idle,
    Dragging(SelectionRange),
    Selected(SelectionRange),
}

impl SelectionState {
    pub fn mouse_down(&mut self, at: (u16, u16)) {
        *self = SelectionState::Dragging(SelectionRange {
            anchor: at,
            cursor: at,
        });
    }

    pub fn mouse_move(&mut self, at: (u16, u16)) {
        if let SelectionState::Dragging(r) = self {
            *self = SelectionState::Dragging(SelectionRange {
                anchor: r.anchor,
                cursor: at,
            });
        }
    }

    pub fn mouse_up(&mut self) {
        if let SelectionState::Dragging(r) = *self {
            *self = SelectionState::Selected(r);
        }
    }

    pub fn clear(&mut self) {
        *self = SelectionState::Idle;
    }

    #[must_use]
    pub fn range(&self) -> Option<SelectionRange> {
        match self {
            SelectionState::Dragging(r) | SelectionState::Selected(r) => Some(*r),
            SelectionState::Idle => None,
        }
    }
}
