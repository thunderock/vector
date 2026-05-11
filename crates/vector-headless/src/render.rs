//! 30Hz full-grid ANSI repaint to parent stdout (D-36).
//!
//! Damage tracking is Phase 3's concern; we full-repaint each tick.

use std::io::{stdout, Write as _};

use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::vte::ansi::{Color, NamedColor};
use anyhow::Result;
use vector_term::Term;

use crate::bridge::SharedTerm;

/// Snapshot the Term grid under the lock, then write the frame to stdout.
/// No `.await` here — render is synchronous (called from the tick branch).
pub fn render_grid_to_stdout(term: &SharedTerm) -> Result<()> {
    let frame = {
        let t = term.lock();
        build_frame(&t)
    };
    let mut out = stdout().lock();
    out.write_all(&frame)?;
    out.flush()?;
    Ok(())
}

#[allow(clippy::similar_names)] // last_fg / last_bg follow the SGR pair convention.
fn build_frame(term: &Term) -> Vec<u8> {
    let (cols, rows) = term.dims();
    let (cur_col, cur_row) = term.cursor();
    let grid = term.grid();
    let mut buf: Vec<u8> = Vec::with_capacity(usize::from(cols) * usize::from(rows) * 8);

    // Hide cursor + home + clear (prevents flicker on cursor relocation mid-frame).
    buf.extend_from_slice(b"\x1b[?25l\x1b[H\x1b[2J");

    let mut last_fg: Option<Color> = None;
    let mut last_bg: Option<Color> = None;

    for row in 0..i32::from(rows) {
        // Cursor home for this row (1-based).
        let row1 = row + 1;
        let _ = write!(&mut buf, "\x1b[{row1};1H");
        let mut col: u16 = 0;
        while col < cols {
            let cell = &grid[Point::new(Line(row), Column(usize::from(col)))];

            // Skip the spacer that follows a WIDE_CHAR cell.
            if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                col += 1;
                continue;
            }

            if last_fg != Some(cell.fg) {
                write_fg(&mut buf, cell.fg);
                last_fg = Some(cell.fg);
            }
            if last_bg != Some(cell.bg) {
                write_bg(&mut buf, cell.bg);
                last_bg = Some(cell.bg);
            }

            let c = cell.c;
            if c.is_control() {
                buf.push(b' ');
            } else {
                let mut tmp = [0u8; 4];
                buf.extend_from_slice(c.encode_utf8(&mut tmp).as_bytes());
            }

            col += 1;
        }
        buf.extend_from_slice(b"\x1b[0m");
        last_fg = None;
        last_bg = None;
    }

    // Reposition cursor + unhide.
    let cur_row1 = cur_row + 1;
    let cur_col1 = cur_col + 1;
    let _ = write!(&mut buf, "\x1b[{cur_row1};{cur_col1}H\x1b[?25h");
    buf
}

fn write_fg(buf: &mut Vec<u8>, c: Color) {
    match c {
        Color::Spec(rgb) => {
            let (r, g, b) = (rgb.r, rgb.g, rgb.b);
            let _ = write!(buf, "\x1b[38;2;{r};{g};{b}m");
        }
        Color::Indexed(i) => {
            let _ = write!(buf, "\x1b[38;5;{i}m");
        }
        Color::Named(NamedColor::Foreground) => buf.extend_from_slice(b"\x1b[39m"),
        Color::Named(n) => {
            let idx = n as u32 & 0xFF;
            let _ = write!(buf, "\x1b[38;5;{idx}m");
        }
    }
}

fn write_bg(buf: &mut Vec<u8>, c: Color) {
    match c {
        Color::Spec(rgb) => {
            let (r, g, b) = (rgb.r, rgb.g, rgb.b);
            let _ = write!(buf, "\x1b[48;2;{r};{g};{b}m");
        }
        Color::Indexed(i) => {
            let _ = write!(buf, "\x1b[48;5;{i}m");
        }
        Color::Named(NamedColor::Background) => buf.extend_from_slice(b"\x1b[49m"),
        Color::Named(n) => {
            let idx = n as u32 & 0xFF;
            let _ = write!(buf, "\x1b[48;5;{idx}m");
        }
    }
}
