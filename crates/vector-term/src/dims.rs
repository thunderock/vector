//! Dimensions impl for Term::new — hand-rolled per API-SPIKE.md Q2.

use alacritty_terminal::grid::Dimensions;

#[derive(Debug, Clone, Copy)]
pub(crate) struct VectorDims {
    pub cols: usize,
    pub rows: usize,
}

impl Dimensions for VectorDims {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}
