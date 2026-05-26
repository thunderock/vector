//! POLISH-05 D-70/D-71 — OSC 52 inbound: raw + DCS-wrapped Store, query Denied.
//! NOTE: HARDEN-02 corpus mirrors round-trip scenarios at crates/vector-term/tests/vt_conformance/osc52_round_trip.rs.
//!
//! Consumes the ForwardingListener pipeline created by Plan 05-05.
//! - `raw_clipboard_store`: alacritty native OSC 52 path fires ClipboardStore
//!   with the base64-DECODED payload.
//! - `dcs_wrapped_round_trip`: Open Question #1 — empirically resolved:
//!   alacritty_terminal 0.26 auto-peels DCS envelopes containing OSC.
//! - `read_denied`: D-70 v1 — OSC 52 reads denied at alacritty's `Osc52::OnlyCopy`
//!   default layer (silent denial: neither clipboard nor PTY write event fires).

use tokio::sync::mpsc;
use vector_term::{listener::ClipboardEvent, Term};

#[tokio::test(flavor = "current_thread")]
async fn raw_clipboard_store() {
    let (write_tx, _wrx) = mpsc::channel(16);
    let (clip_tx, mut clip_rx) = mpsc::channel(16);
    let mut t = Term::with_channels(80, 24, 1000, write_tx, clip_tx);

    // OSC 52 raw: "hello" base64 = "aGVsbG8="
    t.feed(b"\x1b]52;c;aGVsbG8=\x07");

    let ev = clip_rx.recv().await.expect("ClipboardStore must arrive");
    match ev {
        ClipboardEvent::Store(_kind, data) => assert_eq!(data, "hello"),
        ClipboardEvent::LoadDenied => panic!("expected Store, got LoadDenied"),
    }
    // Empirically: alacritty_terminal 0.26 base64-decodes OSC 52 payload before
    // delivering ClipboardStore — the data is the decoded plaintext, not the b64.
}

#[tokio::test(flavor = "current_thread")]
async fn dcs_wrapped_round_trip() {
    // Empirically confirmed 2026-05-12: alacritty_terminal 0.26 auto-peels DCS envelopes
    // containing OSC sequences — no additional DCS-unwrap shim required in osc_sniff.rs.
    // (Open Question #1 resolved.)
    let (write_tx, _wrx) = mpsc::channel(16);
    let (clip_tx, mut clip_rx) = mpsc::channel(16);
    let mut t = Term::with_channels(80, 24, 1000, write_tx, clip_tx);

    // DCS-wrapped OSC 52: ESC P ESC ] 52 ; c ; aGVsbG8= BEL ESC \
    t.feed(b"\x1bP\x1b]52;c;aGVsbG8=\x07\x1b\\");

    let ev = tokio::time::timeout(std::time::Duration::from_millis(100), clip_rx.recv())
        .await
        .expect("DCS-wrapped OSC 52 MUST surface ClipboardStore within 100ms")
        .expect("channel closed");
    match ev {
        ClipboardEvent::Store(_kind, data) => assert_eq!(data, "hello"),
        ClipboardEvent::LoadDenied => panic!("expected Store, got LoadDenied"),
    }
}

#[tokio::test(flavor = "current_thread")]
async fn read_denied() {
    // D-70 v1: OSC 52 reads MUST be denied. alacritty_terminal 0.26's default
    // `Osc52::OnlyCopy` mode SILENTLY denies reads at the term layer — it never
    // invokes the listener's `Event::ClipboardLoad`, so no PtyWrite reply is
    // emitted and the clipboard channel receives nothing. The denial is
    // observable as the *absence* of any clipboard or write event after a query.
    let (write_tx, mut write_rx) = mpsc::channel(16);
    let (clip_tx, mut clip_rx) = mpsc::channel(16);
    let mut t = Term::with_channels(80, 24, 1000, write_tx, clip_tx);

    // OSC 52 read query: "?".
    t.feed(b"\x1b]52;c;?\x07");

    // Both channels MUST remain silent within 50ms (denial is silent).
    let no_clip = tokio::time::timeout(std::time::Duration::from_millis(50), clip_rx.recv()).await;
    assert!(
        no_clip.is_err(),
        "D-70: OSC 52 read must NOT yield ClipboardEvent (got {no_clip:?})"
    );
    let no_write =
        tokio::time::timeout(std::time::Duration::from_millis(50), write_rx.recv()).await;
    assert!(
        no_write.is_err(),
        "D-70: OSC 52 read must NOT yield PTY write reply (got {no_write:?})"
    );
}
