//! `SshChannelTransport::resize` is a synchronous mpsc send.
//! Hundreds of consecutive resizes must not panic, block, or err.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use vector_mux::{PtyTransport, TransportKind};
use vector_ssh::SshChannelTransport;

#[tokio::test]
async fn resize_enqueues_without_panic() {
    let log: Arc<Mutex<Vec<(u16, u16)>>> = Arc::new(Mutex::new(Vec::new()));
    let mut t = SshChannelTransport::for_test_no_channel(log.clone());
    for _ in 0..100 {
        t.resize(24, 80, 0, 0).expect("resize must not err");
    }
    // Allow recorder task to drain the unbounded queue.
    tokio::time::sleep(Duration::from_millis(50)).await;
    let len = log.lock().unwrap().len();
    assert!(len >= 1, "recorder saw at least one resize (got {len})");
}

#[tokio::test]
async fn transport_kind_is_dev_tunnel() {
    let log: Arc<Mutex<Vec<(u16, u16)>>> = Arc::new(Mutex::new(Vec::new()));
    let t = SshChannelTransport::for_test_no_channel(log);
    assert_eq!(t.kind(), TransportKind::DevTunnel);
}

#[tokio::test]
async fn resize_records_rows_cols_order() {
    let log: Arc<Mutex<Vec<(u16, u16)>>> = Arc::new(Mutex::new(Vec::new()));
    let mut t = SshChannelTransport::for_test_no_channel(log.clone());
    t.resize(40, 132, 0, 0).expect("resize");
    tokio::time::sleep(Duration::from_millis(50)).await;
    let entries = log.lock().unwrap().clone();
    assert!(
        entries.contains(&(40, 132)),
        "recorder did not see (rows=40, cols=132): {entries:?}"
    );
}
