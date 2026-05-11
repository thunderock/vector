//! Thin wrapper over `alacritty_terminal::Term` — owns the parser + grid.
//! Pitfall 4: feed `&[u8]` directly; never decode UTF-8 here.

use alacritty_terminal::grid::Grid;
use alacritty_terminal::term::cell::Cell;
use alacritty_terminal::term::{Config, TermMode};
use alacritty_terminal::Term as AlacrittyTerm;

use crate::dims::VectorDims;
use crate::listener::NoopListener;
use crate::parser::Processor;

pub struct Term {
    inner: AlacrittyTerm<NoopListener>,
    parser: Processor,
    cols: u16,
    rows: u16,
}

impl Term {
    pub fn new(cols: u16, rows: u16, scrollback: usize) -> Self {
        let config = Config {
            scrolling_history: scrollback,
            ..Config::default()
        };
        let dims = VectorDims {
            cols: cols.into(),
            rows: rows.into(),
        };
        let inner = AlacrittyTerm::new(config, &dims, NoopListener);
        let parser = Processor::new();
        Self {
            inner,
            parser,
            cols,
            rows,
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.inner, bytes);
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let dims = VectorDims {
            cols: cols.into(),
            rows: rows.into(),
        };
        self.inner.resize(dims);
        self.cols = cols;
        self.rows = rows;
    }

    pub fn grid(&self) -> &Grid<Cell> {
        self.inner.grid()
    }

    /// Cursor as (col, row), 0-based, in viewport coordinates.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn cursor(&self) -> (u16, u16) {
        let p = self.inner.grid().cursor.point;
        (p.column.0 as u16, p.line.0 as u16)
    }

    pub fn mode(&self) -> TermMode {
        *self.inner.mode()
    }

    pub fn dims(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    /// Per-row damage iterator for the renderer. `&mut self` per alacritty 0.26 contract.
    pub fn damage(&mut self) -> alacritty_terminal::term::TermDamage<'_> {
        self.inner.damage()
    }

    /// Clear damage tracking after the renderer has consumed it.
    pub fn reset_damage(&mut self) {
        self.inner.reset_damage();
    }

    /// Scroll the display by `delta` lines; positive = into scrollback history.
    pub fn scroll_display(&mut self, delta: i32) {
        use alacritty_terminal::grid::Scroll;
        self.inner.scroll_display(Scroll::Delta(delta));
    }

    /// Current display offset; 0 = live grid, >0 = looking at scrollback.
    pub fn scrollback_offset(&self) -> usize {
        self.inner.grid().display_offset()
    }

    pub(crate) fn inner(&self) -> &AlacrittyTerm<NoopListener> {
        &self.inner
    }
}
