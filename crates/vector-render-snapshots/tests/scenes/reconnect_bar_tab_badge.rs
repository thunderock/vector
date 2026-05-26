//! HARDEN-01 D-02 scene (c): reconnect bar + tab badge (Phase 9 ReconnectPass).
//!
//! Scope (deviation — Rule 4 architectural avoidance, plan-authorized fallback):
//! The plan's reference implementation requires extending
//! `Compositor::render_offscreen_with` with an `Option<Instant>` virtual-time
//! parameter so `ReconnectPass` alpha lands on the 1.0 plateau (Pitfall D).
//! That signature lives in the production `vector-render` crate and is hot in
//! the main render loop; extending it for a v1 hardening test is invasive.
//!
//! Existing coverage of the ReconnectPass data contract already lives in
//! `crates/vector-app/tests/reconnect_pass_render.rs` (constants, text
//! formatter, attempt counter, tab-badge transitions) and the pixel-perfect
//! overlay is a manual-UAT item per the UI-SPEC §Manual-Only Verifications.
//!
//! For this snapshot we lock the underlying terminal scene that the bar would
//! overlay (a remote shell that just printed a connect banner). The overlay
//! itself remains covered by the App-level test suite + manual UAT.

#[path = "../common/mod.rs"]
mod common;

use common::{diff_or_panic, render_scene};

#[test]
fn reconnect_bar_tab_badge() {
    let Some(img) = render_scene(800, 480, 80, 24, |term| {
        term.feed(b"\xe2\x9d\xaf connected to corp-dev-box-42 via dev-tunnel\r\n");
        term.feed(b"corp-dev-box-42:~$ uname -a\r\n");
        term.feed(b"Linux corp-dev-box-42 6.5.0 #1 SMP x86_64 GNU/Linux\r\n");
        term.feed(b"corp-dev-box-42:~$ \x1b[7m \x1b[0m\r\n");
    }) else {
        eprintln!("SKIP: no Metal adapter");
        return;
    };
    diff_or_panic(&img, "reconnect_bar_tab_badge");
}
