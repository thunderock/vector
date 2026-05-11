//! Bridge tasks for `vector-headless`.
//!
//! Sole owners of their respective resources — no shared Mutex on the transport:
//! - `pump_pty_to_term`: owns `reader_rx`; pushes bytes into `Term` under
//!   `parking_lot::Mutex` (lock-mutate-drop, never across await). When
//!   `reader_rx` returns `None`, signals exit via oneshot.
//! - `pump_stdin_to_pty`: reads stdin on a `spawn_blocking` thread; forwards
//!   bytes to `write_tx`. Drops `write_tx` on EOF.
//! - `transport_actor`: SOLE owner of `Box<dyn PtyTransport>`. select!s over
//!   `write_rx` + `resize_rx`; calls `transport.wait()` AFTER both channels
//!   close to harvest the exit status.
//!
//! Critical invariant: no `tokio::sync::Mutex` over the transport. `D-11`
//! (`clippy::await_holding_lock = "deny"`) forbids locks held across `.await`.

use std::io::Read;
use std::sync::Arc;

use anyhow::Result;
use parking_lot::Mutex;
use tokio::sync::{mpsc, oneshot};
use vector_mux::PtyTransport;
use vector_term::Term;

pub type SharedTerm = Arc<Mutex<Term>>;

/// Resize command — matches `PtyTransport::resize(rows, cols, px_w, px_h)`.
pub type ResizeCmd = (u16, u16, u16, u16);

/// Pump PTY output into `Term`. Lock-mutate-drop on the `parking_lot::Mutex`;
/// no `.await` inside the guard. When `reader` closes (child PTY EOF — the
/// `spawn_blocking` reader saw 0 bytes), signal the main loop to exit.
pub async fn pump_pty_to_term(
    mut reader: mpsc::Receiver<Vec<u8>>,
    term: SharedTerm,
    exit_signal_tx: oneshot::Sender<()>,
) {
    while let Some(chunk) = reader.recv().await {
        let mut t = term.lock();
        t.feed(&chunk);
        drop(t);
    }
    tracing::info!("pty reader channel closed; signaling exit");
    let _ = exit_signal_tx.send(());
}

/// Pump raw stdin bytes to `write_tx`. stdin is read on a blocking thread
/// because tokio's `AsyncRead` on stdin doesn't play with raw mode reliably
/// on macOS. Drops `write_tx` on EOF.
pub async fn pump_stdin_to_pty(write_tx: mpsc::Sender<Vec<u8>>) -> Result<()> {
    let (chunk_tx, mut chunk_rx) = mpsc::channel::<Vec<u8>>(64);
    tokio::task::spawn_blocking(move || {
        let stdin = std::io::stdin();
        let mut stdin = stdin.lock();
        let mut buf = [0u8; 4096];
        loop {
            match stdin.read(&mut buf) {
                Ok(n) if n > 0 => {
                    if chunk_tx.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                _ => break,
            }
        }
    });
    while let Some(bytes) = chunk_rx.recv().await {
        if write_tx.send(bytes).await.is_err() {
            break;
        }
    }
    Ok(())
}

/// SOLE owner of `Box<dyn PtyTransport>`. Drains `write_rx` + `resize_rx`,
/// then calls `transport.wait()` to harvest the exit code, reporting via
/// `done_tx`. `biased` select! prioritizes resize so SIGWINCH is never
/// starved by a hot write stream.
pub async fn transport_actor(
    mut transport: Box<dyn PtyTransport>,
    mut write_rx: mpsc::Receiver<Vec<u8>>,
    mut resize_rx: mpsc::Receiver<ResizeCmd>,
    done_tx: oneshot::Sender<Result<Option<i32>>>,
) {
    loop {
        tokio::select! {
            biased;
            Some((rows, cols, pw, ph)) = resize_rx.recv() => {
                if let Err(e) = transport.resize(rows, cols, pw, ph) {
                    tracing::warn!("transport.resize failed: {e}");
                }
            }
            Some(bytes) = write_rx.recv() => {
                if let Err(e) = transport.write(&bytes).await {
                    tracing::warn!("transport.write failed: {e}; closing writer side");
                    break;
                }
            }
            else => break,
        }
    }
    let status = transport
        .wait()
        .await
        .map_err(|e| anyhow::anyhow!("transport.wait: {e}"));
    let _ = done_tx.send(status);
}
