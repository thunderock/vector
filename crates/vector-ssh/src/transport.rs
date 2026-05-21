//! `SshChannelTransport` — adapter from a russh `Channel` to the
//! `vector_mux::PtyTransport` trait.
//!
//! Internals: a single tokio task owns the russh `Channel` and runs a
//! `tokio::select! { biased; ... }` over (1) resize-requests (highest
//! priority — window_change must not starve under chatty output),
//! (2) writes, (3) channel.wait() messages from the server, in that order.
//!
//! `resize` is a sync `mpsc::UnboundedSender::send` — never blocks, never
//! awaits, never panics.
//!
//! An optional subprocess `Child` is held for `kill_on_drop(true)` so any
//! stdio-bridge process is reaped when the transport drops.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};
use vector_mux::{PtyTransport, TransportKind};

use crate::handler::VectorHandler;

pub struct SshChannelTransport {
    reader_rx: Option<mpsc::Receiver<Vec<u8>>>,
    writer_tx: mpsc::Sender<Vec<u8>>,
    resize_tx: mpsc::UnboundedSender<(u16, u16)>,
    exit_rx: Option<oneshot::Receiver<Option<i32>>>,
    /// Channel task join handle — dropping aborts the task.
    _task: tokio::task::JoinHandle<()>,
    /// Optional stdio-bridge subprocess held so `kill_on_drop` reaps it.
    _child: Option<tokio::process::Child>,
    /// Held russh handle so the SSH session outlives `spawn`.
    _handle: Option<russh::client::Handle<VectorHandler>>,
}

impl SshChannelTransport {
    /// Construct a transport from a live russh `Channel` (returned by
    /// `SshClient::open_pty_shell`), the russh `Handle` (kept alive), and an
    /// optional bridge subprocess `Child` (held for `kill_on_drop`).
    pub fn spawn(
        channel: russh::Channel<russh::client::Msg>,
        handle: russh::client::Handle<VectorHandler>,
        child: Option<tokio::process::Child>,
    ) -> Self {
        let (reader_tx, reader_rx) = mpsc::channel::<Vec<u8>>(256);
        let (writer_tx, writer_rx) = mpsc::channel::<Vec<u8>>(64);
        let (resize_tx, resize_rx) = mpsc::unbounded_channel::<(u16, u16)>();
        let (exit_tx, exit_rx) = oneshot::channel::<Option<i32>>();

        let task = tokio::spawn(channel_task(
            channel, reader_tx, writer_rx, resize_rx, exit_tx,
        ));

        Self {
            reader_rx: Some(reader_rx),
            writer_tx,
            resize_tx,
            exit_rx: Some(exit_rx),
            _task: task,
            _child: child,
            _handle: Some(handle),
        }
    }

    /// Test affordance: build a transport whose driver task records resize
    /// requests into a shared `Vec` instead of calling `channel.window_change`.
    /// Writes are accepted but discarded; `wait()` resolves to `None`.
    #[doc(hidden)]
    pub fn for_test_no_channel(
        recorder: std::sync::Arc<std::sync::Mutex<Vec<(u16, u16)>>>,
    ) -> Self {
        let (_reader_tx, reader_rx) = mpsc::channel::<Vec<u8>>(256);
        let (writer_tx, mut writer_rx) = mpsc::channel::<Vec<u8>>(64);
        let (resize_tx, mut resize_rx) = mpsc::unbounded_channel::<(u16, u16)>();
        let (exit_tx, exit_rx) = oneshot::channel::<Option<i32>>();

        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    Some((rows, cols)) = resize_rx.recv() => {
                        recorder.lock().unwrap().push((rows, cols));
                    }
                    Some(_bytes) = writer_rx.recv() => {
                        // Discard.
                    }
                    else => break,
                }
            }
            let _ = exit_tx.send(None);
        });

        Self {
            reader_rx: Some(reader_rx),
            writer_tx,
            resize_tx,
            exit_rx: Some(exit_rx),
            _task: task,
            _child: None,
            _handle: None,
        }
    }
}

async fn channel_task(
    mut channel: russh::Channel<russh::client::Msg>,
    reader_tx: mpsc::Sender<Vec<u8>>,
    mut writer_rx: mpsc::Receiver<Vec<u8>>,
    mut resize_rx: mpsc::UnboundedReceiver<(u16, u16)>,
    exit_tx: oneshot::Sender<Option<i32>>,
) {
    let mut exit_status: Option<i32> = None;
    loop {
        tokio::select! {
            biased;
            // Priority 1: resize — window_change must not starve.
            Some((rows, cols)) = resize_rx.recv() => {
                if let Err(e) = channel
                    .window_change(u32::from(cols), u32::from(rows), 0, 0)
                    .await
                {
                    tracing::warn!(error = %e, "channel.window_change failed");
                }
            }
            // Priority 2: outbound writes from the pane.
            Some(bytes) = writer_rx.recv() => {
                if let Err(e) = channel.data(bytes.as_slice()).await {
                    tracing::warn!(error = %e, "channel.data failed");
                    break;
                }
            }
            // Priority 3: inbound server messages.
            msg = channel.wait() => {
                match msg {
                    Some(russh::ChannelMsg::Data { data }) => {
                        if reader_tx.send(data.to_vec()).await.is_err() {
                            break;
                        }
                    }
                    Some(russh::ChannelMsg::ExtendedData { data, .. }) => {
                        // Fold stderr into the same reader channel — pane
                        // renders both streams identically.
                        if reader_tx.send(data.to_vec()).await.is_err() {
                            break;
                        }
                    }
                    Some(russh::ChannelMsg::ExitStatus { exit_status: code }) => {
                        exit_status = Some(i32::try_from(code).unwrap_or(i32::MAX));
                        // Don't break immediately — let any trailing Data drain.
                    }
                    Some(russh::ChannelMsg::Eof | russh::ChannelMsg::Close) | None => {
                        break;
                    }
                    Some(_) => {}
                }
            }
            else => break,
        }
    }
    let _ = exit_tx.send(exit_status);
}

#[async_trait]
impl PtyTransport for SshChannelTransport {
    #[rustfmt::skip]
    fn kind(&self) -> TransportKind { TransportKind::DevTunnel }

    fn resize(&mut self, rows: u16, cols: u16, _px_w: u16, _px_h: u16) -> Result<()> {
        // Sync send onto an unbounded mpsc — never awaits, never blocks.
        let send_res = self.resize_tx.send((rows, cols));
        send_res.map_err(|e| anyhow!("ssh resize tx: {e}"))
    }

    async fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.writer_tx
            .send(bytes.to_vec())
            .await
            .map_err(|e| anyhow!("ssh write tx: {e}"))
    }

    fn take_reader(&mut self) -> Option<mpsc::Receiver<Vec<u8>>> {
        self.reader_rx.take()
    }

    async fn wait(&mut self) -> Result<Option<i32>> {
        if let Some(rx) = self.exit_rx.take() {
            return Ok(rx.await.ok().flatten());
        }
        Ok(None)
    }
}
