//! I/O-thread actor: owns LocalDomain + Box<dyn PtyTransport>; reads → coalesce buffer,
//! writes ← main thread, resizes ← main thread. Plan 02-05 actor pattern;
//! Plan 03-04 added the write + resize branches via `biased tokio::select!`;
//! Plan 03-05 routes reads through a shared `CoalesceBuffer` drained by frame_tick.

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc;
use vector_mux::{Domain, LocalDomain, SpawnCommand};
use winit::event_loop::EventLoopProxy;

use crate::frame_tick::CoalesceBuffer;
use crate::UserEvent;

pub async fn io_main(
    proxy: EventLoopProxy<UserEvent>,
    coalesce: Arc<CoalesceBuffer>,
    write_rx: mpsc::Receiver<Vec<u8>>,
    resize_rx: mpsc::Receiver<(u16, u16)>,
) {
    if let Err(err) = run(proxy, coalesce, write_rx, resize_rx).await {
        tracing::error!(?err, "pty actor exited with error");
    }
}

async fn run(
    proxy: EventLoopProxy<UserEvent>,
    coalesce: Arc<CoalesceBuffer>,
    mut write_rx: mpsc::Receiver<Vec<u8>>,
    mut resize_rx: mpsc::Receiver<(u16, u16)>,
) -> Result<()> {
    let domain = LocalDomain::new()?;
    let mut transport = domain
        .spawn(SpawnCommand {
            argv: None,
            cwd: None,
            rows: 24,
            cols: 80,
            env: vec![],
        })
        .await?;
    let mut reader = transport
        .take_reader()
        .expect("take_reader() must succeed on first call");

    loop {
        // Resize takes priority so SIGWINCH isn't starved by chatty PTY output.
        // Plan 02-05 hand-off: biased select! over resize / write / read.
        tokio::select! {
            biased;
            maybe_resize = resize_rx.recv() => {
                let Some((rows, cols)) = maybe_resize else { break; };
                if let Err(err) = transport.resize(rows, cols, 0, 0) {
                    tracing::warn!(?err, "transport.resize failed");
                }
                if proxy.send_event(UserEvent::Resized { rows, cols }).is_err() {
                    tracing::info!("event loop closed; pty actor exiting");
                    break;
                }
            }
            maybe_write = write_rx.recv() => {
                let Some(bytes) = maybe_write else { break; };
                if let Err(err) = transport.write(&bytes).await {
                    tracing::warn!(?err, "transport.write failed");
                }
            }
            maybe_read = reader.recv() => {
                let Some(chunk) = maybe_read else { break; };
                // D-47: append to the coalesce buffer; frame_tick drains every ~8 ms.
                coalesce.push(&chunk);
            }
        }
    }
    let _ = transport.wait().await;
    Ok(())
}
