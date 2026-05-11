//! Stub for Task 2. Real implementation lands there.

use tokio::sync::mpsc;

use crate::bridge::{ResizeCmd, SharedTerm};

pub async fn watch(_term: SharedTerm, _resize_tx: mpsc::Sender<ResizeCmd>) {
    // TODO(02-05 Task 2): SignalKind::window_change + crossterm::terminal::size + propagate.
    std::future::pending::<()>().await;
}
