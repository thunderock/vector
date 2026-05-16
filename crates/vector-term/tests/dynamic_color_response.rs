//! POLISH-04 — OSC 10/11/12 dynamic color queries round-trip via ForwardingListener.

use tokio::sync::mpsc;
use vector_term::Term;

#[tokio::test(flavor = "current_thread")]
async fn osc10_query_response() {
    let (write_tx, mut write_rx) = mpsc::channel::<Vec<u8>>(16);
    let (clip_tx, _clip_rx) = mpsc::channel(16);
    let mut t = Term::with_channels(80, 24, 1000, write_tx, clip_tx);
    // OSC 10 (foreground) query — terminal must reply via PtyWrite event.
    t.feed(b"\x1b]10;?\x07");
    let reply = write_rx
        .recv()
        .await
        .expect("OSC 10 query MUST yield a PtyWrite reply");
    assert!(
        reply.starts_with(b"\x1b]10;"),
        "expected OSC 10 reply prefix, got {:?}",
        String::from_utf8_lossy(&reply)
    );
}
