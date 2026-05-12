//! Thin wrapper over `alacritty_terminal::Term` — owns the parser + grid.
//! Pitfall 4: feed `&[u8]` directly; never decode UTF-8 here.

use std::collections::VecDeque;
use std::path::PathBuf;

use alacritty_terminal::grid::Grid;
use alacritty_terminal::term::cell::Cell;
use alacritty_terminal::term::{Config, TermMode};
use alacritty_terminal::Term as AlacrittyTerm;

use crate::dims::VectorDims;
use crate::listener::NoopListener;
use crate::osc_sniff::{OscSniff, PromptMark};
use crate::parser::Processor;

const CWD_RING_CAP: usize = 16;
const PROMPT_RING_CAP: usize = 1000;

pub struct Term {
    inner: AlacrittyTerm<NoopListener>,
    parser: Processor,
    osc_parser: vte::Parser,
    osc_sniff: OscSniff,
    cwd_ring: VecDeque<PathBuf>,
    prompt_marks: VecDeque<PromptMark>,
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
            osc_parser: vte::Parser::new(),
            osc_sniff: OscSniff::default(),
            cwd_ring: VecDeque::with_capacity(CWD_RING_CAP),
            prompt_marks: VecDeque::with_capacity(PROMPT_RING_CAP),
            cols,
            rows,
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        // POLISH-04 D-79: sniff OSC 7 + 133 in parallel — observer-only, bytes
        // also flow through alacritty unchanged below.
        self.osc_parser.advance(&mut self.osc_sniff, bytes);
        for cwd in self.osc_sniff.events.cwd_updates.drain(..) {
            if self.cwd_ring.len() >= CWD_RING_CAP {
                self.cwd_ring.pop_front();
            }
            self.cwd_ring.push_back(cwd);
        }
        for mark in self.osc_sniff.events.prompt_marks.drain(..) {
            if self.prompt_marks.len() >= PROMPT_RING_CAP {
                self.prompt_marks.pop_front();
            }
            self.prompt_marks.push_back(mark);
        }
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

    /// Bounded ring of recent OSC 7 cwd updates; most-recent at `back()`.
    pub fn cwd_ring(&self) -> &VecDeque<PathBuf> {
        &self.cwd_ring
    }

    /// Bounded ring of OSC 133 prompt marks (cap 1000 per D-79).
    pub fn prompt_marks(&self) -> &VecDeque<PromptMark> {
        &self.prompt_marks
    }

    pub(crate) fn inner(&self) -> &AlacrittyTerm<NoopListener> {
        &self.inner
    }
}
