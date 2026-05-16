//! POLISH-08 / D-81 / Pitfall 9 — NSTextInputClient basic IME state-machine tests.

use tokio::sync::mpsc;
use vector_app::ime::ImeState;

#[tokio::test(flavor = "current_thread")]
async fn preedit_not_to_pty() {
    let (write_tx, mut write_rx) = mpsc::channel::<Vec<u8>>(16);
    let mut ime = ImeState::new(write_tx);
    ime.set_preedit("か", 1);
    assert!(
        ime.is_active(),
        "preedit must be active after setMarkedText"
    );
    let received = write_rx.try_recv();
    assert!(
        received.is_err(),
        "Pitfall 9: setMarkedText MUST NOT write to PTY; got {received:?}",
    );
}

#[tokio::test(flavor = "current_thread")]
async fn commit_to_pty() {
    let (write_tx, mut write_rx) = mpsc::channel::<Vec<u8>>(16);
    let mut ime = ImeState::new(write_tx);
    ime.set_preedit("か", 1);
    let ok = ime.commit("か");
    assert!(ok, "commit must enqueue bytes");
    let bytes = write_rx.recv().await.expect("commit must write to PTY");
    assert_eq!(bytes, "か".as_bytes());
    assert!(
        !ime.is_active(),
        "after commit, preedit cleared + state inactive"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn unmark_clears() {
    let (write_tx, mut write_rx) = mpsc::channel::<Vec<u8>>(16);
    let mut ime = ImeState::new(write_tx);
    ime.set_preedit("か", 1);
    assert!(ime.is_active());
    ime.clear();
    assert!(!ime.is_active());
    assert_eq!(ime.preedit(), "");
    assert!(write_rx.try_recv().is_err(), "unmark must not commit");
}
