//! WIN-03 #3: real PTY tput cols round-trip after resize.
//!
//! Spawns a real shell via LocalDomain::spawn_local, sends `tput cols\n`,
//! parses the output, asserts the column count reflects the resize. The
//! split path is exercised via Mux::split_pane_async + Mux::resize_window;
//! we then issue `tput cols` on the new transport(s) to verify each pane
//! sees its post-redistribute share.

use std::sync::Arc;
use std::time::Duration;

use vector_mux::{LocalDomain, Mux, PtyTransport, SpawnCommand, SplitDirection};

#[tokio::test(flavor = "multi_thread")]
#[ignore = "real-PTY integration; run with --include-ignored"]
async fn tput_cols_round_trip_after_split() {
    // -------- Phase 1: bare LocalDomain::spawn_local round-trip. --------
    let domain = LocalDomain::new().expect("LocalDomain::new");
    let mut spawned = domain
        .spawn_local(SpawnCommand {
            argv: None,
            cwd: None,
            rows: 24,
            cols: 80,
            env: vec![],
        })
        .await
        .expect("spawn_local");
    let mut reader = spawned.transport.take_reader().expect("take_reader");
    drain(&mut reader, Duration::from_millis(200)).await; // chew banner/prompt
    let cols_before = tput_cols(&mut *spawned.transport, &mut reader).await;
    assert!(
        (78..=80).contains(&cols_before),
        "expected ~80 cols before resize, got {cols_before}"
    );

    spawned.transport.resize(24, 160, 0, 0).expect("resize");
    drain(&mut reader, Duration::from_millis(100)).await;
    let cols_after = tput_cols(&mut *spawned.transport, &mut reader).await;
    assert!(
        (158..=160).contains(&cols_after),
        "expected ~160 cols after resize, got {cols_after}"
    );
    drop(spawned); // drop kills the child

    // -------- Phase 2: Mux split path — each pane reads its share. --------
    let mux = Mux::new(Arc::new(LocalDomain::new().expect("LocalDomain::new")));
    let window_id = mux.create_window();
    let (_t, p1) = mux
        .create_tab_async(window_id, None, 24, 80)
        .await
        .expect("create_tab_async");
    let p2 = mux
        .split_pane_async(p1, SplitDirection::Horizontal, None)
        .await
        .expect("split_pane_async");
    let layout = mux.resize_window(window_id, 24, 80);
    assert_eq!(layout.len(), 2, "two panes after split");

    // Drive transport.resize per-pane (Plan 04-03 router does this in main).
    // Also take transport + reader out of each pane for direct I/O in this test.
    let (mut t1, mut r1) = take_pane_io(&mux, p1);
    let (mut t2, mut r2) = take_pane_io(&mux, p2);
    for (pid, rows, cols) in &layout {
        if *pid == p1 {
            t1.resize(*rows, *cols, 0, 0).expect("resize p1");
        }
        if *pid == p2 {
            t2.resize(*rows, *cols, 0, 0).expect("resize p2");
        }
    }
    drain(&mut r1, Duration::from_millis(200)).await;
    drain(&mut r2, Duration::from_millis(200)).await;

    let c1 = tput_cols(&mut *t1, &mut r1).await;
    let c2 = tput_cols(&mut *t2, &mut r2).await;
    assert!(
        c1 > 30 && c1 < 50,
        "p1 cols after split should be ~40, got {c1}"
    );
    assert!(
        c2 > 30 && c2 < 50,
        "p2 cols after split should be ~39, got {c2}"
    );
    let sum = c1 + c2;
    assert!(
        (76..=82).contains(&sum),
        "p1 + p2 cols should be ~79 (80 minus divider), got {sum}"
    );
}

fn take_pane_io(
    mux: &Mux,
    pane_id: vector_mux::PaneId,
) -> (Box<dyn PtyTransport>, tokio::sync::mpsc::Receiver<Vec<u8>>) {
    let pane = mux.pane(pane_id).expect("pane present");
    let mut t = pane.take_transport().expect("take_transport once");
    let r = t.take_reader().expect("take_reader once");
    (t, r)
}

async fn tput_cols(
    transport: &mut dyn PtyTransport,
    reader: &mut tokio::sync::mpsc::Receiver<Vec<u8>>,
) -> u32 {
    transport.write(b"tput cols\n").await.expect("write");
    let buf = drain(reader, Duration::from_millis(600)).await;
    parse_last_decimal(&buf).unwrap_or(0)
}

async fn drain(reader: &mut tokio::sync::mpsc::Receiver<Vec<u8>>, total: Duration) -> Vec<u8> {
    let deadline = tokio::time::Instant::now() + total;
    let mut buf: Vec<u8> = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, reader.recv()).await {
            Ok(Some(chunk)) => buf.extend_from_slice(&chunk),
            Ok(None) | Err(_) => break,
        }
    }
    buf
}

fn parse_last_decimal(buf: &[u8]) -> Option<u32> {
    let s = String::from_utf8_lossy(buf);
    for line in s.lines().rev() {
        let trimmed = line.trim();
        if let Ok(n) = trimmed.parse::<u32>() {
            return Some(n);
        }
    }
    None
}
