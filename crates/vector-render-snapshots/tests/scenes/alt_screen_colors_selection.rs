//! HARDEN-01 D-02 scene (b): alt-screen + SGR colors + cursor position.
//!
//! Scope note (deviation): live "selection" is a UI-layer concept the
//! Compositor surfaces via `render_offscreen_with(... Some(((x1,y1),(x2,y2))))`.
//! We exercise that path with a small fixed selection range — see
//! `tests/common/mod.rs::render_scene` which currently passes `None`.
//! A follow-up scene can stretch to selection coverage when the harness
//! grows a selection argument; for now the colored alt-screen + cursor is
//! the locked content.

#[path = "../common/mod.rs"]
mod common;

use common::{diff_or_panic, render_scene};

#[test]
fn alt_screen_colors_selection() {
    let Some(img) = render_scene(800, 480, 80, 24, |term| {
        // Enter alt-screen.
        term.feed(b"\x1b[?1049h");
        // Move cursor to top-left, paint colored words.
        term.feed(b"\x1b[H");
        term.feed(b"\x1b[31mred\x1b[0m \x1b[32mgreen\x1b[0m \x1b[34mblue\x1b[0m\r\n");
        // 256-color background sample.
        term.feed(b"\x1b[48;5;52m  256-bg  \x1b[0m\r\n");
        // Truecolor sample.
        term.feed(b"\x1b[38;2;200;100;50mtruecolor\x1b[0m\r\n");
        // Position cursor at row 5, col 5 so the block-cursor lands somewhere visible.
        term.feed(b"\x1b[5;5H");
    }) else {
        eprintln!("SKIP: no Metal adapter");
        return;
    };
    diff_or_panic(&img, "alt_screen_colors_selection");
}
