//! Plan 09-03: PERSIST-02 SC#2 — zero bytes lost across a transport swap.
//!
//! Encodes the drain-before-swap invariant in the pane actor's reconnect
//! path. If a regression silently drops bytes when the old transport dies
//! mid-stream, these tests fail.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::type_complexity
)]

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use vector_app::frame_tick::{CoalesceBuffer, COALESCE_THRESHOLD};
use vector_app::pty_actor::{pane_io_loop, EventSink};
use vector_app::UserEvent;
use vector_mux::{Domain, PaneId, PtyTransport};

mod common;
use common::{test_sink, FakeTransport, ScriptStep, ScriptedDomain};

const TEST_PANE_ID: PaneId = PaneId(99);

/// Spin until either `cond(snapshot)` becomes true or `deadline` passes.
async fn wait_until<F: Fn(&[u8]) -> bool>(coalesce: &CoalesceBuffer, cond: F, deadline: Duration) {
    let start = std::time::Instant::now();
    loop {
        if cond(&coalesce.peek_snapshot()) {
            return;
        }
        if start.elapsed() >= deadline {
            return;
        }
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

async fn wait_for_event<F: Fn(&UserEvent) -> bool>(
    rx: &mut mpsc::UnboundedReceiver<UserEvent>,
    pred: F,
    deadline: Duration,
) -> bool {
    let timeout = tokio::time::Instant::now() + deadline;
    loop {
        let now = tokio::time::Instant::now();
        if now >= timeout {
            return false;
        }
        let remaining = timeout - now;
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(ev)) => {
                if pred(&ev) {
                    return true;
                }
            }
            _ => return false,
        }
    }
}

/// Test A: bytes pushed into the OLD reader after EOF-signal must end up in
/// the coalesce buffer BEFORE the new transport's bytes. Drain-before-swap.
#[tokio::test]
async fn reconnect_drains_old_transport_before_swap() {
    let (sink, mut rx) = test_sink();

    // Old transport: piped — we control when EOF arrives.
    let (old_transport, old_tx) = FakeTransport::piped();

    // Push 1024 bytes BEFORE dropping the sender. These should land in the
    // coalesce buffer via the active loop; some may arrive via the drain.
    let payload_a: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();

    // New transport: piped — test pushes its bytes after seeing PaneReconnected.
    let new_transport_storage: Arc<
        std::sync::Mutex<Option<(FakeTransport, mpsc::Sender<Vec<u8>>)>>,
    > = Arc::new(std::sync::Mutex::new(Some(FakeTransport::piped())));
    let storage_for_builder = Arc::clone(&new_transport_storage);
    let new_tx_handle: Arc<std::sync::Mutex<Option<mpsc::Sender<Vec<u8>>>>> =
        Arc::new(std::sync::Mutex::new(None));
    let new_tx_for_builder = Arc::clone(&new_tx_handle);

    let steps = vec![
        ScriptStep::Swap(Box::new(move || {
            let (t, tx) = storage_for_builder
                .lock()
                .unwrap()
                .take()
                .expect("Swap builder invoked twice");
            *new_tx_for_builder.lock().unwrap() = Some(tx);
            Box::new(t) as Box<dyn PtyTransport>
        })),
        ScriptStep::PermanentNone,
    ];
    let domain: Arc<dyn Domain> = Arc::new(ScriptedDomain::new(steps));

    let (_wtx, wrx) = mpsc::channel::<Vec<u8>>(8);
    let (_rtx, rrx) = mpsc::channel::<(u16, u16)>(4);
    let coalesce = Arc::new(CoalesceBuffer::new(COALESCE_THRESHOLD));
    let cancel = CancellationToken::new();

    let coalesce_actor = Arc::clone(&coalesce);
    let handle = tokio::spawn(pane_io_loop(
        TEST_PANE_ID,
        Box::new(old_transport) as Box<dyn PtyTransport>,
        Arc::clone(&domain),
        "test".to_string(),
        Arc::clone(&sink) as Arc<dyn EventSink>,
        coalesce_actor,
        wrx,
        rrx,
        cancel,
    ));

    // Push payload A in 32-byte chunks through the OLD transport.
    for chunk in payload_a.chunks(32) {
        old_tx.send(chunk.to_vec()).await.expect("old send");
    }
    // Drop the OLD sender → EOF → actor enters reconnect.
    drop(old_tx);
    // Drop the test's storage handle ref so any leftover refs in builder are released.
    drop(new_transport_storage);

    // Wait for PaneReconnected — drain happened by then.
    let got_reconnected = wait_for_event(
        &mut rx,
        |e| matches!(e, UserEvent::PaneReconnected { .. }),
        Duration::from_secs(2),
    )
    .await;
    assert!(got_reconnected, "PaneReconnected never fired");

    // Snapshot AFTER reconnect, BEFORE any byte goes through the new transport.
    let snapshot_a = coalesce.peek_snapshot();
    assert_eq!(
        snapshot_a.len(),
        1024,
        "expected exactly 1024 bytes drained before swap, got {}",
        snapshot_a.len()
    );
    assert_eq!(
        snapshot_a, payload_a,
        "old-transport bytes corrupted across swap"
    );

    // Push 256 bytes through the NEW transport.
    let payload_b: Vec<u8> = (0..256).map(|i| ((i + 50) % 256) as u8).collect();
    let new_tx = new_tx_handle
        .lock()
        .unwrap()
        .take()
        .expect("new transport sender not captured");
    for chunk in payload_b.chunks(32) {
        new_tx.send(chunk.to_vec()).await.expect("new send");
    }

    // Wait for the coalesce buffer to reach the 1024 + 256 final length.
    wait_until(&coalesce, |snap| snap.len() == 1280, Duration::from_secs(2)).await;

    let final_snapshot = coalesce.peek_snapshot();
    assert_eq!(
        final_snapshot.len(),
        1024 + 256,
        "final buffer length wrong: {}",
        final_snapshot.len()
    );

    // Old half is intact; new half is intact and ordered after the old half.
    assert_eq!(&final_snapshot[..1024], &payload_a[..]);
    assert_eq!(&final_snapshot[1024..], &payload_b[..]);

    drop(new_tx);
    let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
}

/// Deterministic LCG so the "urandom" test is reproducible in CI.
fn lcg_bytes(seed: u64, n: usize) -> Vec<u8> {
    let mut state = seed;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        out.push((state >> 33) as u8);
    }
    out
}

/// Test B: 8 KiB of pseudo-random bytes split across two transports —
/// 4096 through OLD, 4096 through NEW — must arrive byte-identical in
/// the coalesce buffer. SC#2 zero-byte-loss invariant.
#[tokio::test]
async fn reconnect_zero_byte_loss_under_urandom() {
    let (sink, mut rx) = test_sink();

    let payload = lcg_bytes(0xDEAD_BEEF_CAFE_BABE, 8 * 1024);
    let (old_half, new_half) = payload.split_at(4096);
    let old_half = old_half.to_vec();
    let new_half = new_half.to_vec();

    let (old_transport, old_tx) = FakeTransport::piped();

    let new_storage: Arc<std::sync::Mutex<Option<(FakeTransport, mpsc::Sender<Vec<u8>>)>>> =
        Arc::new(std::sync::Mutex::new(Some(FakeTransport::piped())));
    let new_storage_b = Arc::clone(&new_storage);
    let new_tx_handle: Arc<std::sync::Mutex<Option<mpsc::Sender<Vec<u8>>>>> =
        Arc::new(std::sync::Mutex::new(None));
    let new_tx_for_builder = Arc::clone(&new_tx_handle);

    let steps = vec![
        ScriptStep::Swap(Box::new(move || {
            let (t, tx) = new_storage_b.lock().unwrap().take().unwrap();
            *new_tx_for_builder.lock().unwrap() = Some(tx);
            Box::new(t) as Box<dyn PtyTransport>
        })),
        ScriptStep::PermanentNone,
    ];
    let domain: Arc<dyn Domain> = Arc::new(ScriptedDomain::new(steps));

    let (_wtx, wrx) = mpsc::channel::<Vec<u8>>(8);
    let (_rtx, rrx) = mpsc::channel::<(u16, u16)>(4);
    let coalesce = Arc::new(CoalesceBuffer::new(COALESCE_THRESHOLD));
    let cancel = CancellationToken::new();

    let handle = tokio::spawn(pane_io_loop(
        TEST_PANE_ID,
        Box::new(old_transport) as Box<dyn PtyTransport>,
        Arc::clone(&domain),
        "urandom".to_string(),
        Arc::clone(&sink) as Arc<dyn EventSink>,
        Arc::clone(&coalesce),
        wrx,
        rrx,
        cancel,
    ));

    // 64-byte writes through OLD.
    for chunk in old_half.chunks(64) {
        old_tx.send(chunk.to_vec()).await.expect("old send");
    }
    drop(old_tx);
    drop(new_storage);

    // Wait for the swap.
    let ok = wait_for_event(
        &mut rx,
        |e| matches!(e, UserEvent::PaneReconnected { .. }),
        Duration::from_secs(2),
    )
    .await;
    assert!(ok, "PaneReconnected not observed");

    // 64-byte writes through NEW.
    let new_tx = new_tx_handle.lock().unwrap().take().unwrap();
    for chunk in new_half.chunks(64) {
        new_tx.send(chunk.to_vec()).await.expect("new send");
    }

    // Wait for full 8192-byte snapshot.
    wait_until(&coalesce, |snap| snap.len() == 8192, Duration::from_secs(3)).await;

    let snapshot = coalesce.peek_snapshot();
    assert_eq!(snapshot.len(), 8 * 1024, "snapshot length wrong");
    assert_eq!(snapshot, payload, "zero-byte-loss invariant violated");

    drop(new_tx);
    let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
}
