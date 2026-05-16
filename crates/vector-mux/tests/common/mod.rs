//! Shared test helpers for Plan 04-02 mux topology tests.
//!
//! `NoopTransport` is a `PtyTransport` stub that lets tests construct `Pane`s
//! without spawning a real PTY. All methods return Ok/None without doing I/O.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use parking_lot::Mutex;
use tokio::sync::mpsc;
use vector_mux::{Pane, PaneId, PtyTransport, TransportKind};

pub struct NoopTransport;

#[async_trait]
impl PtyTransport for NoopTransport {
    fn resize(&mut self, _rows: u16, _cols: u16, _px_w: u16, _px_h: u16) -> Result<()> {
        Ok(())
    }
    async fn write(&mut self, _bytes: &[u8]) -> Result<()> {
        Ok(())
    }
    fn take_reader(&mut self) -> Option<mpsc::Receiver<Vec<u8>>> {
        None
    }
    fn kind(&self) -> TransportKind {
        TransportKind::Local
    }
    async fn wait(&mut self) -> Result<Option<i32>> {
        Ok(None)
    }
}

/// Construct a `Pane` from a NoopTransport — no I/O, no spawn.
pub fn make_pane(id: PaneId) -> Arc<Pane> {
    let term = Arc::new(Mutex::new(vector_term::Term::new(80, 24, 1000)));
    let transport: Box<dyn PtyTransport> = Box::new(NoopTransport);
    Arc::new(Pane::new(id, term, transport, None, None))
}
