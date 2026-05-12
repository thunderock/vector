#![allow(unsafe_code)]

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;

use anyhow::Result;
use tokio::runtime::Builder;
use tokio::sync::mpsc;
use tracing_subscriber::{fmt, EnvFilter};
use vector_app::{app, lpm, pty_actor, UserEvent};
use vector_mux::{LocalDomain, Mux};
use winit::event_loop::{ControlFlow, EventLoop};

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

    let (write_tx, write_rx) = mpsc::channel::<Vec<u8>>(64);
    let (resize_tx, resize_rx) = mpsc::channel::<(u16, u16)>(8);

    let lpm_flag = Arc::new(AtomicBool::new(false));
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
                let local_domain = Arc::new(
                    LocalDomain::new().expect("LocalDomain::new (shell resolution failed)"),
                );
                let mux = Mux::new(local_domain);
                Mux::install(Arc::clone(&mux));

                let window_id = mux.create_window();
                let (_tab_id, pane_id) = match mux.create_tab_async(window_id, None, 24, 80).await {
                    Ok(v) => v,
                    Err(err) => {
                        tracing::error!(?err, "create_tab_async failed; exiting I/O thread");
                        return;
                    }
                };

                let mut router =
                    pty_actor::PtyActorRouter::new(proxy_io.clone(), Arc::clone(&lpm_io));
                if let Some(pane) = mux.pane(pane_id) {
                    if let Some(transport) = pane.take_transport() {
                        router.spawn_pane(pane_id, transport);
                    }
                }

                drop(lpm::spawn_lpm_observer(proxy_io.clone()));
                let proxy_pt = proxy_io.clone();
                drop(vector_mux::spawn_proc_tracker(move |pane_id, label| {
                    let _ = proxy_pt.send_event(UserEvent::PaneTitleChanged { pane_id, label });
                }));

                let router = Arc::new(parking_lot::Mutex::new(router));
                let router_w = Arc::clone(&router);
                let mut write_rx = write_rx;
                drop(tokio::spawn(async move {
                    while let Some(bytes) = write_rx.recv().await {
                        router_w.lock().send_write(pane_id, bytes);
                    }
                }));
                let router_r = Arc::clone(&router);
                let mut resize_rx = resize_rx;
                drop(tokio::spawn(async move {
                    while let Some((rows, cols)) = resize_rx.recv().await {
                        router_r.lock().send_resize(pane_id, rows, cols);
                    }
                }));

                std::future::pending::<()>().await;
            });
        })?;

    let mut application = app::App::new(write_tx, resize_tx, lpm_flag);
    event_loop.run_app(&mut application)?;
    Ok(())
}
