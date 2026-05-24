//! Shared test fakes for Plan 09-03 integration tests
//! (pty_actor_reconnect.rs + reconnect_byte_integrity.rs).
//!
//! Provides:
//! - `FakeTransport` (dead-on-spawn, or piped with a sender for byte streams)
//! - `ScriptedDomain` (queue of reconnect_one_shot responses)
//! - `TestEventSink` (impls vector_app::EventSink, exposes UnboundedReceiver)

#![allow(dead_code)] // each test file uses a subset

use std::sync::Mutex;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};
use vector_app::pty_actor::EventSink;
use vector_app::UserEvent;
use vector_mux::{Domain, PtyTransport, SpawnCommand, TransportKind};

/// Captures every UserEvent emitted by the actor under test.
pub struct TestEventSink {
    pub tx: mpsc::UnboundedSender<UserEvent>,
}

impl EventSink for TestEventSink {
    fn send_user_event(&self, event: UserEvent) {
        let _ = self.tx.send(event);
    }
}

pub fn test_sink() -> (
    std::sync::Arc<dyn EventSink>,
    mpsc::UnboundedReceiver<UserEvent>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    let sink: std::sync::Arc<dyn EventSink> = std::sync::Arc::new(TestEventSink { tx });
    (sink, rx)
}

/// Fake transport.
///
/// - `dead()` — `take_reader()` yields a receiver whose sender is already
///   dropped (so `recv().await == None` immediately, mimicking EOF).
/// - `piped()` — `take_reader()` yields a live receiver; the test holds the
///   matching sender. `wait()` blocks on a oneshot the test can close to
///   simulate the process exiting after EOF.
pub struct FakeTransport {
    reader: Option<mpsc::Receiver<Vec<u8>>>,
    wait_rx: Option<oneshot::Receiver<()>>,
}

impl FakeTransport {
    /// Dead transport: reader yields None on first recv.
    pub fn dead() -> Self {
        let (tx, rx) = mpsc::channel::<Vec<u8>>(1);
        drop(tx);
        // wait() returns immediately
        let (wtx, wrx) = oneshot::channel();
        drop(wtx);
        Self {
            reader: Some(rx),
            wait_rx: Some(wrx),
        }
    }

    /// Piped transport. Returns (transport, sender). Drop the sender to
    /// signal EOF; `wait()` resolves immediately.
    pub fn piped() -> (Self, mpsc::Sender<Vec<u8>>) {
        let (tx, rx) = mpsc::channel::<Vec<u8>>(64);
        let (wtx, wrx) = oneshot::channel();
        drop(wtx);
        let t = Self {
            reader: Some(rx),
            wait_rx: Some(wrx),
        };
        (t, tx)
    }
}

#[async_trait]
impl PtyTransport for FakeTransport {
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
        if let Some(rx) = self.wait_rx.take() {
            let _ = rx.await;
        }
        Ok(Some(0))
    }
}

/// One scripted response to `reconnect_one_shot`.
pub enum ScriptStep {
    /// Domain returns Err(_); actor logs + backs off.
    Err(String),
    /// Domain returns Ok(None); actor exits cleanly.
    PermanentNone,
    /// Domain returns Ok(Some(new_transport)) built lazily.
    Swap(Box<dyn FnMut() -> Box<dyn PtyTransport> + Send + 'static>),
}

/// Scripted Domain. Pops a `ScriptStep` from the front of the queue on each
/// `reconnect_one_shot` call.
pub struct ScriptedDomain {
    queue: Mutex<std::collections::VecDeque<ScriptStep>>,
    calls: Mutex<u32>,
}

impl ScriptedDomain {
    pub fn new(steps: Vec<ScriptStep>) -> Self {
        Self {
            queue: Mutex::new(steps.into_iter().collect()),
            calls: Mutex::new(0),
        }
    }

    pub fn call_count(&self) -> u32 {
        *self.calls.lock().unwrap()
    }
}

#[async_trait]
impl Domain for ScriptedDomain {
    async fn spawn(&self, _: SpawnCommand) -> Result<Box<dyn PtyTransport>> {
        anyhow::bail!("ScriptedDomain::spawn unused in tests");
    }
    fn label(&self) -> String {
        "scripted".into()
    }
    fn is_alive(&self) -> bool {
        true
    }
    async fn reconnect_one_shot(
        &self,
        _rows: u16,
        _cols: u16,
    ) -> Result<Option<Box<dyn PtyTransport>>> {
        *self.calls.lock().unwrap() += 1;
        let step = self.queue.lock().unwrap().pop_front();
        match step {
            None => anyhow::bail!("ScriptedDomain queue exhausted"),
            Some(ScriptStep::Err(msg)) => Err(anyhow::anyhow!(msg)),
            Some(ScriptStep::PermanentNone) => Ok(None),
            Some(ScriptStep::Swap(mut builder)) => Ok(Some(builder())),
        }
    }
}
