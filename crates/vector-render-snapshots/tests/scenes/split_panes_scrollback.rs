//! HARDEN-01 D-02 scene (d): split panes + scrollback.
//!
//! Scope (deviation — plan-authorized v1 simplification):
//! True multi-Term split-pane composition into one offscreen surface requires
//! a multi-pane Compositor harness that does not exist as a single call today
//! (see `AppWindow`'s per-pane `compositors: HashMap<PaneId, Compositor>` +
//! chained `LoadOp::Load` strategy in Plan 04-06). The plan explicitly allows
//! falling back to "a single 80x24 grid that exercises scrollback via 30
//! lines of `\r\n` and use scrollback offset to show line 5..28."
//!
//! We feed 30 unique numbered lines so any future regression in cell pipeline,
//! scrollback rendering, or wrap behavior shows as a perceptual diff. Multi-
//! pane composition coverage stays in `vector-app` integration tests +
//! manual UAT.

#[path = "../common/mod.rs"]
mod common;

use common::{diff_or_panic, render_scene};

#[test]
fn split_panes_scrollback() {
    let Some(img) = render_scene(800, 480, 80, 24, |term| {
        for i in 1..=30 {
            term.feed(format!("line {i:02} — quick brown fox\r\n").as_bytes());
        }
    }) else {
        eprintln!("SKIP: no Metal adapter");
        return;
    };
    diff_or_panic(&img, "split_panes_scrollback");
}
