//! HARDEN-02 Scenario 6: OSC 52 raw + DCS-wrapped round-trip.
//! Maps to PITFALLS.md Pitfall 8 (DCS passthrough).
//! Mirrors `crates/vector-term/tests/osc52.rs`.

use tokio::sync::mpsc;
use vector_term::{listener::ClipboardEvent, Term};

#[tokio::test(flavor = "current_thread")]
async fn raw_osc52_round_trip() {
    let (write_tx, _wrx) = mpsc::channel(16);
    let (clip_tx, mut clip_rx) = mpsc::channel(16);
    let mut t = Term::with_channels(80, 24, 1000, write_tx, clip_tx);

    // OSC 52 raw: "hello" base64 = "aGVsbG8=".
    t.feed(b"\x1b]52;c;aGVsbG8=\x07");

    let ev = tokio::time::timeout(std::time::Duration::from_millis(100), clip_rx.recv())
        .await
        .expect("raw OSC 52 must deliver ClipboardStore within 100ms")
        .expect("channel closed");
    match ev {
        ClipboardEvent::Store(_kind, data) => assert_eq!(data, "hello"),
        ClipboardEvent::LoadDenied => panic!("expected Store, got LoadDenied"),
    }
}

#[tokio::test(flavor = "current_thread")]
async fn dcs_wrapped_osc52_round_trip() {
    let (write_tx, _wrx) = mpsc::channel(16);
    let (clip_tx, mut clip_rx) = mpsc::channel(16);
    let mut t = Term::with_channels(80, 24, 1000, write_tx, clip_tx);

    // DCS-wrapped OSC 52: ESC P ESC ] 52 ; c ; aGVsbG8= BEL ESC \
    t.feed(b"\x1bP\x1b]52;c;aGVsbG8=\x07\x1b\\");

    let ev = tokio::time::timeout(std::time::Duration::from_millis(100), clip_rx.recv())
        .await
        .expect("DCS-wrapped OSC 52 must deliver ClipboardStore within 100ms")
        .expect("channel closed");
    match ev {
        ClipboardEvent::Store(_kind, data) => assert_eq!(data, "hello"),
        ClipboardEvent::LoadDenied => panic!("expected Store, got LoadDenied"),
    }
}
