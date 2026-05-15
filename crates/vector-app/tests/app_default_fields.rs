//! Plan 05-14 Task 1 TDD RED — App.search_bar and App.profile_picker default fields.
//!
//! Tests that App::new initialises both state machines with closed/empty defaults
//! before any shortcut is dispatched.

use std::sync::{atomic::AtomicBool, Arc};
use tokio::sync::mpsc;

#[test]
fn app_search_bar_starts_closed() {
    let (write_tx, _write_rx) = mpsc::channel::<Vec<u8>>(64);
    let (resize_tx, _resize_rx) = mpsc::channel::<(u16, u16)>(8);
    let lpm = Arc::new(AtomicBool::new(false));
    let app = vector_app::app::App::new(write_tx, resize_tx, lpm);
    assert!(!app.search_bar_open(), "search_bar.open must start false");
}

#[test]
fn app_profile_picker_starts_empty() {
    let (write_tx, _write_rx) = mpsc::channel::<Vec<u8>>(64);
    let (resize_tx, _resize_rx) = mpsc::channel::<(u16, u16)>(8);
    let lpm = Arc::new(AtomicBool::new(false));
    let app = vector_app::app::App::new(write_tx, resize_tx, lpm);
    assert!(
        app.profile_picker_entries_empty(),
        "profile_picker.entries must start empty"
    );
}
