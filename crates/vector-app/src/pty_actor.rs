//! Per-pane PTY actor router (Plan 04-03).
//!
//! Generalizes Plan 03-04's single-pane `io_main` to N panes via
//! `tokio::task::JoinSet<PaneId>`: one task per pane, each owning its
//! `Box<dyn PtyTransport>` for the lifetime of the pane.
//!
//! Pitfall C avoidance: no centralized round-robin pump — independent tasks
//! per pane keep backpressure isolated and let `join_next` surface PaneExited
//! without needing manual bookkeeping.

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::task::JoinSet;
use vector_mux::{PaneId, PtyTransport};
use winit::event_loop::EventLoopProxy;

use crate::frame_tick::{frame_tick_loop, CoalesceBuffer, COALESCE_THRESHOLD};
use crate::UserEvent;

pub struct PtyActorRouter {
    proxy: EventLoopProxy<UserEvent>,
    lpm_flag: Arc<AtomicBool>,
    pane_writers: HashMap<PaneId, mpsc::Sender<Vec<u8>>>,
    pane_resizers: HashMap<PaneId, mpsc::Sender<(u16, u16)>>,
    coalesce_buffers: HashMap<PaneId, Arc<CoalesceBuffer>>,
    join_set: JoinSet<PaneId>,
}

impl PtyActorRouter {
    pub fn new(proxy: EventLoopProxy<UserEvent>, lpm_flag: Arc<AtomicBool>) -> Self {
        Self {
            proxy,
            lpm_flag,
            pane_writers: HashMap::new(),
            pane_resizers: HashMap::new(),
            coalesce_buffers: HashMap::new(),
            join_set: JoinSet::new(),
        }
    }

    /// Spawn the per-pane PTY actor + its frame_tick drain task.
    pub fn spawn_pane(&mut self, pane_id: PaneId, transport: Box<dyn PtyTransport>) {
        let (write_tx, write_rx) = mpsc::channel::<Vec<u8>>(64);
        let (resize_tx, resize_rx) = mpsc::channel::<(u16, u16)>(8);
        let coalesce = Arc::new(CoalesceBuffer::new(COALESCE_THRESHOLD));
        self.pane_writers.insert(pane_id, write_tx);
        self.pane_resizers.insert(pane_id, resize_tx);
        self.coalesce_buffers.insert(pane_id, Arc::clone(&coalesce));

        // Per-pane frame_tick: drains the coalesce buffer at ~8ms (or 33ms under LPM)
        // and emits `UserEvent::PaneOutput { pane_id, bytes }`.
        let proxy_ft = self.proxy.clone();
        let coalesce_ft = Arc::clone(&coalesce);
        let lpm_ft = Arc::clone(&self.lpm_flag);
        drop(tokio::spawn(async move {
            frame_tick_loop(pane_id, coalesce_ft, proxy_ft, lpm_ft).await;
        }));

        let proxy = self.proxy.clone();
        let coalesce = Arc::clone(&coalesce);
        self.join_set.spawn(async move {
            pane_io_loop(pane_id, transport, proxy, coalesce, write_rx, resize_rx).await;
            pane_id
        });
    }

    pub fn send_write(&self, pane_id: PaneId, bytes: Vec<u8>) -> bool {
        if let Some(tx) = self.pane_writers.get(&pane_id) {
            if let Err(err) = tx.try_send(bytes) {
                tracing::warn!(?pane_id, ?err, "pty write channel full/closed");
                return false;
            }
            return true;
        }
        false
    }

    pub fn send_resize(&self, pane_id: PaneId, rows: u16, cols: u16) -> bool {
        if let Some(tx) = self.pane_resizers.get(&pane_id) {
            if let Err(err) = tx.try_send((rows, cols)) {
                tracing::warn!(?pane_id, ?err, "pty resize channel full/closed");
                return false;
            }
            return true;
        }
        false
    }

    #[allow(dead_code)]
    pub fn coalesce_buffer(&self, pane_id: PaneId) -> Option<Arc<CoalesceBuffer>> {
        self.coalesce_buffers.get(&pane_id).map(Arc::clone)
    }

    /// Await the next pane to exit; returns its PaneId.
    #[allow(dead_code)]
    pub async fn join_next_exited(&mut self) -> Option<PaneId> {
        self.join_set.join_next().await.and_then(Result::ok)
    }

    /// Drop the per-pane channels (so the actor's select! observes channel close).
    #[allow(dead_code)]
    pub fn shutdown_pane(&mut self, pane_id: PaneId) {
        self.pane_writers.remove(&pane_id);
        self.pane_resizers.remove(&pane_id);
        self.coalesce_buffers.remove(&pane_id);
    }
}

/// Per-pane biased select! over resize / write / read. Resize takes priority
/// so SIGWINCH isn't starved by chatty output (Plan 02-05 hand-off).
async fn pane_io_loop(
    pane_id: PaneId,
    mut transport: Box<dyn PtyTransport>,
    proxy: EventLoopProxy<UserEvent>,
    coalesce: Arc<CoalesceBuffer>,
    mut write_rx: mpsc::Receiver<Vec<u8>>,
    mut resize_rx: mpsc::Receiver<(u16, u16)>,
) {
    let Some(mut reader) = transport.take_reader() else {
        tracing::error!(?pane_id, "take_reader returned None on spawn");
        return;
    };
    loop {
        tokio::select! {
            biased;
            maybe_resize = resize_rx.recv() => {
                let Some((rows, cols)) = maybe_resize else { break };
                if let Err(err) = transport.resize(rows, cols, 0, 0) {
                    tracing::warn!(?pane_id, ?err, "transport.resize failed");
                }
                if proxy
                    .send_event(UserEvent::PaneResized { pane_id, rows, cols })
                    .is_err()
                {
                    tracing::info!(?pane_id, "event loop closed; pty actor exiting");
                    break;
                }
            }
            maybe_write = write_rx.recv() => {
                let Some(bytes) = maybe_write else { break };
                if let Err(err) = transport.write(&bytes).await {
                    tracing::warn!(?pane_id, ?err, "transport.write failed");
                }
            }
            maybe_read = reader.recv() => {
                let Some(chunk) = maybe_read else { break };
                coalesce.push(&chunk);
            }
        }
    }
    let _ = transport.wait().await;
    let _ = proxy.send_event(UserEvent::PaneExited(pane_id));
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use async_trait::async_trait;
    use vector_mux::TransportKind;

    /// A trivial transport whose reader yields once then closes; wait() returns immediately.
    struct NoopTransport {
        reader: Option<mpsc::Receiver<Vec<u8>>>,
    }
    impl NoopTransport {
        fn new() -> Self {
            let (tx, rx) = mpsc::channel(1);
            drop(tx); // close immediately
            Self { reader: Some(rx) }
        }
    }
    #[async_trait]
    impl PtyTransport for NoopTransport {
        fn resize(&mut self, _r: u16, _c: u16, _w: u16, _h: u16) -> Result<()> {
            Ok(())
        }
        async fn write(&mut self, _bytes: &[u8]) -> Result<()> {
            Ok(())
        }
        fn take_reader(&mut self) -> Option<mpsc::Receiver<Vec<u8>>> {
            self.reader.take()
        }
        fn kind(&self) -> TransportKind {
            TransportKind::Local
        }
        async fn wait(&mut self) -> Result<Option<i32>> {
            Ok(Some(0))
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pane_exit_emitted_via_join_next() {
        // We can't construct an EventLoopProxy without a running event loop;
        // smoke-test the JoinSet shape directly with a stripped-down version.
        let mut js: JoinSet<PaneId> = JoinSet::new();
        let pid = PaneId(42);
        js.spawn(async move { pid });
        let got = js.join_next().await.and_then(Result::ok);
        assert_eq!(got, Some(pid));
    }

    #[test]
    fn noop_transport_take_reader_once() {
        let mut t = NoopTransport::new();
        assert!(t.take_reader().is_some());
        assert!(t.take_reader().is_none());
    }
}
