#![allow(unsafe_code)]

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;

use anyhow::Result;
use tokio::runtime::Builder;
use tokio::sync::mpsc;
use tracing_subscriber::{fmt, EnvFilter};
use vector_app::{app, lpm, pty_actor, UserEvent, DEFAULT_CONFIG_TOML};
use vector_mux::{LocalDomain, Mux};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};

#[allow(clippy::too_many_lines)]
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

    // Plan 05-10 Task 3 — config watcher pumped on a dedicated I/O thread.
    // FSEvents → debounced → ConfigReloaded on the main thread via EventLoopProxy.
    let _watcher_thread = spawn_config_watcher_thread(proxy.clone());

    let (write_tx, write_rx) = mpsc::channel::<Vec<u8>>(64);
    let (resize_tx, resize_rx) = mpsc::channel::<(u16, u16)>(8);
    let (split_req_tx, split_req_rx) =
        mpsc::channel::<(vector_mux::PaneId, vector_mux::SplitDirection)>(8);

    let lpm_flag = Arc::new(AtomicBool::new(false));
    let proxy_io = proxy.clone();

    // Plan 04-06: construct the PtyActorRouter on the main thread so we can
    // hand the Arc to both the App (per-pane SIGWINCH fanout) and the I/O
    // thread (per-pane spawn / write / read tasks).
    let router_main = Arc::new(parking_lot::Mutex::new(pty_actor::PtyActorRouter::new(
        proxy.clone(),
        Arc::clone(&lpm_flag),
    )));
    let router_io = Arc::clone(&router_main);

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

                if let Some(pane) = mux.pane(pane_id) {
                    if let Some(transport) = pane.take_transport() {
                        router_io.lock().spawn_pane(pane_id, transport);
                    }
                }

                drop(lpm::spawn_lpm_observer(proxy_io.clone()));
                let proxy_pt = proxy_io.clone();
                drop(vector_mux::spawn_proc_tracker(move |pane_id, label| {
                    let _ = proxy_pt.send_event(UserEvent::PaneTitleChanged { pane_id, label });
                }));

                let router = router_io;
                let router_w = Arc::clone(&router);
                let mut write_rx = write_rx;
                drop(tokio::spawn(async move {
                    while let Some(bytes) = write_rx.recv().await {
                        // Route writes to the currently-active pane. Until per-pane
                        // selection lands, fall back to the bootstrap pane.
                        let target = Mux::try_get()
                            .and_then(|m| m.any_active_pane_id())
                            .unwrap_or(pane_id);
                        router_w.lock().send_write(target, bytes);
                    }
                }));
                let router_r = Arc::clone(&router);
                let mut resize_rx = resize_rx;
                drop(tokio::spawn(async move {
                    while let Some((rows, cols)) = resize_rx.recv().await {
                        // Plan 04-05 fallback path: the per-pane resize fanout
                        // happens inside `TabWindow::flush_pending_resize_if_quiescent`
                        // via `mux.resize_window`. This legacy channel still
                        // delivers SIGWINCH to the bootstrap pane for the
                        // single-pane case.
                        router_r.lock().send_resize(pane_id, rows, cols);
                    }
                }));
                let router_s = Arc::clone(&router);
                let mux_s = Arc::clone(&mux);
                let mut split_req_rx = split_req_rx;
                drop(tokio::spawn(async move {
                    while let Some((parent, dir)) = split_req_rx.recv().await {
                        match mux_s.split_pane_async(parent, dir, None).await {
                            Ok(new_pane_id) => {
                                if let Some(pane) = mux_s.pane(new_pane_id) {
                                    if let Some(transport) = pane.take_transport() {
                                        router_s.lock().spawn_pane(new_pane_id, transport);
                                        tracing::info!(
                                            ?parent,
                                            new = ?new_pane_id,
                                            ?dir,
                                            "split_pane_async + spawn_pane complete"
                                        );
                                    }
                                }
                            }
                            Err(err) => {
                                tracing::warn!(?parent, ?dir, ?err, "split_pane_async failed");
                            }
                        }
                    }
                }));

                std::future::pending::<()>().await;
            });
        })?;

    let mut application = app::App::new(write_tx, resize_tx, lpm_flag);
    application.set_split_req_tx(split_req_tx);
    application.set_router(router_main);
    event_loop.run_app(&mut application)?;
    Ok(())
}

/// Plan 05-10 Task 3 — spawn the config-file watcher on a dedicated thread and
/// forward `ConfigEvent::Dirty` flushes to the main thread as `UserEvent::ConfigReloaded`.
/// Seeds `~/.config/vector/config.toml` from `DEFAULT_CONFIG_TOML` on first run
/// (M4 / D-69: bundled Cmd-Shift-R reload-config keybind).
fn spawn_config_watcher_thread(proxy: EventLoopProxy<UserEvent>) -> Option<thread::JoinHandle<()>> {
    let home = std::env::var_os("HOME")?;
    let config_dir = std::path::PathBuf::from(&home).join(".config").join("vector");
    let config_path = config_dir.join("config.toml");
    let themes_dir = config_dir.join("themes");

    // Seed default config on first launch so the Cmd-Shift-R keybind is live.
    if !config_path.exists() {
        if let Err(e) = std::fs::create_dir_all(&config_dir) {
            tracing::warn!(?e, "could not create ~/.config/vector");
            return None;
        }
        if let Err(e) = std::fs::write(&config_path, DEFAULT_CONFIG_TOML) {
            tracing::warn!(?e, "could not write default config.toml");
        }
    }

    Some(
        thread::Builder::new()
            .name("config-watcher".into())
            .spawn(move || {
                let (tx, rx) = std::sync::mpsc::channel::<vector_config::ConfigEvent>();
                let _debouncer = match vector_config::spawn_watcher(&config_path, &themes_dir, tx) {
                    Ok(d) => d,
                    Err(e) => {
                        tracing::warn!(?e, "spawn_watcher failed; config hot-reload disabled");
                        return;
                    }
                };
                let mut last_good: Option<vector_config::ConfigFile> =
                    std::fs::read_to_string(&config_path)
                        .ok()
                        .and_then(|s| vector_config::parse(&s).ok());
                if let Some(cfg) = &last_good {
                    let _ = proxy.send_event(UserEvent::ConfigReloaded(Arc::new(cfg.clone())));
                }
                for ev in rx {
                    match ev {
                        vector_config::ConfigEvent::Dirty { .. } => {
                            match std::fs::read_to_string(&config_path) {
                                Ok(src) => match vector_config::parse(&src) {
                                    Ok(cfg) => {
                                        last_good = Some(cfg.clone());
                                        let _ = proxy
                                            .send_event(UserEvent::ConfigReloaded(Arc::new(cfg)));
                                    }
                                    Err(e) => {
                                        let _ = proxy.send_event(UserEvent::ConfigError(
                                            e.to_string(),
                                        ));
                                    }
                                },
                                Err(e) => {
                                    let _ =
                                        proxy.send_event(UserEvent::ConfigError(e.to_string()));
                                }
                            }
                        }
                        vector_config::ConfigEvent::Error(msg) => {
                            let _ = proxy.send_event(UserEvent::ConfigError(msg));
                        }
                    }
                }
                drop(last_good);
            })
            .expect("spawn config-watcher thread"),
    )
}
