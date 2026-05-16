//! WIN-02: Cmd-T -> tab/pane allocation invariants. Plan 04-02.

mod common;

use std::sync::Arc;

use common::make_pane;
use vector_mux::{LocalDomain, Mux};

fn fresh_mux() -> Arc<Mux> {
    let domain = Arc::new(LocalDomain::with_shell("/bin/sh".into()));
    Mux::new(domain)
}

#[test]
fn create_window_then_tab_allocates_ids() {
    let mux = fresh_mux();
    let w1 = mux.create_window();
    let pane = make_pane(mux.allocate_pane_id());
    let pane_id = pane.id;
    let (t1, p1) = mux.install_tab(w1, pane, 24, 80);
    assert_eq!(p1, pane_id);
    assert!(t1.0 > 0);
    assert!(p1.0 > 0);
    assert_eq!(mux.pane_count(), 1);
    assert_eq!(mux.tab_count(w1), 1);
    assert_eq!(mux.active_tab_id(w1), Some(t1));
}

#[test]
fn two_tabs_have_distinct_panes() {
    let mux = fresh_mux();
    let w1 = mux.create_window();
    let p_a = make_pane(mux.allocate_pane_id());
    let id_a = p_a.id;
    let (t1, _) = mux.install_tab(w1, p_a, 24, 80);
    let p_b = make_pane(mux.allocate_pane_id());
    let id_b = p_b.id;
    let (t2, _) = mux.install_tab(w1, p_b, 24, 80);
    assert_ne!(t1, t2);
    assert_ne!(id_a, id_b);
    assert_eq!(mux.tab_count(w1), 2);
    // active_tab moves to the most-recently installed.
    assert_eq!(mux.active_tab_id(w1), Some(t2));
}
