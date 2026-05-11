#![allow(unsafe_code)]

use std::thread;

use anyhow::Result;
use tokio::runtime::Builder;
use tracing_subscriber::{fmt, EnvFilter};
use winit::event_loop::{ControlFlow, EventLoop};

mod app;
mod input_bridge;
mod menu;
mod overlay;
mod pty_actor;
mod render_host;
#[allow(dead_code)]
mod tick;

#[derive(Debug, Clone)]
pub enum UserEvent {
    Tick(u64),
    PtyOutput(Vec<u8>),
    Resized { rows: u16, cols: u16 },
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

    let _io_thread = thread::Builder::new()
        .name("tokio-io".into())
        .spawn(move || {
            let rt = Builder::new_multi_thread()
                .enable_all()
                .thread_name("tokio-worker")
                .build()
                .expect("build tokio runtime");
            rt.block_on(pty_actor::io_main(proxy, write_rx, resize_rx));
        })?;

    let mut application = app::App::new(write_tx, resize_tx);
    event_loop.run_app(&mut application)?;
    Ok(())
}
