//! I/O-thread actor: owns LocalDomain + Box<dyn PtyTransport>; pumps PTY reader
//! bytes to the main thread via UserEvent::PtyOutput. Plan 02-05 actor pattern.
//! Plan 03-04 will add a write channel + biased select! for input.

use anyhow::Result;
use vector_mux::{Domain, LocalDomain, SpawnCommand};
use winit::event_loop::EventLoopProxy;

use crate::UserEvent;

pub async fn io_main(proxy: EventLoopProxy<UserEvent>) {
    if let Err(err) = run(proxy).await {
        tracing::error!(?err, "pty actor exited with error");
    }
}

async fn run(proxy: EventLoopProxy<UserEvent>) -> Result<()> {
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
    // Single-owner actor: only this task touches `transport`.
    while let Some(chunk) = reader.recv().await {
        if proxy.send_event(UserEvent::PtyOutput(chunk)).is_err() {
            tracing::info!("event loop closed; pty actor exiting");
            break;
        }
    }
    // Drain wait; clean exit per CORE-04.
    let _ = transport.wait().await;
    Ok(())
}
