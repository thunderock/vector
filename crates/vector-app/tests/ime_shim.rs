//! Plan 05-15 / POLISH-08 / D-81 / Pitfall 9 — ImeState regression tests for
//! the declare_class! NSTextInputClient subclass (appkit_impl module in ime.rs).
//!
//! These tests drive the pure-Rust ImeState layer that the subclass forwards to.
//! The AppKit subclass itself cannot be instantiated in cargo test (no runtime).

use tokio::sync::mpsc;
use vector_app::ime::ImeState;

/// Test 1 (regression): set_preedit activates state; no PTY bytes written.
#[tokio::test(flavor = "current_thread")]
async fn preedit_active_no_pty() {
    let (write_tx, mut write_rx) = mpsc::channel::<Vec<u8>>(16);
    let mut ime = ImeState::new(write_tx);
    ime.set_preedit("こん", 0);
    assert!(ime.is_active(), "preedit must be active after set_preedit");
    assert_eq!(ime.preedit(), "こん");
    // Pitfall 9: preedit NEVER writes to PTY.
    assert!(
        write_rx.try_recv().is_err(),
        "Pitfall 9: setMarkedText must not write PTY bytes"
    );
}

/// Test 2: commit() sends UTF-8 bytes to PTY and clears preedit.
#[tokio::test(flavor = "current_thread")]
async fn commit_sends_utf8() {
    let (write_tx, mut write_rx) = mpsc::channel::<Vec<u8>>(16);
    let mut ime = ImeState::new(write_tx);
    ime.set_preedit("こん", 0);
    let sent = ime.commit("こん");
    assert!(sent, "commit must return true when bytes enqueued");
    let bytes = write_rx.recv().await.expect("commit must write to PTY");
    // UTF-8 encoding of "こん" = 0xe3 0x81 0x93 0xe3 0x82 0x93
    assert_eq!(bytes, "こん".as_bytes(), "PTY must receive exact UTF-8 bytes");
    assert!(!ime.is_active(), "after commit preedit is cleared");
    assert_eq!(ime.preedit(), "");
}

/// Test 3: clear() drops preedit without writing PTY bytes (Pitfall 9).
#[tokio::test(flavor = "current_thread")]
async fn clear_no_pty_bytes() {
    let (write_tx, mut write_rx) = mpsc::channel::<Vec<u8>>(16);
    let mut ime = ImeState::new(write_tx);
    ime.set_preedit("a", 0);
    assert!(ime.is_active());
    ime.clear();
    assert!(!ime.is_active(), "after clear preedit must be inactive");
    assert_eq!(ime.preedit(), "");
    // Pitfall 9: unmarkText must not write PTY bytes.
    assert!(
        write_rx.try_recv().is_err(),
        "Pitfall 9: unmarkText must not write PTY bytes"
    );
}
