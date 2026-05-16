//! WIN-02: Cmd-Shift-]/[ next/prev tab cycle. Plan 04-02.

mod common;

use std::sync::Arc;

use common::make_pane;
use vector_mux::{Direction, LocalDomain, Mux};

fn three_tabs() -> (Arc<Mux>, vector_mux::WindowId, [vector_mux::TabId; 3]) {
    let mux = Mux::new(Arc::new(LocalDomain::with_shell("/bin/sh".into())));
    let w = mux.create_window();
    let p1 = make_pane(mux.allocate_pane_id());
    let p2 = make_pane(mux.allocate_pane_id());
    let p3 = make_pane(mux.allocate_pane_id());
    let (t1, _) = mux.install_tab(w, p1, 24, 80);
    let (t2, _) = mux.install_tab(w, p2, 24, 80);
    let (t3, _) = mux.install_tab(w, p3, 24, 80);
    // Reset active to t1 so we have a known starting point.
    cycle_to(&mux, w, t1);
    (mux, w, [t1, t2, t3])
}

fn cycle_to(mux: &Mux, w: vector_mux::WindowId, target: vector_mux::TabId) {
    // Cycle left up to 4 times until active == target.
    for _ in 0..4 {
        if mux.active_tab_id(w) == Some(target) {
            return;
        }
        mux.cycle_tab(w, Direction::Left);
    }
    panic!("could not cycle to {target:?}");
}

#[test]
fn cycle_next_wraps_around() {
    let (mux, w, [t1, t2, t3]) = three_tabs();
    assert_eq!(mux.active_tab_id(w), Some(t1));
    mux.cycle_tab(w, Direction::Right);
    assert_eq!(mux.active_tab_id(w), Some(t2));
    mux.cycle_tab(w, Direction::Right);
    assert_eq!(mux.active_tab_id(w), Some(t3));
    mux.cycle_tab(w, Direction::Right);
    assert_eq!(mux.active_tab_id(w), Some(t1));
}

#[test]
fn cycle_prev_wraps_around() {
    let (mux, w, [t1, t2, t3]) = three_tabs();
    assert_eq!(mux.active_tab_id(w), Some(t1));
    mux.cycle_tab(w, Direction::Left);
    assert_eq!(mux.active_tab_id(w), Some(t3));
    mux.cycle_tab(w, Direction::Left);
    assert_eq!(mux.active_tab_id(w), Some(t2));
    mux.cycle_tab(w, Direction::Left);
    assert_eq!(mux.active_tab_id(w), Some(t1));
}

#[test]
fn cycle_with_one_tab_is_noop() {
    let mux = Mux::new(Arc::new(LocalDomain::with_shell("/bin/sh".into())));
    let w = mux.create_window();
    let p = make_pane(mux.allocate_pane_id());
    let (t1, _) = mux.install_tab(w, p, 24, 80);
    assert_eq!(mux.active_tab_id(w), Some(t1));
    mux.cycle_tab(w, Direction::Right);
    assert_eq!(mux.active_tab_id(w), Some(t1));
    mux.cycle_tab(w, Direction::Left);
    assert_eq!(mux.active_tab_id(w), Some(t1));
}
