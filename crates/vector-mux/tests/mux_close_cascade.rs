//! WIN-02: Cmd-W cascade pane -> tab -> window -> quit (D-61). Plan 04-02.

mod common;

use std::sync::Arc;

use common::make_pane;
use vector_mux::{CloseResult, LocalDomain, Mux, SplitDirection};

fn fresh_mux() -> Arc<Mux> {
    Mux::new(Arc::new(LocalDomain::with_shell("/bin/sh".into())))
}

#[test]
fn close_pane_with_sibling_returns_pane_closed() {
    let mux = fresh_mux();
    let w = mux.create_window();
    let p1 = make_pane(mux.allocate_pane_id());
    let id1 = p1.id;
    let (tab_id, _) = mux.install_tab(w, p1, 24, 80);

    let p2 = make_pane(mux.allocate_pane_id());
    let id2 = p2.id;
    mux.split_pane(id1, SplitDirection::Horizontal, p2)
        .expect("split should succeed on 80-col viewport");

    let result = mux.close_pane(id1);
    assert_eq!(result, CloseResult::PaneClosed { tab_id });
    assert_eq!(mux.pane_count(), 1);
    assert!(mux.pane(id1).is_none());
    // Remaining tab is now a Leaf of id2 + active_pane points at it.
    let (active_pane,) = mux
        .with_tab(w, tab_id, |t| (t.active_pane_id,))
        .expect("tab still exists");
    assert_eq!(active_pane, id2);
}

#[test]
fn close_last_pane_in_tab_with_sibling_tab_returns_tab_closed() {
    let mux = fresh_mux();
    let w = mux.create_window();
    let p1 = make_pane(mux.allocate_pane_id());
    let id1 = p1.id;
    let (_t1, _) = mux.install_tab(w, p1, 24, 80);
    let p2 = make_pane(mux.allocate_pane_id());
    let (t2, _) = mux.install_tab(w, p2, 24, 80);

    let result = mux.close_pane(id1);
    assert_eq!(result, CloseResult::TabClosed { window_id: w });
    assert_eq!(mux.tab_count(w), 1);
    assert_eq!(mux.active_tab_id(w), Some(t2));
}

#[test]
fn close_last_pane_in_last_tab_with_sibling_window_returns_window_closed() {
    let mux = fresh_mux();
    let w1 = mux.create_window();
    let w2 = mux.create_window();
    let p1 = make_pane(mux.allocate_pane_id());
    let id1 = p1.id;
    mux.install_tab(w1, p1, 24, 80);
    let p2 = make_pane(mux.allocate_pane_id());
    mux.install_tab(w2, p2, 24, 80);

    let result = mux.close_pane(id1);
    assert_eq!(result, CloseResult::WindowClosed { window_id: w1 });
    assert_eq!(mux.window_count(), 1);
}

#[test]
fn close_last_pane_overall_returns_last_window_closed() {
    let mux = fresh_mux();
    let w = mux.create_window();
    let p1 = make_pane(mux.allocate_pane_id());
    let id1 = p1.id;
    mux.install_tab(w, p1, 24, 80);

    let result = mux.close_pane(id1);
    assert_eq!(result, CloseResult::LastWindowClosed);
    assert_eq!(mux.window_count(), 0);
    assert_eq!(mux.pane_count(), 0);
}
