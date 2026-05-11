//! Parent SIGWINCH watcher. Propagates resizes to `Term` (parking_lot
//! lock-mutate-drop) AND to the transport via mpsc (`transport_actor` owns
//! the transport; we never touch it directly here, so no held lock can be
//! carried across `.await`).
//!
//! Kernel delivers SIGWINCH to the child's foreground pgrp once the
//! transport_actor calls `transport.resize` (CORE-04).

use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;

use crate::bridge::{ResizeCmd, SharedTerm};

pub async fn watch(term: SharedTerm, resize_tx: mpsc::Sender<ResizeCmd>) {
    let mut winch = match signal(SignalKind::window_change()) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("cannot install SIGWINCH handler: {e}");
            return;
        }
    };
    while winch.recv().await.is_some() {
        let (cols, rows) = match crossterm::terminal::size() {
            Ok((c, r)) => (c, r),
            Err(e) => {
                tracing::warn!("crossterm::terminal::size failed: {e}");
                continue;
            }
        };
        tracing::debug!(cols, rows, "parent SIGWINCH; resizing term + transport");
        {
            let mut t = term.lock();
            t.resize(cols, rows);
            drop(t);
        }
        if resize_tx.send((rows, cols, 0, 0)).await.is_err() {
            tracing::info!("transport_actor closed resize_rx; sigwinch watcher exiting");
            return;
        }
    }
}
