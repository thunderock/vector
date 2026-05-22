//! Per-pane PTY actor router (Plan 04-03 + Plan 09-03).
//!
//! Generalizes Plan 03-04's single-pane `io_main` to N panes via
//! `tokio::task::JoinSet<PaneId>`: one task per pane, each owning its
//! `Box<dyn PtyTransport>` for the lifetime of the pane.
//!
//! Plan 09-03 extends the per-pane actor with an Active → Reconnecting →
//! Swapping → Active state machine for PERSIST-01/02. Reader EOF triggers
//! drain-before-swap, then `Domain::reconnect_one_shot` with exponential
//! backoff (1/2/4/8/16/30 s cap; D-08). `Ok(None)` → exit cleanly;
//! `CancellationToken::cancel()` aborts in-flight sleeps within ms.

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use vector_mux::{Domain, PaneId, PtyTransport};
use winit::event_loop::EventLoopProxy;

use crate::frame_tick::{frame_tick_loop, CoalesceBuffer, COALESCE_THRESHOLD};
use crate::UserEvent;

/// CONTEXT.md D-08: 1/2/4/8/16/30 cap; attempts past the cap repeat 30 s indefinitely.
const BACKOFF_SCHEDULE_SECS: &[u64] = &[1, 2, 4, 8, 16, 30];

/// Abstraction over the winit `EventLoopProxy<UserEvent>` so unit tests can
/// drive the actor without spinning up a real event loop. Production wires
/// `Arc::new(ProxyEventSink(proxy)) as Arc<dyn EventSink>` once at
/// `PtyActorRouter` construction. Tests build their own `Arc<dyn EventSink>`
/// backed by an `mpsc::UnboundedSender<UserEvent>`.
pub trait EventSink: Send + Sync + 'static {
    fn send_user_event(&self, event: UserEvent);
}

/// Newtype wrapper over `EventLoopProxy<UserEvent>` — keeps the `EventSink`
/// impl on a type defined in this crate and gives tests a stable target.
pub struct ProxyEventSink(pub EventLoopProxy<UserEvent>);

impl EventSink for ProxyEventSink {
    fn send_user_event(&self, event: UserEvent) {
        let _ = self.0.send_event(event);
    }
}

pub struct PtyActorRouter {
    sink: Arc<dyn EventSink>,
    lpm_flag: Arc<AtomicBool>,
    pane_writers: HashMap<PaneId, mpsc::Sender<Vec<u8>>>,
    pane_resizers: HashMap<PaneId, mpsc::Sender<(u16, u16)>>,
    coalesce_buffers: HashMap<PaneId, Arc<CoalesceBuffer>>,
    // Plan 09-03: per-pane frame_tick still needs a raw proxy (it doesn't go
    // through the actor's EventSink path). Keep one canonical proxy for that.
    proxy_for_frame_tick: EventLoopProxy<UserEvent>,
    join_set: JoinSet<PaneId>,
}

impl PtyActorRouter {
    pub fn new(proxy: EventLoopProxy<UserEvent>, lpm_flag: Arc<AtomicBool>) -> Self {
        let sink: Arc<dyn EventSink> = Arc::new(ProxyEventSink(proxy.clone()));
        Self {
            sink,
            lpm_flag,
            pane_writers: HashMap::new(),
            pane_resizers: HashMap::new(),
            coalesce_buffers: HashMap::new(),
            proxy_for_frame_tick: proxy,
            join_set: JoinSet::new(),
        }
    }

    /// Spawn the per-pane PTY actor + its frame_tick drain task.
    ///
    /// Plan 09-03 signature: `domain`, `profile_label`, `cancel` added so the
    /// actor can drive `reconnect_one_shot` on transport EOF.
    pub fn spawn_pane(
        &mut self,
        pane_id: PaneId,
        transport: Box<dyn PtyTransport>,
        domain: Arc<dyn Domain>,
        profile_label: String,
        cancel: CancellationToken,
    ) {
        let (write_tx, write_rx) = mpsc::channel::<Vec<u8>>(64);
        let (resize_tx, resize_rx) = mpsc::channel::<(u16, u16)>(8);
        let coalesce = Arc::new(CoalesceBuffer::new(COALESCE_THRESHOLD));
        self.pane_writers.insert(pane_id, write_tx);
        self.pane_resizers.insert(pane_id, resize_tx);
        self.coalesce_buffers.insert(pane_id, Arc::clone(&coalesce));

        // Per-pane frame_tick: drains the coalesce buffer at ~8ms (or 33ms under LPM)
        // and emits `UserEvent::PaneOutput { pane_id, bytes }`.
        let proxy_ft = self.proxy_for_frame_tick.clone();
        let coalesce_ft = Arc::clone(&coalesce);
        let lpm_ft = Arc::clone(&self.lpm_flag);
        drop(tokio::spawn(async move {
            frame_tick_loop(pane_id, coalesce_ft, proxy_ft, lpm_ft).await;
        }));

        let sink = Arc::clone(&self.sink);
        let coalesce = Arc::clone(&coalesce);
        self.join_set.spawn(async move {
            pane_io_loop(
                pane_id,
                transport,
                domain,
                profile_label,
                sink,
                coalesce,
                write_rx,
                resize_rx,
                cancel,
            )
            .await;
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

/// Per-pane Active → Reconnecting → Swapping → Active state machine
/// (Plan 09-03 / PERSIST-01/02). Exits cleanly only on cancellation,
/// upstream channel close, or `Domain::reconnect_one_shot` returning
/// `Ok(None)` (LocalDomain shell-death path).
pub(crate) async fn pane_io_loop(
    pane_id: PaneId,
    mut transport: Box<dyn PtyTransport>,
    domain: Arc<dyn Domain>,
    profile_label: String,
    sink: Arc<dyn EventSink>,
    coalesce: Arc<CoalesceBuffer>,
    mut write_rx: mpsc::Receiver<Vec<u8>>,
    mut resize_rx: mpsc::Receiver<(u16, u16)>,
    cancel: CancellationToken,
) {
    let mut latest_rows: u16 = 24;
    let mut latest_cols: u16 = 80;
    'outer: loop {
        let Some(mut reader) = transport.take_reader() else {
            tracing::error!(?pane_id, "take_reader returned None");
            break 'outer;
        };
        let exit = run_active_segment(
            pane_id,
            &mut transport,
            &mut reader,
            &coalesce,
            &mut write_rx,
            &mut resize_rx,
            &sink,
            &cancel,
            &mut latest_rows,
            &mut latest_cols,
        )
        .await;
        match exit {
            ActiveExit::CancelOrChannelClosed => break 'outer,
            ActiveExit::TransportDead => {
                // Drain anything left in the old reader queue (Pattern 2).
                drain_reader_to_end(&mut reader, &coalesce).await;
                drop(reader);
                // Wait for the dying transport's wait() so we don't leak the pump task.
                let _ = transport.wait().await;

                match reconnect_with_backoff(
                    &domain,
                    &profile_label,
                    &sink,
                    pane_id,
                    latest_rows,
                    latest_cols,
                    &cancel,
                )
                .await
                {
                    ReconnectOutcome::Cancelled | ReconnectOutcome::PermanentNone => break 'outer,
                    ReconnectOutcome::Swapped(new_transport) => {
                        transport = new_transport;
                        if let Err(err) = transport.resize(latest_rows, latest_cols, 0, 0) {
                            tracing::warn!(?pane_id, ?err, "post-reconnect resize failed");
                        }
                        sink.send_user_event(UserEvent::PaneReconnected { pane_id });
                        continue 'outer;
                    }
                }
            }
        }
    }
    sink.send_user_event(UserEvent::PaneExited(pane_id));
}

enum ActiveExit {
    /// `cancel.cancelled()` fired OR `write_rx` / `resize_rx` closed (parent dropped).
    CancelOrChannelClosed,
    /// Reader returned None — transport is dead.
    TransportDead,
}

async fn run_active_segment(
    pane_id: PaneId,
    transport: &mut Box<dyn PtyTransport>,
    reader: &mut mpsc::Receiver<Vec<u8>>,
    coalesce: &Arc<CoalesceBuffer>,
    write_rx: &mut mpsc::Receiver<Vec<u8>>,
    resize_rx: &mut mpsc::Receiver<(u16, u16)>,
    sink: &Arc<dyn EventSink>,
    cancel: &CancellationToken,
    latest_rows: &mut u16,
    latest_cols: &mut u16,
) -> ActiveExit {
    loop {
        tokio::select! {
            biased;
            () = cancel.cancelled() => return ActiveExit::CancelOrChannelClosed,
            maybe_resize = resize_rx.recv() => {
                let Some((rows, cols)) = maybe_resize else {
                    return ActiveExit::CancelOrChannelClosed;
                };
                *latest_rows = rows;
                *latest_cols = cols;
                if let Err(err) = transport.resize(rows, cols, 0, 0) {
                    tracing::warn!(?pane_id, ?err, "transport.resize failed");
                }
                sink.send_user_event(UserEvent::PaneResized { pane_id, rows, cols });
            }
            maybe_write = write_rx.recv() => {
                let Some(bytes) = maybe_write else {
                    return ActiveExit::CancelOrChannelClosed;
                };
                if let Err(err) = transport.write(&bytes).await {
                    tracing::warn!(?pane_id, ?err, "transport.write failed");
                }
            }
            maybe_read = reader.recv() => {
                match maybe_read {
                    Some(chunk) => coalesce.push(&chunk),
                    None => return ActiveExit::TransportDead,
                }
            }
        }
    }
}

async fn drain_reader_to_end(
    reader: &mut mpsc::Receiver<Vec<u8>>,
    coalesce: &Arc<CoalesceBuffer>,
) {
    while let Some(chunk) = reader.recv().await {
        coalesce.push(&chunk);
    }
}

enum ReconnectOutcome {
    Cancelled,
    PermanentNone,
    Swapped(Box<dyn PtyTransport>),
}

async fn reconnect_with_backoff(
    domain: &Arc<dyn Domain>,
    profile_label: &str,
    sink: &Arc<dyn EventSink>,
    pane_id: PaneId,
    rows: u16,
    cols: u16,
    cancel: &CancellationToken,
) -> ReconnectOutcome {
    let mut attempt: u32 = 1;
    loop {
        sink.send_user_event(UserEvent::PaneReconnecting {
            pane_id,
            attempt,
            profile_label: profile_label.to_string(),
        });
        match domain.reconnect_one_shot(rows, cols).await {
            Ok(Some(t)) => return ReconnectOutcome::Swapped(t),
            Ok(None) => return ReconnectOutcome::PermanentNone,
            Err(err) => {
                tracing::warn!(?pane_id, attempt, ?err, "reconnect attempt failed");
            }
        }
        let idx = ((attempt as usize) - 1).min(BACKOFF_SCHEDULE_SECS.len() - 1);
        let delay = Duration::from_secs(BACKOFF_SCHEDULE_SECS[idx]);
        tokio::select! {
            biased;
            () = cancel.cancelled() => return ReconnectOutcome::Cancelled,
            () = tokio::time::sleep(delay) => {}
        }
        attempt = attempt.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use async_trait::async_trait;
    use vector_mux::{SpawnCommand, TransportKind};

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

    /// Test-only Domain whose `reconnect_one_shot` returns `Ok(None)`
    /// (LocalDomain shell-death path). The actor exits cleanly on EOF.
    struct PermanentNoneDomain;
    #[async_trait]
    impl Domain for PermanentNoneDomain {
        async fn spawn(&self, _: SpawnCommand) -> Result<Box<dyn PtyTransport>> {
            anyhow::bail!("test");
        }
        fn label(&self) -> String {
            "test".into()
        }
        fn is_alive(&self) -> bool {
            true
        }
        async fn reconnect_one_shot(
            &self,
            _: u16,
            _: u16,
        ) -> Result<Option<Box<dyn PtyTransport>>> {
            Ok(None)
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

    #[test]
    fn permanent_none_domain_constructible() {
        // Sanity check: trait obj coercion holds.
        let _d: Arc<dyn Domain> = Arc::new(PermanentNoneDomain);
    }
}
