use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tokio::sync::mpsc;
use vector_input::{
    encode_key, wrap_bracketed_paste, AppShortcut, EncodedKey, ModState, MuxCommand, SelectionState,
};
use vector_mux::{compute_layout, Mux, PaneId, Rect, SplitDirection, WindowId as MuxWindowId};
use vector_render::Compositor;
use vector_term::Term;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::Key;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::hyperlink_dispatch;
use crate::input_bridge::InputBridge;
use crate::mux_commands::{WindowFactory, WinitWindowFactory, VECTOR_TABBING_IDENTIFIER};
use crate::overlay::Overlay;
use crate::pty_actor::PtyActorRouter;
use crate::render_host::RenderHost;
use crate::ske::SecureInputGuard;
use crate::toast::{ToastBanner, ToastStack};
use crate::{menu, overlay, UserEvent};

/// MEDIUM-2 (05-REVIEWS.md): snapshot of the active pane's rect in surface pixels.
/// Captured during the per-pane compositor loop and consumed by the chrome pass for
/// search-bar positioning (bottom-Y) and content width.
#[derive(Debug, Clone, Copy)]
pub struct PaneRectPx {
    pub x_px: f32,
    pub y_px: f32,
    pub w_px: f32,
    pub h_px: f32,
}

/// Plan 09-05 — per-pane reconnect bookkeeping for the inline status bar.
#[derive(Debug, Clone)]
pub struct ReconnectingState {
    pub profile_label: String,
    pub attempt: u32,
    pub started_at: Instant,
    pub fade_in_started_at: Option<Instant>,
}

/// Plan 09-05 — pure helper for the input-lock gate. Extracted so tests can
/// assert the gate behavior without standing up a real `App` (the field
/// `reconnecting_panes` is private; the gate is what the test cares about).
#[must_use]
pub fn pane_input_locked<S: std::hash::BuildHasher>(
    reconnecting: &HashMap<PaneId, ReconnectingState, S>,
    pane_id: PaneId,
) -> bool {
    reconnecting.contains_key(&pane_id)
}

/// D-66 active-pane border color (light blue accent).
const BORDER_COLOR_ACTIVE: [f32; 4] = [0.4, 0.6, 1.0, 1.0];
/// Inactive pane: alpha 0 disables the border shader contribution.
const BORDER_COLOR_INACTIVE: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

/// Window size threshold for debouncing `Term::resize` (D-49).
const RESIZE_DEBOUNCE: Duration = Duration::from_millis(50);

/// Per-winit-Window state. Plan 04-04 (D-56): each NSWindowTabbingMode-grouped
/// window holds its own RenderHost + overlay + first-paint gate. Plan 04-06:
/// multi-pane shape with per-pane `compositors` map + `active_pane_id`.
struct AppWindow {
    window: Arc<Window>,
    render_host: Option<RenderHost>,
    /// Plan 05-16 (HIGH-2): chrome pipelines as a PARALLEL field to render_host
    /// so they can be borrowed independently in render_window without a
    /// double-mutable-borrow on RenderHost's surface/compositor.
    chrome_pipelines: Option<crate::chrome::ChromePipelines>,
    overlay: Option<Overlay>,
    overlay_dropped: bool,
    first_paint_ready: bool,
    last_resize_at: Option<Instant>,
    pending_resize: Option<(u16, u16)>,
    /// Plan 04-06: per-pane compositors keyed by Mux PaneId. Populated lazily
    /// on first `UserEvent::PaneOutput` for a pane.
    compositors: HashMap<PaneId, Compositor>,
    /// Plan 04-06: which pane currently owns the active-pane border + filled cursor.
    /// First pane registered becomes active; Cmd-Opt-Arrow flips it.
    active_pane_id: Option<PaneId>,
}

pub struct App {
    /// Plan 04-04: HashMap<winit::WindowId, AppWindow> replaces the single
    /// `Option<Arc<Window>>` so Cmd-T can spawn additional tab-grouped windows.
    windows: HashMap<WindowId, AppWindow>,
    term: Arc<Mutex<Term>>,
    input_bridge: InputBridge,
    mods: ModState,
    cursor_px: PhysicalPosition<f64>,
    lpm_flag: Arc<AtomicBool>,
    /// Plan 04-05: dispatches Cmd-D/Cmd-Shift-D split requests to the I/O thread
    /// which drives `Mux::split_pane_async` + `router.spawn_pane`.
    split_req_tx: Option<mpsc::Sender<(PaneId, SplitDirection)>>,
    /// Plan 04-06: shared handle to the per-pane PtyActorRouter so the App's
    /// per-pane SIGWINCH walk in `flush_pending_resize_if_quiescent` can call
    /// `router.send_resize(pane_id, rows, cols)` for each pane in the layout.
    router: Option<Arc<Mutex<PtyActorRouter>>>,
    /// Plan 04-06: winit::WindowId -> vector_mux::WindowId map. The bootstrap
    /// window records its mapping in `resumed`; Cmd-T windows reuse the
    /// bootstrap mux WindowId (TODO(phase-5): allocate a fresh Mux Window per
    /// Cmd-T NSWindow when handle_new_tab spawns a real Mux Tab+Pane).
    winit_to_mux_window: HashMap<WindowId, MuxWindowId>,
    /// Plan 05-10 M2 — single-banner toast stack (info / action). UI-SPEC §5.4.
    toasts: ToastStack,
    /// Plan 05-10 B1 — last hovered (row, col) + URI for the active pane. Used by
    /// the Cmd-click handler in `WindowEvent::MouseInput` and the Cmd-hover
    /// `NSCursor.pointingHand` swap.
    hover_uri: Option<String>,
    /// Plan 05-10 Task 3 — current ConfigFile applied. Populated by the watcher
    /// thread via UserEvent::ConfigReloaded.
    current_config: Option<std::sync::Arc<vector_config::ConfigFile>>,
    /// Plan 05-09 / POLISH-08 / D-80 — RAII guard for Carbon SKE. Drop on
    /// app exit disables the flag (Pitfall 6).
    ske_guard: SecureInputGuard,
    /// Plan 05-12 (POLISH-05 gap-closure) — live ClipboardRouter consuming
    /// OSC 52 Store events forwarded as UserEvent::ClipboardStore. Policy is
    /// re-resolved from the active profile on every UserEvent::ConfigReloaded.
    clipboard_router: crate::clipboard_router::ClipboardRouter,
    /// Plan 05-15 / POLISH-08 / D-81 — pure-Rust IME state machine backed by
    /// the NSTextInputClient subclass (appkit_impl::VectorInputView). Pitfall 9:
    /// preedit text NEVER reaches the PTY; only commit() writes to write_tx.
    ime: crate::ime::ImeState,
    /// Plan 05-14 gap-closure — SearchBar state (Cmd-F / D-76). Rendered by Plan 05-16's
    /// SearchBarPass. Defaults to closed.
    search_bar: crate::search_bar::SearchBar,
    /// Plan 05-14 gap-closure — ProfilePicker state (Cmd-Shift-P / D-75). Rendered by
    /// Plan 05-16's PickerPass. Defaults to empty entries (populated on ConfigReloaded).
    profile_picker: crate::profile_picker::ProfilePicker,
    /// Plan 05-16 (MEDIUM-2): snapshot of the active pane's rect; updated each frame
    /// during the per-pane compositor loop in render_window. None if no active pane.
    active_pane_rect: Option<PaneRectPx>,
    /// EventLoopProxy for spawning UserEvents from menu items and actors.
    /// Wired by main.rs via `set_proxy`.
    proxy: Option<winit::event_loop::EventLoopProxy<UserEvent>>,
    /// Handle to the I/O-thread tokio runtime so actors can `handle.spawn(...)`
    /// without standing up a second runtime. Wired by main.rs via `set_tokio_handle`.
    tokio_handle: Option<tokio::runtime::Handle>,
    /// Phase 8 / Plan 08-05 — DevTunnelsActor command sender; wired by main.rs.
    devtunnels_cmd_tx: Option<mpsc::Sender<crate::devtunnels_actor::Command>>,
    /// Phase 8 / Plan 08-05 — live MicrosoftAuthDeviceFlowModal.
    microsoft_auth_modal: Option<crate::microsoft_auth_modal::MicrosoftAuthDeviceFlowModal>,
    /// Phase 8 / Plan 08-05 — live DevTunnelsPickerModal.
    devtunnels_modal: Option<crate::devtunnels_modal::DevTunnelsPickerModal>,
    /// Plan 09-05 / PERSIST-01 — per-pane reconnect state. Drives the inline
    /// status bar render hook, the keystroke gate, and the tab badge flip.
    reconnecting_panes: HashMap<PaneId, ReconnectingState>,
    /// Plan 09-05 / PERSIST-01 — per-pane cancellation tokens for tunnel
    /// panes. Cmd-W fires `.cancel()` so the reconnect backoff loop exits
    /// promptly instead of dragging the close UX behind the next backoff slot.
    pane_cancel_tokens: HashMap<PaneId, tokio_util::sync::CancellationToken>,
    /// Plan 09-05 / PERSIST-01 — pane ids that already saw the one-shot
    /// "Input ignored — reconnecting" toast for the CURRENT Reconnecting
    /// span. Cleared on `PaneReconnected` / pane close.
    reconnect_first_keystroke_shown: HashSet<PaneId>,
}

impl App {
    pub fn new(
        write_tx: mpsc::Sender<Vec<u8>>,
        resize_tx: mpsc::Sender<(u16, u16)>,
        lpm_flag: Arc<AtomicBool>,
    ) -> Self {
        // W7: clone write_tx BEFORE it moves into InputBridge so ImeState can
        // own its own sender for the insertText: commit path.
        let ime_write_tx = write_tx.clone();
        Self {
            windows: HashMap::new(),
            term: Arc::new(Mutex::new(Term::new(80, 24, 10_000))),
            input_bridge: InputBridge::new(write_tx, resize_tx),
            mods: ModState::default(),
            cursor_px: PhysicalPosition::new(0.0, 0.0),
            lpm_flag,
            split_req_tx: None,
            router: None,
            winit_to_mux_window: HashMap::new(),
            toasts: ToastStack::default(),
            hover_uri: None,
            current_config: None,
            ske_guard: SecureInputGuard::new(),
            clipboard_router: crate::clipboard_router::ClipboardRouter {
                active_profile: "default".to_owned(),
                policy: None, // resolved on ConfigReloaded; None == prompt-first
            },
            // Plan 05-15: ImeState owns the cloned sender (W7 fix: clone before move).
            ime: crate::ime::ImeState::new(ime_write_tx),
            // Plan 05-14: both default-closed; ProfilePicker gets entries on ConfigReloaded.
            search_bar: crate::search_bar::SearchBar::default(),
            profile_picker: crate::profile_picker::ProfilePicker::new(Vec::new()),
            // Plan 05-16: populated per frame by the compositor loop.
            active_pane_rect: None,
            // Wired by main.rs.
            proxy: None,
            tokio_handle: None,
            devtunnels_cmd_tx: None,
            microsoft_auth_modal: None,
            devtunnels_modal: None,
            // Plan 09-05 / PERSIST-01 — per-pane reconnect bookkeeping.
            reconnecting_panes: HashMap::new(),
            pane_cancel_tokens: HashMap::new(),
            reconnect_first_keystroke_shown: HashSet::new(),
        }
    }

    /// Phase 8 / Plan 08-05 — wire the DevTunnelsActor command sender at startup.
    pub fn set_devtunnels_cmd_tx(&mut self, tx: mpsc::Sender<crate::devtunnels_actor::Command>) {
        self.devtunnels_cmd_tx = Some(tx);
    }

    /// Plan 06-05 / AUTH-01 — supply the EventLoopProxy so menu items and
    /// the auth_actor task can pump UserEvents back to user_event().
    pub fn set_proxy(&mut self, proxy: winit::event_loop::EventLoopProxy<UserEvent>) {
        self.proxy = Some(proxy);
    }

    /// Plan 06-05 / AUTH-01 — supply the tokio runtime handle from the
    /// existing I/O thread so the auth_actor can `handle.spawn(...)`.
    pub fn set_tokio_handle(&mut self, handle: tokio::runtime::Handle) {
        self.tokio_handle = Some(handle);
    }

    /// Plan 05-10 Task 3 — Cmd-C native pasteboard write (CONTEXT Cmd-C
    /// Claude's Discretion: NSPasteboard, NEVER OSC 52).
    #[allow(clippy::unused_self)]
    fn write_pasteboard(&self, s: &str) {
        use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
        use objc2_foundation::NSString;
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();
        let ns_s = NSString::from_str(s);
        unsafe {
            pb.setString_forType(&ns_s, NSPasteboardTypeString);
        }
    }

    /// Plan 04-05: hook the split request channel so Cmd-D / Cmd-Shift-D can
    /// dispatch async splits to the I/O thread.
    pub fn set_split_req_tx(&mut self, tx: mpsc::Sender<(PaneId, SplitDirection)>) {
        self.split_req_tx = Some(tx);
    }

    /// Plan 04-06: hook the per-pane PtyActorRouter so the App's
    /// `flush_pending_resize_if_quiescent` can fan SIGWINCH out per-pane via
    /// `Mux::resize_window` + `router.send_resize`.
    pub fn set_router(&mut self, router: Arc<Mutex<PtyActorRouter>>) {
        self.router = Some(router);
    }

    /// Plan 05-14: set current_config for testing purposes.
    pub fn set_current_config(&mut self, cfg: std::sync::Arc<vector_config::ConfigFile>) {
        self.current_config = Some(cfg);
    }

    /// Plan 05-16 test accessor — set active_pane_rect directly.
    pub fn set_active_pane_rect_for_test(&mut self, rect: Option<PaneRectPx>) {
        self.active_pane_rect = rect;
    }

    /// Plan 05-16 test accessor — read active_pane_rect.
    pub fn active_pane_rect_pub(&self) -> Option<PaneRectPx> {
        self.active_pane_rect
    }

    /// Plan 05-16 test accessor — check if tint would draw given current config.
    pub fn active_profile_tint_rgba_pub(&self) -> Option<[f32; 4]> {
        self.active_profile_tint_rgba()
    }

    /// Plan 05-16 test accessor — borrow current toast banner.
    pub fn toasts_current_pub(&self) -> Option<&crate::toast::ToastBanner> {
        self.toasts.current()
    }

    /// Plan 05-16 test accessor — show a toast banner (for tests that need toast state).
    pub fn show_toast_for_test(&mut self, text: &str) {
        self.toasts.show(crate::toast::ToastBanner::info(text));
    }

    /// Plan 05-14 test accessor — exposes search_bar.open for integration tests.
    pub fn search_bar_open(&self) -> bool {
        self.search_bar.open
    }

    /// Plan 05-14 test accessor — exposes profile_picker.entries.is_empty() for integration tests.
    pub fn profile_picker_entries_empty(&self) -> bool {
        self.profile_picker.entries.is_empty()
    }

    /// Plan 05-14 test accessor — exposes profile_picker.open for integration tests.
    pub fn profile_picker_open(&self) -> bool {
        self.profile_picker.open
    }

    /// Plan 05-14 test accessor — exposes profile_picker.entries.len() for integration tests.
    pub fn profile_picker_entry_count(&self) -> usize {
        self.profile_picker.entries.len()
    }

    /// Plan 05-14 — toggle search bar open/close. D-76: Cmd-F toggles.
    pub fn do_toggle_search(&mut self) {
        if self.search_bar.open {
            let restored = self.search_bar.close();
            if let Some(range) = restored {
                self.input_bridge.selection = SelectionState::Selected(range);
            }
        } else {
            let prior = self.input_bridge.selection.range();
            self.search_bar.open_with(prior);
        }
        self.request_redraw_all();
    }

    /// Plan 05-14 — build picker entries from current_config and open the picker.
    pub fn do_open_profile_picker(&mut self) {
        if let Some(cfg) = self.current_config.clone() {
            let entries: Vec<crate::profile_picker::PickerEntry> = cfg
                .profile
                .iter()
                .map(|(name, block)| crate::profile_picker::PickerEntry {
                    name: name.clone(),
                    kind: block.kind.unwrap_or(vector_config::Kind::Local),
                })
                .collect();
            self.profile_picker = crate::profile_picker::ProfilePicker::new(entries);
        }
        self.profile_picker.open();
        self.request_redraw_all();
    }

    /// Plan 05-14 — reload config from disk (same path as FSEvents watcher). D-69 fallback.
    fn do_reload_config(&mut self) {
        let path = std::env::var_os("HOME")
            .map(|h| std::path::PathBuf::from(h).join(".config/vector/config.toml"));
        if let Some(path) = path {
            match std::fs::read_to_string(&path) {
                Ok(src) => match vector_config::parse(&src) {
                    Ok(cfg) => {
                        self.current_config = Some(std::sync::Arc::new(cfg));
                        // LOW-3: rebuild_switch_profile_submenu is idempotent (proven by
                        // switch_profile_menu_idempotent test). Safe to call from both
                        // UserEvent::ConfigReloaded and AppShortcut::ReloadConfig.
                        if let Some(mtm) = objc2::MainThreadMarker::new() {
                            if let Some(c) = self.current_config.as_ref() {
                                unsafe {
                                    menu::rebuild_switch_profile_submenu(mtm, c);
                                }
                            }
                        }
                        self.toasts.show(ToastBanner::info("config reloaded"));
                        tracing::info!("D-69 Cmd-Shift-R: config reloaded from disk");
                    }
                    Err(e) => {
                        self.toasts
                            .show(ToastBanner::info(format!("config error: {e}")));
                        tracing::warn!(error = %e, "D-69 Cmd-Shift-R: config parse error");
                    }
                },
                Err(e) => {
                    self.toasts
                        .show(ToastBanner::info(format!("config error: {e}")));
                    tracing::warn!(error = %e, "D-69 Cmd-Shift-R: config read error");
                }
            }
            self.request_redraw_all();
        }
    }

    /// Plan 05-14 — dispatch an AppShortcut to the corresponding handler.
    /// Called from both the `EncodedKey::App` keyboard path and the `UserEvent::*` menu path.
    pub(crate) fn handle_app_shortcut(
        &mut self,
        event_loop: &ActiveEventLoop,
        shortcut: AppShortcut,
    ) {
        match shortcut {
            AppShortcut::ToggleSearch => self.do_toggle_search(),
            AppShortcut::OpenProfilePicker => self.do_open_profile_picker(),
            AppShortcut::SpawnNewWindow => {
                // D-82: clean slate — fresh ungrouped NSWindow. v1 simplification: the new
                // window opens without an auto-spawned PTY; the user presses Cmd-T for a shell.
                // PTY spawn for the new window is deferred (see 05-14 SUMMARY deferred section).
                let attrs = WindowAttributes::default()
                    .with_title("Vector")
                    .with_inner_size(winit::dpi::LogicalSize::new(1024.0_f64, 640.0_f64));
                let factory = crate::mux_commands::WinitWindowFactory { event_loop };
                match factory.create_ungrouped(attrs) {
                    Ok(window) => {
                        // MEDIUM-3 (05-REVIEWS.md): this plan OWNS the set_ime_allowed call
                        // site for the SpawnNewWindow branch. Plan 05-15 cannot edit code that
                        // does not yet exist; 05-15 only handles resumed() + handle_new_tab().
                        window.set_ime_allowed(true);
                        // SAFETY: window_event runs on main thread per winit contract.
                        let overlay_inst = unsafe { Some(crate::overlay::install(&window)) };
                        let render_host = match RenderHost::new(&window) {
                            Ok(h) => Some(h),
                            Err(err) => {
                                tracing::error!(?err, "Cmd-N: RenderHost init failed");
                                None
                            }
                        };
                        // HIGH-2: ChromePipelines constructed from the just-created device + format.
                        let chrome_pipelines = render_host.as_ref().map(|h| {
                            crate::chrome::ChromePipelines::new(h.device(), h.surface_format())
                        });
                        let id = window.id();
                        self.windows.insert(
                            id,
                            AppWindow {
                                window,
                                render_host,
                                chrome_pipelines,
                                overlay: overlay_inst,
                                overlay_dropped: false,
                                first_paint_ready: false,
                                last_resize_at: None,
                                pending_resize: None,
                                compositors: std::collections::HashMap::new(),
                                active_pane_id: None,
                            },
                        );
                        tracing::info!(window_id = ?id, "D-82 Cmd-N: ungrouped NSWindow spawned");
                    }
                    Err(err) => {
                        tracing::error!(?err, "Cmd-N: create_ungrouped failed");
                    }
                }
            }
            AppShortcut::ReloadConfig => self.do_reload_config(),
            // Phase 8 / D-11 / Plan 08-05 — Cmd-Shift-T opens DevTunnels picker.
            AppShortcut::OpenDevTunnelsPicker => {
                self.handle_open_devtunnels_picker();
            }
        }
    }

    fn handle_open_devtunnels_picker(&mut self) {
        let Some(mtm) = objc2::MainThreadMarker::new() else {
            tracing::warn!("OpenDevTunnelsPicker: not on main thread");
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            tracing::warn!("OpenDevTunnelsPicker: missing proxy");
            return;
        };
        if let Some(prev) = self.devtunnels_modal.take() {
            prev.dismiss();
        }
        let modal = crate::devtunnels_modal::DevTunnelsPickerModal::show(
            mtm,
            crate::devtunnels_modal::DevTunnelsModalCtx {
                poll_cancel: tokio_util::sync::CancellationToken::new(),
                proxy,
            },
        );
        self.devtunnels_modal = Some(modal);
        if let Some(tx) = &self.devtunnels_cmd_tx {
            let _ = tx.try_send(crate::devtunnels_actor::Command::Load);
        }
    }

    fn primary_window(&self) -> Option<&AppWindow> {
        self.windows.values().next()
    }

    fn cell_from_pixel(&self, px: PhysicalPosition<f64>) -> Option<(u16, u16)> {
        let host = self.primary_window()?.render_host.as_ref()?;
        let (cw, ch) = host.cell_metrics_px()?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px_x = px.x.max(0.0).min(f64::from(u32::MAX)) as u32;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px_y = px.y.max(0.0).min(f64::from(u32::MAX)) as u32;
        let col = u16::try_from(px_x / cw.max(1)).unwrap_or(u16::MAX);
        let row = u16::try_from(px_y / ch.max(1)).unwrap_or(u16::MAX);
        Some((col, row))
    }

    fn request_redraw_all(&self) {
        for w in self.windows.values() {
            w.window.request_redraw();
        }
    }

    fn request_redraw(&self, id: WindowId) {
        if let Some(w) = self.windows.get(&id) {
            w.window.request_redraw();
        }
    }

    /// D-49 debounce + Plan 04-06 per-pane SIGWINCH fanout. Mirrors
    /// `TabWindow::flush_pending_resize_if_quiescent`. When the pending resize is
    /// ≥ 50 ms old, walks `Mux::resize_window` (which redistributes split ratios
    /// and emits per-pane (rows, cols)) and routes each tuple through the
    /// PtyActorRouter so the kernel SIGWINCH reaches each child shell.
    fn flush_pending_resize_if_quiescent(&mut self, id: WindowId) {
        let Some(aw) = self.windows.get_mut(&id) else {
            return;
        };
        let (Some(at), Some((rows, cols))) = (aw.last_resize_at, aw.pending_resize) else {
            return;
        };
        if at.elapsed() < RESIZE_DEBOUNCE {
            return;
        }
        let Some(mux) = Mux::try_get() else {
            return;
        };
        let Some(mux_window_id) = self.winit_to_mux_window.get(&id).copied() else {
            // No Mux mapping yet (pre-bootstrap); clear the pending so we don't spin.
            aw.pending_resize = None;
            aw.last_resize_at = None;
            return;
        };
        let Some(router) = self.router.clone() else {
            return;
        };
        let walk = mux.resize_window(mux_window_id, rows, cols);
        {
            let router = router.lock();
            for (pane_id, prows, pcols) in walk {
                router.send_resize(pane_id, prows, pcols);
            }
        }
        aw.pending_resize = None;
        aw.last_resize_at = None;
    }

    /// Phase 8 / Plan 08-05 — apply Microsoft blue tint to the chrome pipeline
    /// when the pane with `pane_id` is or becomes active, and that pane's
    /// `TransportKind` is `DevTunnel`. UI-SPEC §Color: blue is reserved
    /// exclusively for active DevTunnel panes.
    fn apply_devtunnel_tint_for_pane(&mut self, pane_id: PaneId) {
        let Some(mux) = Mux::try_get() else {
            return;
        };
        let Some(pane) = mux.pane(pane_id) else {
            return;
        };
        if pane.transport_kind() != vector_mux::TransportKind::DevTunnel {
            return;
        }
        for aw in self.windows.values_mut() {
            let Some(host) = aw.render_host.as_ref() else {
                continue;
            };
            let Some(chrome) = aw.chrome_pipelines.as_mut() else {
                continue;
            };
            chrome.tint.set_color(
                host.queue(),
                Some(vector_render::tint_stripe::MICROSOFT_BLUE),
            );
        }
    }

    /// Plan 04-06 (Gap 3): when focus moves to `new_id`, paint the D-66 border
    /// on the new-active pane's compositor, clear the border on the old-active
    /// pane's compositor, and flip `cursor_focused` to filled (active) / hollow
    /// (inactive). Updates `aw.active_pane_id` for the window holding `new_id`.
    fn apply_focus_change(&mut self, new_id: PaneId) {
        for aw in self.windows.values_mut() {
            if !aw.compositors.contains_key(&new_id) {
                continue;
            }
            let Some(host) = aw.render_host.as_ref() else {
                continue;
            };
            let queue = host.queue();
            let old_id = aw.active_pane_id;
            if let Some(old) = old_id {
                if old != new_id {
                    if let Some(comp) = aw.compositors.get_mut(&old) {
                        comp.set_border_color(queue, BORDER_COLOR_INACTIVE);
                        comp.set_cursor_focused(false);
                    }
                }
            }
            if let Some(comp) = aw.compositors.get_mut(&new_id) {
                comp.set_border_color(queue, BORDER_COLOR_ACTIVE);
                comp.set_cursor_focused(true);
            }
            aw.active_pane_id = Some(new_id);
        }
    }

    /// Plan 04-06 (Gap 1): per-pane render loop. Acquires the surface frame
    /// once, then iterates the window's compositors against the layout from
    /// `Mux::compute_layout`. First pane uses `LoadOp::Clear`; subsequent panes
    /// use `LoadOp::Load` so each compositor paints onto the same view.
    /// Plan 05-16: after the pane loop, invoke the chrome pass (tint, search bar,
    /// toast, picker) via `aw.chrome_pipelines` (parallel field to render_host).
    #[allow(
        clippy::too_many_lines,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::manual_let_else
    )]
    fn render_window(&mut self, id: WindowId, sel: Option<((u16, u16), (u16, u16))>) {
        // Plan 05-16: tick the toast stack so Info toasts auto-expire after 5 s.
        self.toasts.tick(std::time::Instant::now());

        let Some(aw) = self.windows.get_mut(&id) else {
            return;
        };
        // Fall back to the legacy single-pane render path when the per-pane map
        // hasn't been populated yet (pre-first-paint).
        if aw.compositors.is_empty() {
            let Some(host) = aw.render_host.as_mut() else {
                return;
            };
            let mut t = self.term.lock();
            if let Err(err) = host.render(&mut t, sel) {
                tracing::warn!(?err, "render failed");
            }
            return;
        }

        // Resolve the Mux tab + layout once per frame, off any aw borrow.
        let mux_window_id = self.winit_to_mux_window.get(&id).copied();
        let mux = Mux::try_get();
        let layout_snapshot = match (mux.as_ref(), mux_window_id) {
            (Some(mux), Some(wid)) => {
                let tab_id = mux.active_tab_id(wid);
                tab_id.and_then(|tid| {
                    mux.with_tab(wid, tid, |tab| {
                        let viewport = Rect {
                            x: 0,
                            y: 0,
                            w: tab.last_cols,
                            h: tab.last_rows,
                        };
                        let layout = compute_layout(&tab.root, viewport);
                        // Sort leaves by PaneId for deterministic render order.
                        let mut leaves = tab.root.leaves();
                        leaves.sort();
                        (leaves, layout)
                    })
                })
            }
            _ => None,
        };
        let Some((leaves, layout)) = layout_snapshot else {
            tracing::warn!(
                mux_wid = ?mux_window_id,
                mux_present = mux.is_some(),
                "render_window: layout_snapshot is None — skipping frame"
            );
            return;
        };

        // Plan 05-16 MEDIUM-2: reset the active-pane snapshot before the pane loop
        // so stale data from a closed pane doesn't persist.
        self.active_pane_rect = None;

        let Some(aw) = self.windows.get_mut(&id) else {
            return;
        };
        // Resolve cell metrics from any existing compositor in the window.
        let Some((cell_w, cell_h)) = aw
            .compositors
            .values()
            .next()
            .map(|c| (c.cell_width_px(), c.cell_height_px()))
        else {
            tracing::warn!("render_window: compositors empty after first-paint — skipping frame");
            return;
        };
        tracing::warn!(
            cell_w,
            cell_h,
            leaves = leaves.len(),
            "render_window: DBG about to render"
        );

        // Per-pane render block — `host` borrow is scoped here so chrome pass can
        // borrow aw.chrome_pipelines independently afterwards. The acquired frame
        // is carried OUT of this block and re-used by the chrome pass below, so the
        // surface is acquired + presented exactly once per render_window call
        // (acquiring twice would advance the swapchain and overwrite the terminal
        // frame with a blank chrome-only texture — see debug session black-screen-render).
        let (frame, frame_width, frame_height) = {
            let Some(host) = aw.render_host.as_mut() else {
                return;
            };
            // Acquire the surface frame once. Skip the frame on Outdated/Lost.
            let frame = match host.acquire_frame() {
                Ok(Some(f)) => f,
                Ok(None) => return,
                Err(err) => {
                    tracing::warn!(?err, "acquire_frame failed");
                    return;
                }
            };
            let width = frame.width;
            let height = frame.height;
            let device = host.device();
            let queue = host.queue();
            let view = &frame.view;
            let default_bg = wgpu::Color {
                r: 0.06,
                g: 0.06,
                b: 0.06,
                a: 1.0,
            };
            let active_pane_id = aw.active_pane_id;
            let mut first = true;
            for pane_id in &leaves {
                let pane_id = *pane_id;
                let Some(rect) = layout.get(&pane_id) else {
                    continue;
                };
                let Some(comp) = aw.compositors.get_mut(&pane_id) else {
                    continue;
                };
                #[allow(clippy::cast_precision_loss)]
                let offset_px = [
                    f32::from(rect.x) * cell_w as f32,
                    f32::from(rect.y) * cell_h as f32,
                ];
                #[allow(clippy::cast_precision_loss)]
                let size_px = [
                    f32::from(rect.w) * cell_w as f32,
                    f32::from(rect.h) * cell_h as f32,
                ];
                comp.set_viewport(queue, offset_px, size_px);
                let is_active = active_pane_id == Some(pane_id);
                comp.set_border_color(
                    queue,
                    if is_active {
                        BORDER_COLOR_ACTIVE
                    } else {
                        BORDER_COLOR_INACTIVE
                    },
                );
                comp.set_cursor_focused(is_active);
                // Plan 05-16 MEDIUM-2: snapshot the active pane's pixel rect for the
                // chrome pass (search-bar positioning + content width).
                if is_active {
                    self.active_pane_rect = Some(PaneRectPx {
                        x_px: offset_px[0],
                        y_px: offset_px[1],
                        w_px: size_px[0],
                        h_px: size_px[1],
                    });
                }
                // Source-of-truth term per pane: the Mux Pane's own term mutex.
                // Selection is forwarded only to the active pane.
                let load_op = if first {
                    wgpu::LoadOp::Clear(default_bg)
                } else {
                    wgpu::LoadOp::Load
                };
                let pane_sel = if is_active { sel } else { None };
                if let Some(pane) = Mux::try_get().and_then(|m| m.pane(pane_id)) {
                    let mut t = pane.term.lock();
                    if let Err(err) = comp.render_into_view(
                        device, queue, view, width, height, &mut t, pane_sel, load_op,
                    ) {
                        tracing::warn!(?pane_id, ?err, "compositor render_into_view failed");
                    }
                } else {
                    // No Mux pane for this id (race): fall back to the shared term
                    // so we still paint something instead of a black hole.
                    let mut t = self.term.lock();
                    if let Err(err) = comp.render_into_view(
                        device, queue, view, width, height, &mut t, pane_sel, load_op,
                    ) {
                        tracing::warn!(
                            ?pane_id,
                            ?err,
                            "compositor render_into_view fallback failed"
                        );
                    }
                }
                first = false;
            }
            // Do NOT present here. The chrome pass below reuses the same surface
            // texture so terminal + chrome land in a single presented frame.
            (frame, width, height)
        };

        // Plan 05-16: chrome pass — AFTER per-pane compositor loop, BEFORE next frame.
        // Snapshot App-level state BEFORE borrowing aw (avoids borrow conflict with
        // self.windows.get_mut which holds a mutable borrow on self.windows).
        #[allow(clippy::cast_precision_loss)]
        let surface_w = frame_width as f32;
        #[allow(clippy::cast_precision_loss)]
        let surface_h = frame_height as f32;
        let active_tint_rgba = self.active_profile_tint_rgba();
        // Plan 09-05 — per-reconnecting-pane snapshot: rect (px), alpha,
        // background RGBA. Built BEFORE the chrome-pipeline borrow so the
        // chrome pass can compose ReconnectPass quads at the top of each
        // reconnecting pane. Composition order: AFTER per-pane Compositor,
        // BEFORE TintStripe/SearchBar/Toast/Picker (UI-SPEC §Spacing).
        let reconnect_draws: Vec<(u32, u32, u32, f32, [f32; 4])> = {
            let now = Instant::now();
            let mut out = Vec::new();
            for pane_id in &leaves {
                let pane_id = *pane_id;
                let Some(rect) = layout.get(&pane_id) else {
                    continue;
                };
                let Some(state) = self.reconnecting_panes.get_mut(&pane_id) else {
                    continue;
                };
                let elapsed_ms = now
                    .saturating_duration_since(state.started_at)
                    .as_millis()
                    .min(u128::from(u32::MAX)) as u32;
                if elapsed_ms < vector_render::RECONNECT_DEBOUNCE_MS {
                    continue; // UI-SPEC §Animation: 250 ms debounce.
                }
                if state.fade_in_started_at.is_none() {
                    state.fade_in_started_at = Some(now);
                }
                let fade_started_at = state.fade_in_started_at.unwrap_or(now);
                let fade_elapsed = now
                    .saturating_duration_since(fade_started_at)
                    .as_millis()
                    .min(u128::from(u32::MAX)) as u32;
                let alpha = (fade_elapsed.min(vector_render::RECONNECT_FADE_IN_MS) as f32)
                    / (vector_render::RECONNECT_FADE_IN_MS as f32);
                #[allow(clippy::cast_precision_loss)]
                let x_px = (f32::from(rect.x) * cell_w as f32) as u32;
                #[allow(clippy::cast_precision_loss)]
                let y_px = (f32::from(rect.y) * cell_h as f32) as u32;
                #[allow(clippy::cast_precision_loss)]
                let w_px = (f32::from(rect.w) * cell_w as f32) as u32;
                // UI-SPEC §Color row 1 — `chrome.surface` at α=0.9.
                // Dark-mode default; light-mode swap is a backlog polish item
                // (Phase 9 ships with the dark token only since `ChromePalette`
                // is not yet plumbed through the App-level render path).
                let bg_rgba = [0.110_f32, 0.110, 0.118, 0.90];
                out.push((x_px, y_px, w_px, alpha, bg_rgba));
            }
            out
        };
        // Search bar snapshot (open flag + rect + no_match)
        let search_bar_draw = if self.search_bar.open {
            self.active_pane_rect.map(|rect| {
                let content_w = rect.w_px; // LOW-2: pane width, not surface width
                let bar_top_y = rect.y_px + rect.h_px - vector_render::SEARCH_BAR_HEIGHT_PX as f32;
                let no_match = self
                    .search_bar
                    .cache
                    .as_ref()
                    .is_some_and(|c| c.matches().is_empty());
                let layout = vector_render::search_bar_layout(content_w as u32, no_match);
                (bar_top_y, content_w, layout.bg_rgba)
            })
        } else {
            None
        };
        // Toast snapshot
        let toast_draw = self.toasts.current().map(|toast| {
            let mode = match toast.mode {
                crate::toast::ToastMode::Info => vector_render::ToastModeKind::Info,
                crate::toast::ToastMode::Action { .. } => vector_render::ToastModeKind::Action,
            };
            let elapsed_ms = u32::try_from(
                toast
                    .shown_at
                    .elapsed()
                    .as_millis()
                    .min(u128::from(u32::MAX)),
            )
            .unwrap_or(u32::MAX);
            let total_visible_ms = match toast.mode {
                crate::toast::ToastMode::Info => 5000,
                crate::toast::ToastMode::Action { .. } => u32::MAX,
            };
            let alpha = vector_render::alpha_at(elapsed_ms, total_visible_ms, false);
            (mode, alpha)
        });
        // Picker snapshot
        let picker_draw = if self.profile_picker.open {
            let longest_label_px = self
                .profile_picker
                .entries
                .iter()
                .map(|e| (e.name.len() as u32).saturating_mul(9))
                .max()
                .unwrap_or(280);
            let visible_rows =
                u32::try_from(self.profile_picker.filtered.len().min(8)).unwrap_or(8);
            let layout = vector_render::picker_layout(
                longest_label_px,
                visible_rows,
                surface_w as u32,
                surface_h as u32,
            );
            Some(layout)
        } else {
            None
        };

        // Now borrow aw for the chrome pass.
        // HIGH-2 borrow disjointness: aw.render_host and aw.chrome_pipelines are
        // separate fields; simultaneous &mut borrows are safe via field projection.
        // The frame acquired above is reused here so we present terminal + chrome
        // as a single swapchain image.
        let Some(aw) = self.windows.get_mut(&id) else {
            // Window vanished between blocks — present what we have and return.
            frame.present();
            return;
        };
        if let (Some(host), Some(chrome)) = (aw.render_host.as_mut(), aw.chrome_pipelines.as_mut())
        {
            let encoder = host
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("chrome-passes"),
                });
            let mut enc = encoder;
            {
                let mut rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("chrome"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &frame.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });

                // 0. Plan 09-05 — per-pane reconnect status bar. Drawn FIRST
                //    (before tint) so chrome surfaces overlay on top of it,
                //    matching UI-SPEC §Spacing composition order (per-pane
                //    Compositor → ReconnectPass → TintStripe → SearchBar →
                //    Toast → Picker). One quad per reconnecting pane; loop
                //    update+draw because ChromeQuadPipeline holds a single
                //    quad uniform.
                for (x_px, y_px, w_px, alpha, bg_rgba) in &reconnect_draws {
                    chrome.reconnect.update(
                        host.queue(),
                        (*x_px, *y_px, *w_px, vector_render::RECONNECT_BAR_HEIGHT_PX),
                        (frame_width, frame_height),
                        *alpha,
                        *bg_rgba,
                    );
                    chrome.reconnect.draw(&mut rpass);
                }

                // 1. Tint stripe (UI-SPEC §5.1, §9.3) — skip if no tint.
                if active_tint_rgba.is_some() {
                    chrome.tint.set_color(host.queue(), active_tint_rgba);
                    chrome
                        .tint
                        .update_quad(host.queue(), frame_width, frame_height);
                    chrome.tint.draw(&mut rpass);
                }

                // 2. OSC-8 hover underline — inside grid attributes; no chrome pass.

                // 3. Search bar — drawn iff open AND active_pane_rect known (MEDIUM-2/LOW-2).
                if let Some((bar_top_y, content_w, bg_rgba)) = search_bar_draw {
                    chrome.search_bar.update_for_pane(
                        host.queue(),
                        bar_top_y,
                        content_w,
                        surface_w,
                        surface_h,
                        bg_rgba,
                    );
                    chrome.search_bar.draw(&mut rpass);
                }

                // 4. Toast — drawn iff current is Some.
                if let Some((mode, alpha)) = toast_draw {
                    let top_y = 0.0_f32; // chrome starts at content top
                    let content_w = surface_w; // toast spans full window width (UI-SPEC §5.4)
                    chrome.toast.update(
                        host.queue(),
                        top_y,
                        content_w,
                        mode,
                        alpha,
                        surface_w,
                        surface_h,
                    );
                    chrome.toast.draw(&mut rpass);
                }

                // 5. Picker — drawn iff open.
                if let Some(layout) = &picker_draw {
                    chrome.picker.draw_scrim(
                        host.queue(),
                        surface_w,
                        surface_h,
                        surface_w,
                        surface_h,
                        &mut rpass,
                    );
                    let bg = [0.11_f32, 0.11, 0.12, 0.96];
                    chrome.picker.draw_modal(
                        host.queue(),
                        layout,
                        bg,
                        surface_w,
                        surface_h,
                        &mut rpass,
                    );
                }
            } // rpass dropped
            host.queue().submit(std::iter::once(enc.finish()));
            frame.present();
        } else {
            // No chrome pipelines available — still present the terminal frame.
            frame.present();
        }
    }

    /// Plan 09-05 / PERSIST-01 — gate keystroke + paste bytes by the active
    /// pane's reconnect state. Returns `true` if bytes were forwarded; `false`
    /// if dropped because the pane is reconnecting. Drives the one-shot
    /// `Input ignored — reconnecting` toast (Info, first dropped keystroke
    /// per Reconnecting span only).
    fn try_send_pty_bytes(&mut self, bytes: Vec<u8>) -> bool {
        let active = Mux::try_get().and_then(|m| m.any_active_pane_id());
        if let Some(pane_id) = active {
            if self.reconnecting_panes.contains_key(&pane_id) {
                // CONTEXT D-03: input locked during reconnect; drop silently.
                if !self.reconnect_first_keystroke_shown.contains(&pane_id) {
                    self.toasts
                        .show(ToastBanner::info("Input ignored \u{2014} reconnecting"));
                    self.reconnect_first_keystroke_shown.insert(pane_id);
                }
                return false;
            }
        }
        self.input_bridge.send_bytes(bytes);
        true
    }

    /// Plan 09-05 / PERSIST-01 — current UI state for a pane (Active vs
    /// Reconnecting). Drives the tab badge swap in `format_tab_title`.
    fn pane_ui_state(&self, pane_id: PaneId) -> vector_mux::PaneUiState {
        if self.reconnecting_panes.contains_key(&pane_id) {
            vector_mux::PaneUiState::Reconnecting
        } else {
            vector_mux::PaneUiState::Active
        }
    }

    /// Plan 09-05 / PERSIST-01 — re-set the window title for `pane_id` using
    /// the current cached process name + cwd + pane UI state. Fires on
    /// `PaneReconnecting` / `PaneReconnected` so the tab badge flips between
    /// `[remote]` and `[reconnecting]` without waiting for the next
    /// `PaneTitleChanged` event.
    fn update_tab_title_for_pane(&mut self, pane_id: PaneId) {
        let Some(mux) = Mux::try_get() else {
            return;
        };
        let Some(pane) = mux.pane(pane_id) else {
            return;
        };
        let process_name = pane.last_proc_name.lock().clone();
        let cwd = pane.cwd.lock().clone();
        let kind = pane.transport_kind();
        let ui_state = self.pane_ui_state(pane_id);
        let title = vector_mux::format_tab_title(&process_name, cwd.as_deref(), kind, ui_state);
        if let Some(aw) = self.primary_window() {
            aw.window.set_title(&format!("Vector — {title}"));
        }
    }

    /// Plan 05-16: return the tint RGBA of the currently active profile, or None if
    /// no tint is configured (UI-SPEC §9.3: tint stripe is a 0-cost skip when None).
    fn active_profile_tint_rgba(&self) -> Option<[f32; 4]> {
        let cfg = self.current_config.as_ref()?;
        let name = &self.clipboard_router.active_profile;
        let block = cfg.profile.get(name)?;
        let tint_hex = block.tint.as_ref()?;
        Self::parse_hex_rgba(tint_hex)
    }

    /// Plan 05-16: parse a 6-char hex color (#RRGGBB) into [r, g, b, 1.0].
    fn parse_hex_rgba(hex: &str) -> Option<[f32; 4]> {
        let s = hex.trim_start_matches('#');
        if s.len() != 6 {
            return None;
        }
        let r = f32::from(u8::from_str_radix(&s[0..2], 16).ok()?) / 255.0;
        let g = f32::from(u8::from_str_radix(&s[2..4], 16).ok()?) / 255.0;
        let b = f32::from(u8::from_str_radix(&s[4..6], 16).ok()?) / 255.0;
        Some([r, g, b, 1.0])
    }

    /// First-paint sync: winit's initial Resized(physical) fires before the lazy
    /// compositor exists, so cell_metrics_px is None and the resize handler
    /// skips queueing pending_resize. Without this sync the tab dims stay at
    /// the create_tab_async initial 80×24 and the grid renders as a top-left
    /// rectangle inside a larger surface. Called from ensure_compositors_for_pane
    /// after the first compositor's cell metrics are known.
    fn sync_tab_dims_to_surface(
        &mut self,
        window_id: WindowId,
        mux_window_id: MuxWindowId,
        tab_cols: u16,
        tab_rows: u16,
        cell_w: u32,
        cell_h: u32,
    ) {
        let Some(mux) = Mux::try_get() else {
            return;
        };
        let (sw, sh) = match self
            .windows
            .get(&window_id)
            .and_then(|aw| aw.render_host.as_ref())
        {
            Some(h) => h.surface_size(),
            None => return,
        };
        if sw == 0 || sh == 0 {
            return;
        }
        let cols = u16::try_from((sw / cell_w.max(1)).max(1)).unwrap_or(u16::MAX);
        let rows = u16::try_from((sh / cell_h.max(1)).max(1)).unwrap_or(u16::MAX);
        if (cols, rows) == (tab_cols, tab_rows) {
            return;
        }
        let walk = mux.resize_window(mux_window_id, rows, cols);
        if let Some(router) = self.router.clone() {
            let router = router.lock();
            for (pane_id, prows, pcols) in &walk {
                router.send_resize(*pane_id, *prows, *pcols);
            }
        }
    }

    /// Resolve cell metrics for a window. If no compositor exists yet, lazily
    /// build the first one sized to the full surface (under `seed_pane_id`) and
    /// return its cell metrics. Returns None on init failure / no render host.
    fn resolve_or_init_cell_metrics(
        &mut self,
        window_id: WindowId,
        seed_pane_id: PaneId,
    ) -> Option<(u32, u32)> {
        let aw = self.windows.get_mut(&window_id)?;
        let host = aw.render_host.as_ref()?;
        if let Some(m) = aw
            .compositors
            .values()
            .next()
            .map(|c| (c.cell_width_px(), c.cell_height_px()))
        {
            return Some(m);
        }
        let (sw, sh) = host.surface_size();
        let viewport_offset = [0.0_f32, 0.0_f32];
        #[allow(clippy::cast_precision_loss)]
        let viewport_size = [sw as f32, sh as f32];
        match host.new_compositor_for_viewport(viewport_offset, viewport_size) {
            Ok(comp) => {
                let cw = comp.cell_width_px();
                let ch = comp.cell_height_px();
                aw.compositors.insert(seed_pane_id, comp);
                Some((cw, ch))
            }
            Err(err) => {
                tracing::error!(?err, "lazy Compositor init failed");
                None
            }
        }
    }

    /// Plan 04-06 (Gap 1 plumbing): lazily create a per-pane Compositor for
    /// every Mux leaf in the tab that holds `seed_pane_id`. No-op if the window
    /// has no render host. Idempotent — only creates compositors for leaves not
    /// already in the map.
    fn ensure_compositors_for_pane(&mut self, window_id: WindowId, seed_pane_id: PaneId) {
        let Some(mux) = Mux::try_get() else {
            return;
        };
        let Some((mux_window_id, tab_id)) = mux.locate_pane(seed_pane_id) else {
            return;
        };
        // Back-fill the winit→mux mapping if `resumed()` lost the race against Mux::install.
        self.winit_to_mux_window
            .entry(window_id)
            .or_insert(mux_window_id);
        // Snapshot leaves + initial tab dims under a single with_tab read lock.
        let snapshot = mux.with_tab(mux_window_id, tab_id, |tab| {
            (tab.root.leaves(), tab.last_cols, tab.last_rows)
        });
        let Some((leaves_initial, tab_cols, tab_rows)) = snapshot else {
            return;
        };
        let first_build = self
            .windows
            .get(&window_id)
            .is_some_and(|aw| aw.compositors.is_empty());
        let Some((cell_w, cell_h)) = self.resolve_or_init_cell_metrics(window_id, seed_pane_id)
        else {
            return;
        };
        if cell_w == 0 || cell_h == 0 {
            return;
        }
        if first_build {
            self.sync_tab_dims_to_surface(
                window_id,
                mux_window_id,
                tab_cols,
                tab_rows,
                cell_w,
                cell_h,
            );
        }
        // Re-snapshot the (possibly updated) layout for per-pane viewport build.
        let snapshot2 = mux.with_tab(mux_window_id, tab_id, |tab| {
            let viewport = Rect {
                x: 0,
                y: 0,
                w: tab.last_cols,
                h: tab.last_rows,
            };
            let layout = compute_layout(&tab.root, viewport);
            (tab.root.leaves(), layout)
        });
        let (leaves, layout) =
            snapshot2.unwrap_or((leaves_initial, std::collections::HashMap::default()));
        let Some(aw) = self.windows.get_mut(&window_id) else {
            return;
        };
        let Some(host) = aw.render_host.as_ref() else {
            return;
        };
        for pane_id in leaves {
            if aw.compositors.contains_key(&pane_id) {
                continue;
            }
            let Some(rect) = layout.get(&pane_id) else {
                continue;
            };
            #[allow(clippy::cast_precision_loss)]
            let offset_px = [
                f32::from(rect.x) * cell_w as f32,
                f32::from(rect.y) * cell_h as f32,
            ];
            #[allow(clippy::cast_precision_loss)]
            let size_px = [
                f32::from(rect.w) * cell_w as f32,
                f32::from(rect.h) * cell_h as f32,
            ];
            match host.new_compositor_for_viewport(offset_px, size_px) {
                Ok(comp) => {
                    aw.compositors.insert(pane_id, comp);
                }
                Err(err) => {
                    tracing::error!(?pane_id, ?err, "per-pane Compositor init failed");
                }
            }
        }
        if aw.active_pane_id.is_none() {
            aw.active_pane_id = Some(seed_pane_id);
        }
    }

    /// Cmd-T: create a new NSWindowTabbingMode-grouped winit Window (D-56)
    /// and register an AppWindow for it. Plan 04-04 only ships the window-
    /// spawn flow; per-pane Mux wiring + a fresh PTY actor land in Plan 04-05.
    fn handle_new_tab(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = WindowAttributes::default()
            .with_title("Vector")
            .with_inner_size(LogicalSize::new(1024.0, 640.0));
        let factory = WinitWindowFactory { event_loop };
        let win = match factory.create_tabbed(attrs, VECTOR_TABBING_IDENTIFIER) {
            Ok(w) => w,
            Err(err) => {
                tracing::error!(?err, "Cmd-T: create_tabbed failed");
                return;
            }
        };
        let id = win.id();
        // SAFETY: winit guarantees user_event/window_event run on the macOS main thread.
        let overlay_inst = unsafe { Some(overlay::install(&win)) };
        let render_host = match RenderHost::new(&win) {
            Ok(h) => Some(h),
            Err(err) => {
                tracing::error!(?err, "Cmd-T: RenderHost init failed");
                None
            }
        };
        // HIGH-2: ChromePipelines parallel to render_host (disjoint borrows in render_window).
        let chrome_pipelines = render_host
            .as_ref()
            .map(|h| crate::chrome::ChromePipelines::new(h.device(), h.surface_format()));
        self.windows.insert(
            id,
            AppWindow {
                window: win,
                render_host,
                chrome_pipelines,
                overlay: overlay_inst,
                overlay_dropped: false,
                first_paint_ready: false,
                last_resize_at: None,
                pending_resize: None,
                compositors: HashMap::new(),
                active_pane_id: None,
            },
        );
        // TODO(phase-5): per-NSWindow mux WindowId allocation when Cmd-T spawns a
        // fresh Mux Tab+Pane. Plan 04-06 bounded scope: reuse the bootstrap mux
        // WindowId so newly-created tab-group NSWindows still route resize.
        if let Some(mux_window_id) =
            Mux::try_get().and_then(|m| m.window_ids_snapshot().first().copied())
        {
            self.winit_to_mux_window.insert(id, mux_window_id);
        }
        // Plan 05-15 / D-81: enable IME delivery on Cmd-T windows too.
        if let Some(aw) = self.windows.get(&id) {
            aw.window.set_ime_allowed(true);
        }
        tracing::info!(window_id = ?id, "Cmd-T: new tab-grouped window created");
    }

    /// Dispatch an `EncodedKey::Mux(...)` command. Plan 04-04 wires Cmd-T
    /// directly (window spawn); other commands route through the Mux
    /// singleton and log their outcome — Plan 04-05 polishes the per-pane
    /// renderer side-effects (border focus flip, viewport redistribute, etc.).
    fn handle_mux_command(&mut self, event_loop: &ActiveEventLoop, cmd: MuxCommand) {
        tracing::info!(?cmd, "mux command dispatch");
        match cmd {
            MuxCommand::NewTab => self.handle_new_tab(event_loop),
            MuxCommand::ClosePane => {
                if let Some(mux) = Mux::try_get() {
                    if let Some(active) = mux.any_active_pane_id() {
                        // Plan 09-05: fire the per-pane cancel token BEFORE
                        // closing so the reconnect backoff loop exits within
                        // ms instead of dragging the UX behind the next slot.
                        if let Some(cancel) = self.pane_cancel_tokens.remove(&active) {
                            cancel.cancel();
                        }
                        self.reconnecting_panes.remove(&active);
                        self.reconnect_first_keystroke_shown.remove(&active);
                        let result = mux.close_pane(active);
                        tracing::info!(?result, "close_pane cascade");
                        if matches!(result, vector_mux::CloseResult::LastWindowClosed) {
                            event_loop.exit();
                        }
                    }
                }
            }
            MuxCommand::SplitHorizontal | MuxCommand::SplitVertical => {
                // Plan 04-05: dispatch the async split to the I/O thread. Per-pane
                // Compositor wiring + visible second-shell rendering lands in the
                // multi-pane render polish (Plan 04-06 gap-closure).
                if let Some(mux) = Mux::try_get() {
                    if let Some(active) = mux.any_active_pane_id() {
                        let dir = if matches!(cmd, MuxCommand::SplitHorizontal) {
                            vector_mux::SplitDirection::Horizontal
                        } else {
                            vector_mux::SplitDirection::Vertical
                        };
                        if let Some(req_tx) = self.split_req_tx.as_ref() {
                            if let Err(err) = req_tx.try_send((active, dir)) {
                                tracing::warn!(?err, "split request channel full/closed");
                            } else {
                                tracing::info!(
                                    pane = ?active,
                                    ?dir,
                                    "split request dispatched to I/O thread"
                                );
                            }
                        }
                    }
                }
            }
            MuxCommand::CycleTabNext | MuxCommand::CycleTabPrev => {
                if let Some(mux) = Mux::try_get() {
                    let dir = if matches!(cmd, MuxCommand::CycleTabNext) {
                        vector_mux::Direction::Right
                    } else {
                        vector_mux::Direction::Left
                    };
                    // Cycle the (single) window's tabs in mux; AppKit owns the
                    // visible tab-bar switch (D-56). Plan 04-05 reconciles when
                    // mux runs multi-tab.
                    for &wid in &mux.window_ids_snapshot() {
                        mux.cycle_tab(wid, dir);
                    }
                }
            }
            MuxCommand::FocusDir(dir) => {
                if let Some(mux) = Mux::try_get() {
                    if let Some(active) = mux.any_active_pane_id() {
                        if let Some(new_id) = mux.focus_direction(active, dir) {
                            tracing::info!(?active, ?new_id, "focus moved");
                            // Plan 04-06 Gap 3: invoke the D-66 border-color setter
                            // on both the old-active and new-active compositors,
                            // and flip cursor_focused for filled vs hollow cursor.
                            self.apply_focus_change(new_id);
                            self.request_redraw_all();
                        } else {
                            tracing::debug!("focus_direction returned no neighbor; absorbed");
                        }
                    }
                }
            }
            MuxCommand::NudgeSplit(dir) => {
                if let Some(mux) = Mux::try_get() {
                    if let Some(active) = mux.any_active_pane_id() {
                        if let Err(err) = mux.nudge_split(active, dir) {
                            tracing::debug!(?err, "nudge_split no-op");
                        }
                    }
                }
            }
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.windows.is_empty() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title("Vector")
            .with_inner_size(LogicalSize::new(1024.0, 640.0));
        // Use the factory so the bootstrap window also joins the tab group on
        // first launch (D-56 + winit#2238 belt-and-braces).
        let factory = WinitWindowFactory { event_loop };
        let window = factory
            .create_tabbed(attrs, VECTOR_TABBING_IDENTIFIER)
            .expect("create bootstrap window");

        // SAFETY: winit guarantees `resumed` runs on the macOS main thread.
        let overlay_inst = unsafe {
            menu::install_main_menu();
            // Plan 09.1-03 (Gap C): install Microsoft sign-in + Dev Tunnels menu
            // items (defined in Plan 08-05 menu.rs but not previously called).
            if let (Some(mtm), Some(proxy)) = (objc2::MainThreadMarker::new(), self.proxy.clone()) {
                menu::install_microsoft_menu_items(mtm, proxy);
            } else {
                tracing::warn!("install_microsoft_menu_items skipped: missing mtm or proxy");
            }
            Some(overlay::install(&window))
        };
        let render_host = match RenderHost::new(&window) {
            Ok(host) => Some(host),
            Err(err) => {
                tracing::error!(?err, "RenderHost init failed");
                None
            }
        };
        // HIGH-2: ChromePipelines parallel to render_host (disjoint borrows in render_window).
        let chrome_pipelines = render_host
            .as_ref()
            .map(|h| crate::chrome::ChromePipelines::new(h.device(), h.surface_format()));
        let id = window.id();
        self.windows.insert(
            id,
            AppWindow {
                window,
                render_host,
                chrome_pipelines,
                overlay: overlay_inst,
                overlay_dropped: false,
                first_paint_ready: false,
                last_resize_at: None,
                pending_resize: None,
                compositors: HashMap::new(),
                active_pane_id: None,
            },
        );
        // Plan 04-06: record the bootstrap mux WindowId mapping. The I/O thread
        // creates exactly one Mux window on startup (main.rs); we adopt its id.
        if let Some(mux_window_id) =
            Mux::try_get().and_then(|m| m.window_ids_snapshot().first().copied())
        {
            self.winit_to_mux_window.insert(id, mux_window_id);
        }
        // Plan 05-15 / D-81: tell the OS to deliver WindowEvent::Ime to this window.
        if let Some(aw) = self.windows.get(&id) {
            aw.window.set_ime_allowed(true);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::PaneOutput { pane_id, bytes } => {
                if bytes.is_empty() {
                    return;
                }
                // Plan 04-06: feed bytes into the pane's own per-pane Term (the
                // source of truth in the Mux). Backward-compat: keep mirroring
                // the ACTIVE pane's bytes into the App's shared `self.term` so
                // existing selection / cell_from_pixel plumbing still works.
                let active = Mux::try_get().and_then(|m| m.any_active_pane_id());
                let is_active = active.is_some_and(|a| a == pane_id);
                if let Some(pane) = Mux::try_get().and_then(|m| m.pane(pane_id)) {
                    let mut t = pane.term.lock();
                    t.feed(&bytes);
                    // D-79 OSC 7 consumer: sync the ring's most-recent cwd into
                    // the pane so spawn_cwd_for / format_tab_title can read it.
                    let latest = t.cwd_ring().back().cloned();
                    drop(t);
                    *pane.cwd.lock() = latest;
                }
                if is_active {
                    let mut t = self.term.lock();
                    t.feed(&bytes);
                }
                // Plan 04-06: lazily ensure per-pane Compositors are built for
                // every leaf in this pane's tab. New panes from Cmd-D land here.
                let window_ids: Vec<WindowId> = self.windows.keys().copied().collect();
                for wid in window_ids {
                    self.ensure_compositors_for_pane(wid, pane_id);
                }
                // First-paint gate (D-51, per-window per Pitfall H): flip on ANY
                // pane's first non-empty drain.
                for aw in self.windows.values_mut() {
                    if !aw.overlay_dropped {
                        aw.overlay = None;
                        aw.overlay_dropped = true;
                    }
                    if !aw.first_paint_ready {
                        aw.first_paint_ready = true;
                        tracing::info!(
                            ?pane_id,
                            "first PTY byte received; per-window first-paint gate open (D-51)"
                        );
                    }
                    // Plan 04-06: redraw on ANY pane's output (not only the
                    // active one), since the per-pane Compositor map paints
                    // every pane independently.
                    aw.window.request_redraw();
                }
            }
            UserEvent::PaneResized {
                pane_id,
                rows,
                cols,
            } => {
                let _ = pane_id;
                {
                    let mut t = self.term.lock();
                    t.resize(cols, rows);
                }
                self.request_redraw_all();
            }
            UserEvent::PaneExited(pane_id) => {
                // Phase 9.1 Gap A: mirror the Cmd-W ClosePane cascade at app.rs:1649-1666.
                // Local PTY EOF (pty_actor.rs:232) → close the pane via Mux and route the cascade.
                // Remote panes never reach here (they enter Reconnecting on EOF — D-03).
                if let Some(cancel) = self.pane_cancel_tokens.remove(&pane_id) {
                    cancel.cancel();
                }
                self.reconnecting_panes.remove(&pane_id);
                self.reconnect_first_keystroke_shown.remove(&pane_id);
                let result = Mux::try_get().map(|mux| mux.close_pane(pane_id));
                tracing::info!(?pane_id, ?result, "PaneExited → close_pane cascade");
                if matches!(result, Some(vector_mux::CloseResult::LastWindowClosed)) {
                    event_loop.exit();
                }
            }
            UserEvent::PaneTitleChanged { pane_id, label } => {
                // D-79 B2: append `: {cwd_stem}` when OSC 7 ring is non-empty.
                // Plan 07-04 / CS-06: append ` [remote]` for non-Local transports.
                // Plan 09-05: append ` [reconnecting]` while pane is in
                // Reconnecting state (overrides `[remote]`).
                let (cwd, kind) = Mux::try_get()
                    .and_then(|m| m.pane(pane_id))
                    .map_or((None, vector_mux::TransportKind::Local), |p| {
                        (p.cwd.lock().clone(), p.transport_kind())
                    });
                let ui_state = self.pane_ui_state(pane_id);
                let title = vector_mux::format_tab_title(&label, cwd.as_deref(), kind, ui_state);
                tracing::info!(?pane_id, %title, "pane title changed");
                if let Some(aw) = self.primary_window() {
                    aw.window.set_title(&format!("Vector — {title}"));
                }
            }
            UserEvent::LpmChanged(enabled) => {
                self.lpm_flag.store(enabled, Ordering::Relaxed);
            }
            // Plan 05-10 Task 3 — chrome / config / hyperlink / Cmd-N arms.
            UserEvent::ConfigReloaded(cfg) => {
                tracing::info!("config reloaded; applying");
                self.current_config = Some(cfg);
                // Plan 05-11 (POLISH-07, MEDIUM-4): rebuild the Switch Profile
                // submenu from the new config via direct OnceLock reference.
                if let Some(mtm) = objc2::MainThreadMarker::new() {
                    if let Some(active) = self.current_config.as_ref() {
                        unsafe {
                            menu::rebuild_switch_profile_submenu(mtm, active);
                        }
                    }
                }
                // Plan 05-12 (POLISH-05 gap-closure): re-resolve the active
                // profile's clipboard_write policy. None == prompt-first.
                if let Some(cfg) = self.current_config.as_ref() {
                    let policy = cfg
                        .profile
                        .get(&self.clipboard_router.active_profile)
                        .and_then(|p| p.clipboard_write);
                    self.clipboard_router.policy = policy;
                }
            }
            UserEvent::ConfigError(msg) => {
                tracing::warn!(error = %msg, "config error");
                self.toasts
                    .show(ToastBanner::info(format!("config error: {msg}")));
            }
            UserEvent::ReloadConfig => {
                // M4 — D-69 Cmd-Shift-R fallback: delegate to the same handler path as
                // AppShortcut::ReloadConfig (which reads from disk and rebuilds the submenu).
                self.handle_app_shortcut(event_loop, AppShortcut::ReloadConfig);
            }
            UserEvent::OpenProfilePicker => {
                // Cmd-Shift-P via menu — delegate to the same handler path.
                self.handle_app_shortcut(event_loop, AppShortcut::OpenProfilePicker);
            }
            UserEvent::ProfileSelected(name) => {
                // Phase 9.1 Gap B: codespace-profile divert-to-GitHub-sign-in
                // path removed. Selecting a codespace profile is now a no-op
                // (the remote-connect path lives in the Dev Tunnels picker).
                tracing::info!(profile = %name, "ProfileSelected");
            }
            UserEvent::ToggleSearch => {
                // Cmd-F via menu — delegate to the same handler path.
                self.handle_app_shortcut(event_loop, AppShortcut::ToggleSearch);
            }
            UserEvent::ToggleSecureKeyboardEntry => {
                // POLISH-08 / D-80 — toggle Carbon SKE; menu state mirrors via
                // a future binding (UI-SPEC §5.8). Pitfall 6 RAII guarantees
                // disable on app exit even if the user leaves it on.
                let now_on = self.ske_guard.toggle();
                tracing::info!(secure_keyboard_entry = now_on, "ToggleSecureKeyboardEntry");
            }
            UserEvent::SpawnNewWindow => {
                // D-82 Cmd-N via menu — delegate to the same handler path.
                // MEDIUM-3: set_ime_allowed(true) is called inside handle_app_shortcut's
                // SpawnNewWindow arm so the UserEvent path also gets IME on the new window.
                self.handle_app_shortcut(event_loop, AppShortcut::SpawnNewWindow);
            }
            UserEvent::HyperlinkClicked { url } => {
                hyperlink_dispatch::open_with_nsworkspace(&url);
            }
            UserEvent::ToastInfo(text) => {
                self.toasts.show(ToastBanner::info(text));
            }
            // Plan 05-12 (POLISH-05 gap-closure): OSC 52 Store from any pane's
            // ForwardingListener -> I/O drain task -> here. Route through the
            // ClipboardRouter policy; WritePasteboard hits NSPasteboard,
            // ShowPrompt raises a toast, DenyRead is a no-op log (D-70).
            UserEvent::ClipboardStore {
                kind_is_selection,
                data,
            } => {
                let fg_proc = "shell"; // v1 default; pane label plumbing is a follow-up.
                let event = crate::clipboard_router::make_store_event(kind_is_selection, data);
                match self.clipboard_router.handle(event, fg_proc) {
                    crate::clipboard_router::ClipboardAction::WritePasteboard(s) => {
                        let bytes = s.len();
                        self.write_pasteboard(&s);
                        tracing::info!(bytes, "OSC 52 -> NSPasteboard (via router)");
                    }
                    crate::clipboard_router::ClipboardAction::ShowPrompt(toast) => {
                        self.toasts.show(toast);
                        self.request_redraw_all();
                    }
                    crate::clipboard_router::ClipboardAction::DenyRead => {
                        tracing::info!("clipboard read denied (D-70)");
                    }
                }
            }
            // Phase 9.1 Gap B: GitHub device-flow + Codespaces picker UserEvent
            // arms removed (AuthSignInRequested, AuthDisplayCode, AuthCompleted,
            // AuthFailed, AuthRequired, SignOut, OpenCodespacesPicker,
            // CodespacesLoaded, CodespacesLoadFailed, CodespaceStateChanged).
            // ───── Phase 8 / Plan 08-05 — DevTunnels + Microsoft sign-in ─────
            UserEvent::MicrosoftDeviceFlowStarted {
                user_code,
                verification_uri,
                expires_in,
                cancel,
            } => {
                tracing::info!(%user_code, "microsoft_device_flow_started");
                if let Some(mtm) = objc2::MainThreadMarker::new() {
                    if let Some(prev) = self.microsoft_auth_modal.take() {
                        prev.dismiss();
                    }
                    let modal = crate::microsoft_auth_modal::MicrosoftAuthDeviceFlowModal::show(
                        mtm,
                        crate::microsoft_auth_modal::MicrosoftAuthModalCtx {
                            user_code,
                            verification_uri,
                            expires_in,
                            cancel,
                        },
                    );
                    self.microsoft_auth_modal = Some(modal);
                }
            }
            UserEvent::MicrosoftSignedIn => {
                if let Some(modal) = self.microsoft_auth_modal.take() {
                    modal.dismiss();
                }
                if let Some(mtm) = objc2::MainThreadMarker::new() {
                    unsafe {
                        menu::rebuild_microsoft_signin_section(mtm, menu::SignInState::SignedIn);
                    }
                }
                // Plan 09.1-04 / D-13 — auto-refresh the picker if open.
                if self.devtunnels_modal.is_some() {
                    if let Some(tx) = &self.devtunnels_cmd_tx {
                        let _ = tx.try_send(crate::devtunnels_actor::Command::Load);
                    }
                }
                self.toasts
                    .show(ToastBanner::info("Signed in to Microsoft."));
                self.request_redraw_all();
            }
            UserEvent::MicrosoftSignInCancelled => {
                if let Some(modal) = self.microsoft_auth_modal.take() {
                    modal.dismiss();
                }
                self.toasts
                    .show(ToastBanner::info("Microsoft sign-in cancelled."));
                self.request_redraw_all();
            }
            UserEvent::MicrosoftSignInFailed(reason) => {
                tracing::warn!(%reason, "microsoft_sign_in_failed");
                if let Some(modal) = self.microsoft_auth_modal.take() {
                    modal.dismiss();
                }
                self.toasts.show(ToastBanner::info(format!(
                    "Microsoft sign-in failed: {reason}"
                )));
                self.request_redraw_all();
            }
            UserEvent::DevTunnelsLoaded(views) => {
                tracing::info!(n = views.len(), "devtunnels_loaded");
                if let Some(modal) = self.devtunnels_modal.as_mut() {
                    if let Some(mtm) = objc2::MainThreadMarker::new() {
                        modal.handle_loaded(mtm, views);
                    }
                }
            }
            UserEvent::DevTunnelsLoadFailed(msg) => {
                tracing::warn!(%msg, "devtunnels_load_failed");
                if let Some(modal) = self.devtunnels_modal.as_mut() {
                    if let Some(mtm) = objc2::MainThreadMarker::new() {
                        modal.handle_load_failed(mtm, msg);
                    }
                }
            }
            UserEvent::DevTunnelsAuthRequired => {
                if let Some(modal) = self.devtunnels_modal.as_mut() {
                    if let Some(mtm) = objc2::MainThreadMarker::new() {
                        modal.handle_auth_required(mtm);
                    }
                }
                self.toasts.show(ToastBanner::info(
                    "Sign in with Microsoft to list Dev Tunnels.",
                ));
                self.request_redraw_all();
            }
            UserEvent::DevTunnelConnectRequested { tunnel_id } => {
                if let Some(tx) = &self.devtunnels_cmd_tx {
                    let _ = tx.try_send(crate::devtunnels_actor::Command::Connect {
                        tunnel_id,
                        rows: 24,
                        cols: 80,
                        window_id: vector_mux::WindowId(0),
                    });
                }
            }
            UserEvent::DevTunnelConnectStarted(tunnel_id) => {
                self.toasts
                    .show(ToastBanner::info(format!("Connecting to {tunnel_id}…")));
                self.request_redraw_all();
            }
            UserEvent::DevTunnelPaneReady {
                window_id,
                tab_id,
                pane_id,
            } => {
                tracing::info!(?window_id, ?tab_id, ?pane_id, "devtunnel_pane_ready");
                self.apply_devtunnel_tint_for_pane(pane_id);
                if let Some(modal) = self.devtunnels_modal.take() {
                    modal.dismiss();
                }
                self.request_redraw_all();
            }
            UserEvent::DevTunnelConnectFailed { tunnel_id, reason } => {
                tracing::warn!(%tunnel_id, %reason, "devtunnel_connect_failed");
                self.toasts.show(ToastBanner::info(format!(
                    "Could not connect to '{tunnel_id}': {reason}."
                )));
                self.request_redraw_all();
            }
            UserEvent::MicrosoftSignInRequested => {
                if let Some(tx) = &self.devtunnels_cmd_tx {
                    let _ = tx.try_send(crate::devtunnels_actor::Command::StartMicrosoftSignIn);
                }
            }
            UserEvent::MicrosoftSignOutRequested => {
                if let Some(tx) = &self.devtunnels_cmd_tx {
                    let _ = tx.try_send(crate::devtunnels_actor::Command::SignOutMicrosoft);
                }
                if let Some(mtm) = objc2::MainThreadMarker::new() {
                    unsafe {
                        menu::rebuild_microsoft_signin_section(mtm, menu::SignInState::SignedOut);
                    }
                }
                self.toasts
                    .show(ToastBanner::info("Signed out of Microsoft."));
                self.request_redraw_all();
            }
            UserEvent::OpenDevTunnelsPickerMenu => {
                self.handle_open_devtunnels_picker();
            }
            // Plan 09-05 / PERSIST-01 — reconnect state machine drives the
            // inline status bar render hook + tab badge flip + input gate.
            UserEvent::PaneReconnecting {
                pane_id,
                attempt,
                profile_label,
            } => {
                let entry =
                    self.reconnecting_panes
                        .entry(pane_id)
                        .or_insert_with(|| ReconnectingState {
                            profile_label: profile_label.clone(),
                            attempt,
                            started_at: Instant::now(),
                            fade_in_started_at: None,
                        });
                entry.attempt = attempt;
                entry.profile_label = profile_label;
                self.update_tab_title_for_pane(pane_id);
                self.request_redraw_all();
            }
            UserEvent::PaneReconnected { pane_id } => {
                self.reconnecting_panes.remove(&pane_id);
                self.reconnect_first_keystroke_shown.remove(&pane_id);
                self.update_tab_title_for_pane(pane_id);
                self.request_redraw_all();
            }
            UserEvent::DevTunnelPaneCancelToken { pane_id, cancel } => {
                self.pane_cancel_tokens.insert(pane_id, cancel);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.windows.remove(&id);
                if self.windows.is_empty() {
                    event_loop.exit();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.mods = ModState::from_winit(modifiers.state());
            }
            WindowEvent::KeyboardInput { event, .. } => {
                // Phase 9.1 Gap B: Codespaces picker keyboard routing removed.
                // Cmd-V: read NSPasteboard + wrap in bracketed paste markers (D-53).
                if event.state == ElementState::Pressed && self.mods.cmd {
                    if let Key::Character(s) = &event.logical_key {
                        if s.as_str() == "v" {
                            let pasted = read_clipboard().unwrap_or_default();
                            let bytes = wrap_bracketed_paste(&pasted);
                            // Plan 09-05: gate on reconnecting state.
                            self.try_send_pty_bytes(bytes);
                            return;
                        }
                        // Plan 05-10 / 05-11 Task 1 — Cmd-C native pasteboard write.
                        // CONTEXT Cmd-C Claude's Discretion: NSPasteboard, NEVER OSC 52.
                        // Selection text built via `selection_to_string` over a
                        // `TermGridAccess` newtype (B1: trait impl lives in vector-app
                        // to avoid a vector-input -> vector-mux -> vector-term cycle).
                        if s.as_str() == "c" && !self.mods.shift {
                            if let Some(range) = self.input_bridge.selection.range() {
                                use vector_input::{selection_to_string, SelectionMode};
                                let text = {
                                    let t = self.term.lock();
                                    selection_to_string(
                                        &range,
                                        &crate::term_grid_access::TermGridAccess(&t),
                                        SelectionMode::Stream,
                                    )
                                };
                                self.write_pasteboard(&text);
                            }
                            return;
                        }
                    }
                }
                match encode_key(&event, self.mods) {
                    Some(EncodedKey::Pty(bytes)) => {
                        // Plan 09-05: gate on reconnecting state — one toast,
                        // then silent drop, for the duration of the span.
                        self.try_send_pty_bytes(bytes);
                        self.request_redraw(id);
                    }
                    Some(EncodedKey::Mux(cmd)) => {
                        self.handle_mux_command(event_loop, cmd);
                        self.request_redraw_all();
                    }
                    // Plan 05-14: App-shortcut dispatch — replaces the Plan 05-11 no-op.
                    Some(EncodedKey::App(shortcut)) => {
                        self.handle_app_shortcut(event_loop, shortcut);
                    }
                    None => {}
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                // Plan 05-10 B1 — Cmd-click on a hovered OSC 8 hyperlink dispatches
                // via NSWorkspace; disallowed schemes show the UI-SPEC §6.1 toast.
                // Falls through to normal selection on no-link / no-Cmd.
                if state == ElementState::Pressed && self.mods.cmd {
                    if let Some(url) = self.hover_uri.clone() {
                        match hyperlink_dispatch::dispatch_cmd_click(&url, &mut self.toasts) {
                            hyperlink_dispatch::DispatchAction::OpenUrl(u) => {
                                hyperlink_dispatch::open_with_nsworkspace(&u);
                            }
                            hyperlink_dispatch::DispatchAction::None => {
                                self.request_redraw(id);
                            }
                        }
                        return;
                    }
                }
                if let Some(cell) = self.cell_from_pixel(self.cursor_px) {
                    match state {
                        ElementState::Pressed => self.input_bridge.selection.mouse_down(cell),
                        ElementState::Released => self.input_bridge.selection.mouse_up(),
                    }
                    self.request_redraw(id);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_px = position;
                // Plan 05-10 B1 — Cmd-hover affordance. Resolve cell → hyperlink_at;
                // if Cmd held and cell has a link, swap to CursorIcon::Pointer
                // (winit's portable mapping to NSCursor.pointingHand on macOS).
                self.hover_uri = self.cell_from_pixel(position).and_then(|(col, row)| {
                    let t = self.term.lock();
                    t.hyperlink_at(usize::from(row), usize::from(col))
                        .map(|(uri, _id)| uri)
                });
                if let Some(win) = self.windows.get(&id).map(|aw| Arc::clone(&aw.window)) {
                    use winit::window::Cursor;
                    if self.mods.cmd && self.hover_uri.is_some() {
                        win.set_cursor(Cursor::Icon(winit::window::CursorIcon::Pointer));
                    } else {
                        win.set_cursor(Cursor::Icon(winit::window::CursorIcon::Default));
                    }
                }
                if matches!(self.input_bridge.selection, SelectionState::Dragging(_)) {
                    if let Some(cell) = self.cell_from_pixel(position) {
                        self.input_bridge.selection.mouse_move(cell);
                        self.request_redraw(id);
                    }
                }
            }
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(_, y),
                ..
            } => {
                #[allow(clippy::cast_possible_truncation)]
                let delta = y.round() as i32;
                if delta != 0 {
                    {
                        let mut t = self.term.lock();
                        t.scroll_display(delta);
                    }
                    self.request_redraw(id);
                }
            }
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::PixelDelta(pos),
                ..
            } => {
                if let Some(host) = self.windows.get(&id).and_then(|aw| aw.render_host.as_ref()) {
                    if let Some((_cw, ch)) = host.cell_metrics_px() {
                        #[allow(clippy::cast_possible_truncation)]
                        let lines = (pos.y / f64::from(ch.max(1))) as i32;
                        if lines != 0 {
                            {
                                let mut t = self.term.lock();
                                t.scroll_display(lines);
                            }
                            self.request_redraw(id);
                        }
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(host) = self
                    .windows
                    .get_mut(&id)
                    .and_then(|aw| aw.render_host.as_mut())
                {
                    #[allow(clippy::cast_possible_truncation)]
                    let dpr = scale_factor as f32;
                    host.clear_atlases();
                    host.set_dpr(dpr);
                }
                self.request_redraw(id);
                tracing::info!(scale_factor, "DPR change; cleared atlases (D-48)");
            }
            WindowEvent::Resized(size) => {
                let Some(aw) = self.windows.get_mut(&id) else {
                    return;
                };
                if let Some(host) = aw.render_host.as_mut() {
                    host.resize(size.width, size.height);
                }
                if let Some(overlay) = aw.overlay.as_mut() {
                    overlay.relayout();
                }
                if let Some(host) = aw.render_host.as_ref() {
                    if let Some((cell_w, cell_h)) = host.cell_metrics_px() {
                        let cols =
                            u16::try_from((size.width / cell_w.max(1)).max(1)).unwrap_or(u16::MAX);
                        let rows =
                            u16::try_from((size.height / cell_h.max(1)).max(1)).unwrap_or(u16::MAX);
                        aw.pending_resize = Some((rows, cols));
                        aw.last_resize_at = Some(Instant::now());
                    }
                }
                aw.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let ready = self.windows.get(&id).is_some_and(|aw| aw.first_paint_ready);
                if !ready {
                    return;
                }
                self.flush_pending_resize_if_quiescent(id);
                let sel = self
                    .input_bridge
                    .selection
                    .range()
                    .map(|r| (r.anchor, r.cursor));
                self.render_window(id, sel);
            }
            // Plan 05-15 / D-81 / Pitfall 9 — IME events from the OS.
            // winit 0.30: Ime::Preedit(text, Option<(cursor_start, cursor_end)>)
            // Pitfall 9: preedit text NEVER writes to PTY; only Commit does.
            WindowEvent::Ime(ime_event) => {
                use winit::event::Ime;
                match ime_event {
                    Ime::Enabled => {
                        tracing::debug!("Ime enabled");
                    }
                    Ime::Preedit(text, cursor_range) => {
                        // empty preedit string signals preedit cancel.
                        let offset = cursor_range.map_or(0, |(start, _)| start);
                        if text.is_empty() {
                            self.ime.clear();
                        } else {
                            self.ime.set_preedit(&text, offset);
                        }
                        self.request_redraw(id);
                    }
                    Ime::Commit(text) => {
                        // commit() writes UTF-8 bytes to the PTY channel.
                        let _ = self.ime.commit(&text);
                        self.request_redraw(id);
                    }
                    Ime::Disabled => {
                        self.ime.clear();
                    }
                }
            }
            _ => {}
        }
    }
}

/// Read the macOS general pasteboard's string content. Must run on the main thread.
fn read_clipboard() -> Option<String> {
    let pb = objc2_app_kit::NSPasteboard::generalPasteboard();
    let ns_str = pb.stringForType(unsafe { objc2_app_kit::NSPasteboardTypeString })?;
    Some(ns_str.to_string())
}
