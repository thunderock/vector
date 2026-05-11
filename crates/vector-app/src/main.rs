#![allow(unsafe_code)]

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;

use anyhow::Result;
use tokio::runtime::Builder;
use tracing_subscriber::{fmt, EnvFilter};
use winit::event_loop::{ControlFlow, EventLoop};

use crate::frame_tick::{CoalesceBuffer, COALESCE_THRESHOLD};

mod app;
mod frame_tick;
mod input_bridge;
mod lpm;
mod menu;
mod overlay;
mod pty_actor;
mod render_host;

#[derive(Debug, Clone)]
pub enum UserEvent {
    PtyOutput(Vec<u8>),
    Resized { rows: u16, cols: u16 },
    LpmChanged(bool),
}

fn main() -> Result<()> {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        sha = env!("VECTOR_BUILD_SHA"),
        "vector starting"
    );

    let event_loop: EventLoop<UserEvent> = EventLoop::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let proxy = event_loop.create_proxy();

    let (write_tx, write_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
    let (resize_tx, resize_rx) = tokio::sync::mpsc::channel::<(u16, u16)>(8);

    let coalesce = Arc::new(CoalesceBuffer::new(COALESCE_THRESHOLD));
    let lpm_flag = Arc::new(AtomicBool::new(false));

    let coalesce_io = Arc::clone(&coalesce);
    let proxy_io = proxy.clone();
    let lpm_io = Arc::clone(&lpm_flag);

    let _io_thread = thread::Builder::new()
        .name("tokio-io".into())
        .spawn(move || {
            let rt = Builder::new_multi_thread()
                .enable_all()
                .thread_name("tokio-worker")
                .build()
                .expect("build tokio runtime");
            rt.block_on(async move {
                // Frame-tick + LPM observer live on the tokio runtime alongside the PTY actor.
                drop(tokio::spawn(frame_tick::frame_tick_loop(
                    Arc::clone(&coalesce_io),
                    proxy_io.clone(),
                    Arc::clone(&lpm_io),
                )));
                drop(lpm::spawn_lpm_observer(proxy_io.clone()));
                pty_actor::io_main(proxy_io, coalesce_io, write_rx, resize_rx).await;
            });
        })?;

    let mut application = app::App::new(write_tx, resize_tx, lpm_flag);
    event_loop.run_app(&mut application)?;
    Ok(())
}
