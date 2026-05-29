#![allow(unsafe_code)]

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;

use anyhow::Result;
use tokio::runtime::Builder;
use tokio::sync::mpsc;
use tracing_subscriber::{fmt, EnvFilter};
use vector_app::{app, devtunnels_actor, lpm, pty_actor, ske, UserEvent, DEFAULT_CONFIG_TOML};
use vector_mux::{LocalDomain, Mux};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use winit::window::WindowId;

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

    // POLISH-08 / D-80 / Pitfall 6: panic hook disables SKE on unwind so a
    // crash mid-secure-input doesn't orphan the process-level flag.
    ske::install_panic_hook();

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
    // Cmd-T new-tab requests: main thread sends the just-created winit WindowId;
    // I/O thread spawns a fresh mux window + pane and acks via NewTabReady.
    let (new_tab_req_tx, new_tab_req_rx) = mpsc::channel::<WindowId>(8);
    // Plan 05-12 (POLISH-05 gap-closure, HIGH-3): clipboard channel — flows
    // ForwardingListener.clipboard_tx (inside every pane's Term, via
    // Term::with_channels) -> here -> drain task -> UserEvent::ClipboardStore
    // -> App.clipboard_router. The sender is moved into Mux::new_with_clipboard
    // so every create_tab_async / split_pane_async wires a live channel.
    let (clip_tx, mut clip_rx) = mpsc::channel::<vector_term::ClipboardEvent>(32);

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

    // Plan 06-05 / AUTH-01: hand the tokio runtime handle back to the main
    // thread via a one-shot channel so the App can spawn the auth_actor
    // task on the same runtime that powers the PTY I/O.
    let (handle_tx, handle_rx) = std::sync::mpsc::sync_channel::<tokio::runtime::Handle>(1);

    // Plan 09-07 — ship DevTunnelsActor cmd_tx from io-thread back to main thread.
    let (dt_tx, dt_rx) =
        std::sync::mpsc::sync_channel::<tokio::sync::mpsc::Sender<devtunnels_actor::Command>>(1);

    let _io_thread = thread::Builder::new()
        .name("tokio-io".into())
        .spawn(move || {
            let rt = Builder::new_multi_thread()
                .enable_all()
                .thread_name("tokio-worker")
                .build()
                .expect("build tokio runtime");
            // Share the runtime handle with the App on the main thread.
            let _ = handle_tx.send(rt.handle().clone());
            rt.block_on(async move {
                let local_domain = Arc::new(
                    LocalDomain::new().expect("LocalDomain::new (shell resolution failed)"),
                );
                // Plan 09-03: keep a `Domain` trait-object handle for the actor's
                // reconnect path. LocalDomain's `reconnect_one_shot` returns
                // `Ok(None)` — i.e. preserves legacy "exit on EOF" behavior.
                let local_domain_dyn: Arc<dyn vector_mux::Domain> = local_domain.clone();
                let mux = Mux::new_with_clipboard(local_domain, clip_tx);
                Mux::install(Arc::clone(&mux));

                // Plan 09-07 — construct DevTunnelsActor so Cmd-Shift-T can reach the picker.
                let dt_api = vector_tunnels::DevTunnelsApi::new();
                let dt_auth = vector_tunnels::auth::GitHubAuth::new(
                    vector_tunnels::auth::GITHUB_DEVTUNNELS_CLIENT_ID,
                );
                let dt_store = vector_tunnels::auth::GitHubTokenStore::for_vector();
                let mut dt_actor = devtunnels_actor::DevTunnelsActor::new(
                    dt_api,
                    Arc::clone(&mux),
                    dt_auth,
                    dt_store,
                    proxy_io.clone(),
                );
                dt_actor.set_router(Arc::clone(&router_io));
                let dt_cmd_tx = dt_actor.spawn(&tokio::runtime::Handle::current());
                let _ = dt_tx.send(dt_cmd_tx);

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
                        router_io.lock().spawn_pane(
                            pane_id,
                            transport,
                            Arc::clone(&local_domain_dyn),
                            String::new(),
                            tokio_util::sync::CancellationToken::new(),
                        );
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
                // Plan 05-12 (POLISH-05 gap-closure): drain ClipboardEvent from
                // every Pane's ForwardingListener and forward Store events to the
                // main thread as UserEvent::ClipboardStore. LoadDenied is logged
                // (D-70: OSC 52 reads are always denied in v1).
                let proxy_clip = proxy_io.clone();
                drop(tokio::spawn(async move {
                    while let Some(ev) = clip_rx.recv().await {
                        match ev {
                            vector_term::ClipboardEvent::Store(kind, data) => {
                                let kind_is_selection =
                                    matches!(kind, vector_term::ClipboardType::Selection);
                                let _ = proxy_clip.send_event(UserEvent::ClipboardStore {
                                    kind_is_selection,
                                    data,
                                });
                            }
                            vector_term::ClipboardEvent::LoadDenied => {
                                tracing::info!("OSC 52 read denied (D-70)");
                            }
                        }
                    }
                }));

                // Cmd-T new-tab consumer: allocate a fresh mux Window, spawn
                // its first pane, and ack via UserEvent::NewTabReady.
                let router_newtab = Arc::clone(&router);
                let mux_newtab = Arc::clone(&mux);
                let local_domain_newtab = Arc::clone(&local_domain_dyn);
                let proxy_newtab = proxy_io.clone();
                let mut new_tab_req_rx = new_tab_req_rx;
                drop(tokio::spawn(async move {
                    while let Some(winit_window_id) = new_tab_req_rx.recv().await {
                        let new_mux_window_id = mux_newtab.create_window();
                        match mux_newtab
                            .create_tab_async(new_mux_window_id, None, 24, 80)
                            .await
                        {
                            Ok((_tab_id, new_pane_id)) => {
                                if let Some(pane) = mux_newtab.pane(new_pane_id) {
                                    if let Some(transport) = pane.take_transport() {
                                        router_newtab.lock().spawn_pane(
                                            new_pane_id,
                                            transport,
                                            Arc::clone(&local_domain_newtab),
                                            String::new(),
                                            tokio_util::sync::CancellationToken::new(),
                                        );
                                    }
                                }
                                let _ = proxy_newtab.send_event(UserEvent::NewTabReady {
                                    winit_window_id,
                                    mux_window_id: new_mux_window_id,
                                    pane_id: new_pane_id,
                                });
                                tracing::info!(
                                    ?winit_window_id,
                                    ?new_mux_window_id,
                                    ?new_pane_id,
                                    "Cmd-T: new tab pane spawned"
                                );
                            }
                            Err(err) => {
                                tracing::error!(
                                    ?winit_window_id,
                                    ?err,
                                    "Cmd-T: create_tab_async failed"
                                );
                            }
                        }
                    }
                }));

                let router_s = Arc::clone(&router);
                let mux_s = Arc::clone(&mux);
                let local_domain_split = Arc::clone(&local_domain_dyn);
                let mut split_req_rx = split_req_rx;
                drop(tokio::spawn(async move {
                    while let Some((parent, dir)) = split_req_rx.recv().await {
                        match mux_s.split_pane_async(parent, dir, None).await {
                            Ok(new_pane_id) => {
                                if let Some(pane) = mux_s.pane(new_pane_id) {
                                    if let Some(transport) = pane.take_transport() {
                                        router_s.lock().spawn_pane(
                                            new_pane_id,
                                            transport,
                                            Arc::clone(&local_domain_split),
                                            String::new(),
                                            tokio_util::sync::CancellationToken::new(),
                                        );
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
    application.set_new_tab_req_tx(new_tab_req_tx);
    application.set_router(router_main);
    // Plan 06-05 / AUTH-01: wire the proxy + tokio handle so the device-flow
    // path (menu click -> AuthSignInRequested -> auth_actor.spawn) can find
    // both halves of its plumbing.
    application.set_proxy(proxy.clone());
    if let Ok(handle) = handle_rx.recv() {
        application.set_tokio_handle(handle);
    } else {
        tracing::warn!("tokio handle not received from I/O thread; auth disabled");
    }
    // Plan 09-07 — hand the actor's cmd_tx to App so Cmd-Shift-T can drive it.
    if let Ok(cmd_tx) = dt_rx.recv() {
        application.set_devtunnels_cmd_tx(cmd_tx);
    } else {
        tracing::warn!("devtunnels cmd_tx not received from I/O thread; Cmd-Shift-T disabled");
    }
    event_loop.run_app(&mut application)?;
    Ok(())
}

/// Plan 05-10 Task 3 — spawn the config-file watcher on a dedicated thread and
/// forward `ConfigEvent::Dirty` flushes to the main thread as `UserEvent::ConfigReloaded`.
/// Seeds `~/.config/vector/config.toml` from `DEFAULT_CONFIG_TOML` on first run
/// (M4 / D-69: bundled Cmd-Shift-R reload-config keybind).
fn spawn_config_watcher_thread(proxy: EventLoopProxy<UserEvent>) -> Option<thread::JoinHandle<()>> {
    let home = std::env::var_os("HOME")?;
    let config_dir = std::path::PathBuf::from(&home)
        .join(".config")
        .join("vector");
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
                                        let _ =
                                            proxy.send_event(UserEvent::ConfigError(e.to_string()));
                                    }
                                },
                                Err(e) => {
                                    let _ = proxy.send_event(UserEvent::ConfigError(e.to_string()));
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
