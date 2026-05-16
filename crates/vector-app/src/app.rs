use std::collections::HashMap;
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
use winit::keyboard::{Key, NamedKey};
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
    /// Plan 06-05 / AUTH-01 — EventLoopProxy for spawning UserEvents from
    /// menu items, the auth_actor tokio task, and the modal Cancel button.
    /// Wired by main.rs via `set_proxy`.
    proxy: Option<winit::event_loop::EventLoopProxy<UserEvent>>,
    /// Plan 06-05 / AUTH-01 — Handle to the I/O-thread tokio runtime so we
    /// can spawn the auth_actor task without standing up a second runtime.
    /// Wired by main.rs via `set_tokio_handle`.
    tokio_handle: Option<tokio::runtime::Handle>,
    /// Plan 06-05 / AUTH-01 — live AuthDeviceFlowModal NSPanel (Some while
    /// the modal is on screen). The 1 Hz frame tick calls `tick()` on it.
    auth_modal: Option<crate::auth_modal::AuthDeviceFlowModal>,
    /// Plan 06-05 / AUTH-01 — set on AuthSignInRequested when we spawn the
    /// device-flow task; cleared on AuthCompleted/AuthFailed. Lets the modal
    /// share the same cancel flag as the tokio task.
    pending_auth_cancellation: Option<crate::auth_actor::AuthCancellation>,
    /// Plan 06-06 / CS-01..03 — live CodespacesPickerModal NSPanel.
    codespaces_modal: Option<crate::codespaces_modal::CodespacesPickerModal>,
    /// Plan 06-06 — Octocrab-backed client built lazily on first picker open
    /// from the Keychain access token. Reused across fetch / start / poll.
    codespaces_client: Option<std::sync::Arc<vector_codespaces::CodespacesClient>>,
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
            // Plan 06-05 / AUTH-01: wired by main.rs.
            proxy: None,
            tokio_handle: None,
            auth_modal: None,
            pending_auth_cancellation: None,
            codespaces_modal: None,
            codespaces_client: None,
        }
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

    // ---- Phase 6 / AUTH-01 — device-flow handlers ------------------------

    /// Spawn the auth_actor tokio task. Idempotent-ish: if a flow is already
    /// in progress (pending_auth_cancellation set), the new request replaces
    /// the old one (the previous flag is dropped — its task will see the
    /// stale flag value, but since we abandon the modal it doesn't matter).
    fn handle_auth_sign_in_requested(&mut self) {
        let (Some(proxy), Some(handle)) = (self.proxy.clone(), self.tokio_handle.clone()) else {
            tracing::warn!("AuthSignInRequested: proxy or tokio_handle missing");
            return;
        };
        let cancel = crate::auth_actor::spawn_device_flow(&handle, proxy);
        self.pending_auth_cancellation = Some(cancel);
        tracing::info!("auth_actor spawned (device flow in progress)");
    }

    /// Construct + show the AuthDeviceFlowModal. The cancellation flag from
    /// the spawn path is shared with the modal so the Cancel button can
    /// signal the actor task.
    fn handle_auth_display_code(
        &mut self,
        user_code: String,
        verification_uri: String,
        expires_at: std::time::SystemTime,
    ) {
        let Some(mtm) = objc2::MainThreadMarker::new() else {
            tracing::warn!("AuthDisplayCode: not on main thread");
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            tracing::warn!("AuthDisplayCode: proxy missing");
            return;
        };
        let cancellation = self.pending_auth_cancellation.clone().unwrap_or_default();
        if self.auth_modal.is_some() {
            tracing::debug!("AuthDisplayCode: replacing existing modal");
            if let Some(prev) = self.auth_modal.take() {
                prev.dismiss(mtm);
            }
        }
        let modal = crate::auth_modal::AuthDeviceFlowModal::show(
            mtm,
            user_code,
            verification_uri,
            expires_at,
            cancellation,
            proxy,
        );
        self.auth_modal = Some(modal);
    }

    fn handle_auth_completed(&mut self, user_login: &str) {
        if let Some(mtm) = objc2::MainThreadMarker::new() {
            if let Some(modal) = self.auth_modal.take() {
                modal.dismiss(mtm);
            }
            unsafe { menu::rebuild_auth_menu_section(mtm, true, Some(user_login)) };
        }
        self.pending_auth_cancellation = None;
        self.toasts
            .show(ToastBanner::info(format!("signed in as @{user_login}")));
        self.request_redraw_all();
        tracing::info!(user_login, "auth_completed");
    }

    fn handle_auth_failed(&mut self, reason: &str) {
        if let Some(mtm) = objc2::MainThreadMarker::new() {
            if let Some(modal) = self.auth_modal.take() {
                modal.dismiss(mtm);
            }
        }
        self.pending_auth_cancellation = None;
        // UI-SPEC §6.1 toast copy mapping.
        let toast_text = match reason {
            "cancelled" => "sign-in cancelled".to_string(),
            "expired" => "sign-in code expired — try again".to_string(),
            other => format!("sign-in failed: {other}"),
        };
        self.toasts.show(ToastBanner::info(toast_text));
        self.request_redraw_all();
        tracing::info!(reason, "auth_failed");
    }

    // ---- Phase 6 / CS-01..03 — picker handlers --------------------------

    fn handle_open_codespaces_picker(&mut self) {
        // Build client lazily from Keychain.
        if self.codespaces_client.is_none() {
            let Some(c) = crate::codespaces_actor::build_client_from_keychain() else {
                tracing::info!("OpenCodespacesPicker: no token — routing to AuthRequired");
                if let Some(proxy) = &self.proxy {
                    let _ = proxy.send_event(UserEvent::AuthRequired);
                }
                return;
            };
            self.codespaces_client = Some(c);
        }
        let Some(mtm) = objc2::MainThreadMarker::new() else {
            tracing::warn!("OpenCodespacesPicker: not on main thread");
            return;
        };
        let Some(proxy) = self.proxy.clone() else {
            return;
        };
        let Some(handle) = self.tokio_handle.clone() else {
            return;
        };
        let Some(client) = self.codespaces_client.clone() else {
            return;
        };
        // Dismiss prior modal (if any) before showing a fresh one.
        if let Some(prev) = self.codespaces_modal.take() {
            prev.dismiss();
        }
        let modal = crate::codespaces_modal::CodespacesPickerModal::show(mtm);
        self.codespaces_modal = Some(modal);
        crate::codespaces_actor::spawn_fetch_codespaces(&handle, proxy, client);
    }

    fn handle_codespaces_loaded(
        &mut self,
        list: &std::sync::Arc<Vec<vector_codespaces::Codespace>>,
    ) {
        let Some(mtm) = objc2::MainThreadMarker::new() else {
            return;
        };
        let Some(modal) = self.codespaces_modal.as_mut() else {
            return;
        };
        // Spawn poll tasks for any row in the Starting family so live state
        // ticks render in the picker.
        let cancel_token = modal.poll_cancel.clone();
        modal.handle_loaded(mtm, list.clone());
        if let (Some(handle), Some(client), Some(proxy)) = (
            self.tokio_handle.clone(),
            self.codespaces_client.clone(),
            self.proxy.clone(),
        ) {
            for cs in list.iter() {
                if crate::relative_time::state_label(cs.state) == "Starting" {
                    crate::codespaces_actor::spawn_poll_row(
                        &handle,
                        proxy.clone(),
                        client.clone(),
                        &cs.name,
                        cancel_token.clone(),
                    );
                }
            }
        }
    }

    /// Connect to currently selected codespace (Phase-6 stub: emits placeholder
    /// toast per UI-SPEC §6.1; Phase 7 replaces with real SSH transport).
    fn codespaces_connect_selected(&mut self) {
        let Some(modal) = self.codespaces_modal.as_ref() else {
            return;
        };
        if modal.selected().is_some() {
            self.toasts.show(ToastBanner::info(
                "codespace ssh transport not yet wired — phase 7",
            ));
            self.request_redraw_all();
        }
    }

    /// Start the currently selected Shutdown-family codespace (CS-02).
    fn codespaces_start_selected(&mut self) {
        let (Some(modal), Some(handle), Some(client), Some(proxy)) = (
            self.codespaces_modal.as_ref(),
            self.tokio_handle.clone(),
            self.codespaces_client.clone(),
            self.proxy.clone(),
        ) else {
            return;
        };
        let Some(cs) = modal.selected() else {
            return;
        };
        let cancel = modal.poll_cancel.clone();
        crate::codespaces_actor::spawn_start_then_poll(
            &handle,
            proxy,
            client,
            cs.name.clone(),
            cancel,
        );
    }

    /// Save the currently selected codespace as a profile (CS-03).
    fn codespaces_save_selected(&mut self) {
        let Some(modal) = self.codespaces_modal.as_ref() else {
            return;
        };
        let Some(cs) = modal.selected() else {
            return;
        };
        let existing: Vec<String> = self
            .current_config
            .as_ref()
            .map(|c| c.profile.keys().cloned().collect())
            .unwrap_or_default();
        let existing_refs: Vec<&str> = existing.iter().map(String::as_str).collect();
        let suggested =
            vector_config::derive_profile_name(&cs.repository.full_name, &existing_refs);
        let path = crate::codespaces_modal::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match vector_config::append_codespace_profile(&path, &suggested, &cs.name, "#7a3aaf") {
            Ok(final_name) => {
                self.toasts.show(ToastBanner::info(format!(
                    "profile saved as \"{final_name}\""
                )));
            }
            Err(e) => {
                tracing::warn!(error = %e, "append_codespace_profile failed");
                self.toasts
                    .show(ToastBanner::info(format!("could not save profile — {e}")));
            }
        }
        self.request_redraw_all();
    }

    /// Dismiss the codespaces picker if open. Used by Esc and lifecycle paths.
    fn codespaces_picker_dismiss(&mut self) {
        if let Some(modal) = self.codespaces_modal.take() {
            modal.dismiss();
        }
    }

    /// Returns true if the codespaces picker is the AppKit key window.
    fn codespaces_picker_is_key(&self) -> bool {
        self.codespaces_modal
            .as_ref()
            .is_some_and(crate::codespaces_modal::CodespacesPickerModal::is_key_window)
    }

    /// 1 Hz tick into the auth modal countdown. Called from the existing
    /// frame-tick loop on every WindowEvent::RedrawRequested. If the modal
    /// has expired, emit `AuthFailed { reason: "expired" }` so the regular
    /// failure path handles dismiss + toast.
    pub fn tick_auth_modal(&mut self) {
        if let Some(modal) = self.auth_modal.as_ref() {
            if modal.tick() {
                if let Some(proxy) = &self.proxy {
                    let _ = proxy.send_event(UserEvent::AuthFailed {
                        reason: "expired".into(),
                    });
                }
            }
        }
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
            AppShortcut::OpenCodespacesPicker => {
                if let Some(proxy) = &self.proxy {
                    let _ = proxy.send_event(UserEvent::OpenCodespacesPicker);
                }
            }
            AppShortcut::SignInWithGitHub => {
                self.handle_auth_sign_in_requested();
            }
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
        // Plan 06-05 / AUTH-01: 1 Hz auth modal countdown (UI-SPEC §5.1). The
        // tick runs every render frame but only mutates the NSTextField once
        // per second's worth of difference, so the call is cheap.
        self.tick_auth_modal();

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
            return;
        };

        // Per-pane render block — `host` borrow is scoped here so chrome pass can
        // borrow aw.chrome_pipelines independently afterwards.
        let (frame_width, frame_height, ()) = {
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
            frame.present();
            // Return surface dimensions for the chrome pass encoder (no frame needed).
            (width, height, ())
        };
        // frame_view lifetime ended with frame.present() above.

        // Plan 05-16: chrome pass — AFTER per-pane compositor loop, BEFORE next frame.
        // Snapshot App-level state BEFORE borrowing aw (avoids borrow conflict with
        // self.windows.get_mut which holds a mutable borrow on self.windows).
        #[allow(clippy::cast_precision_loss)]
        let surface_w = frame_width as f32;
        #[allow(clippy::cast_precision_loss)]
        let surface_h = frame_height as f32;
        let active_tint_rgba = self.active_profile_tint_rgba();
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
        let Some(aw) = self.windows.get_mut(&id) else {
            return;
        };
        if let (Some(host), Some(chrome)) = (aw.render_host.as_mut(), aw.chrome_pipelines.as_mut())
        {
            let frame = match host.acquire_frame() {
                Ok(Some(f)) => f,
                Ok(None) | Err(_) => return, // skip chrome frame if surface unavailable
            };
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
        // Snapshot leaves + viewport + layout under a single with_tab read lock.
        let snapshot = mux.with_tab(mux_window_id, tab_id, |tab| {
            let viewport = Rect {
                x: 0,
                y: 0,
                w: tab.last_cols,
                h: tab.last_rows,
            };
            let layout = compute_layout(&tab.root, viewport);
            (tab.root.leaves(), layout)
        });
        let Some((leaves, layout)) = snapshot else {
            return;
        };
        let Some(aw) = self.windows.get_mut(&window_id) else {
            return;
        };
        let Some(host) = aw.render_host.as_ref() else {
            return;
        };
        // For the very first compositor we don't know cell metrics yet; build
        // it sized to the full surface and read its metrics back. Subsequent
        // panes use those metrics to derive their viewport pixel rects.
        let (cell_w, cell_h) = if let Some(m) = aw
            .compositors
            .values()
            .next()
            .map(|c| (c.cell_width_px(), c.cell_height_px()))
        {
            m
        } else {
            let (sw, sh) = host.surface_size();
            let viewport_offset = [0.0_f32, 0.0_f32];
            #[allow(clippy::cast_precision_loss)]
            let viewport_size = [sw as f32, sh as f32];
            match host.new_compositor_for_viewport(viewport_offset, viewport_size) {
                Ok(comp) => {
                    let cw = comp.cell_width_px();
                    let ch = comp.cell_height_px();
                    aw.compositors.insert(seed_pane_id, comp);
                    (cw, ch)
                }
                Err(err) => {
                    tracing::error!(?err, "lazy Compositor init failed");
                    return;
                }
            }
        };
        if cell_w == 0 || cell_h == 0 {
            return;
        }
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
            // Plan 06-05 / AUTH-01: insert Sign in / Sign out / Codespaces…
            // above the existing Vector-menu items. Requires proxy (set by
            // main.rs); skip silently if absent (e.g. integration tests).
            if let (Some(mtm), Some(proxy)) = (objc2::MainThreadMarker::new(), self.proxy.clone()) {
                menu::install_auth_menu_items(mtm, proxy);
                // First-launch: reflect Keychain state so the right item is
                // visible. @login is not fetched here (would block the UI
                // thread); the next AuthCompleted will fill it in.
                let token_present = vector_codespaces::TokenStore::new().load_access().is_some();
                menu::rebuild_auth_menu_section(mtm, token_present, None);
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
                tracing::info!(?pane_id, "pane exited (Plan 04-05 will render sentinel)");
            }
            UserEvent::PaneTitleChanged { pane_id, label } => {
                // D-79 B2: append `: {cwd_stem}` when OSC 7 ring is non-empty.
                let cwd = Mux::try_get()
                    .and_then(|m| m.pane(pane_id))
                    .and_then(|p| p.cwd.lock().clone());
                let title = vector_mux::format_tab_title(&label, cwd.as_deref());
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
                // D-84: if the selected profile is kind=codespace and no
                // access token is present in Keychain, divert to the device
                // flow rather than trying to connect. Reuses the same
                // AuthSignInRequested path as the menu item; once auth
                // completes Plan 06-06's Codespaces picker can re-issue the
                // profile selection.
                let is_codespace_profile = self
                    .current_config
                    .as_ref()
                    .and_then(|cfg| cfg.profile.get(&name))
                    .and_then(|p| p.kind)
                    .is_some_and(|k| matches!(k, vector_config::Kind::Codespace));
                let has_token = vector_codespaces::TokenStore::new().load_access().is_some();
                if is_codespace_profile && !has_token {
                    tracing::info!(
                        profile = %name,
                        "D-84: codespace profile selected without token — diverting to sign-in"
                    );
                    if let Some(proxy) = &self.proxy {
                        let _ = proxy.send_event(UserEvent::AuthSignInRequested);
                    }
                } else {
                    tracing::info!(
                        profile = %name,
                        is_codespace_profile,
                        has_token,
                        "ProfileSelected (connect path lands in Phase 7)"
                    );
                }
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
            // Phase 6 / AUTH-01 — device-flow user-event arms.
            UserEvent::AuthSignInRequested => {
                self.handle_auth_sign_in_requested();
            }
            UserEvent::AuthDisplayCode {
                user_code,
                verification_uri,
                expires_at,
                interval_secs,
            } => {
                let _ = interval_secs; // not surfaced in UI; useful for tracing
                self.handle_auth_display_code(user_code, verification_uri, expires_at);
            }
            UserEvent::AuthCompleted { user_login } => {
                self.handle_auth_completed(&user_login);
            }
            UserEvent::AuthFailed { reason } => {
                self.handle_auth_failed(&reason);
            }
            UserEvent::AuthRequired => {
                // Re-enter the device flow as if the user clicked the menu.
                if let Some(modal) = self.auth_modal.take() {
                    if let Some(mtm) = objc2::MainThreadMarker::new() {
                        modal.dismiss(mtm);
                    }
                }
                self.handle_auth_sign_in_requested();
            }
            UserEvent::SignOut => {
                let _ = vector_codespaces::TokenStore::new().clear();
                if let Some(mtm) = objc2::MainThreadMarker::new() {
                    unsafe { menu::rebuild_auth_menu_section(mtm, false, None) };
                }
                self.toasts.show(ToastBanner::info("signed out"));
                tracing::info!("sign_out_complete");
            }
            // Plan 06-06 / CS-01..03 — Codespaces picker handlers.
            UserEvent::OpenCodespacesPicker => {
                self.handle_open_codespaces_picker();
            }
            UserEvent::CodespacesLoaded(list) => {
                self.handle_codespaces_loaded(&list);
            }
            UserEvent::CodespacesLoadFailed(msg) => {
                tracing::warn!(error = %msg, "codespaces_load_failed");
                if let Some(modal) = self.codespaces_modal.as_mut() {
                    if let Some(mtm) = objc2::MainThreadMarker::new() {
                        modal.handle_load_failed(
                            mtm,
                            "could not fetch codespaces — check your connection".to_string(),
                        );
                    }
                }
            }
            UserEvent::CodespaceStateChanged { name, state } => {
                if let Some(modal) = self.codespaces_modal.as_mut() {
                    if let Some(mtm) = objc2::MainThreadMarker::new() {
                        modal.handle_state_change(mtm, &name, state);
                    }
                }
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
                // Plan 06-06 / CS-01..03 — picker keyboard routing.
                // When the codespaces picker is key, intercept Enter / Cmd-S /
                // Esc / arrows before the standard mux dispatch. The picker is
                // a separate NSPanel, so events reaching this winit window mean
                // the picker is NOT key; but we route here defensively so that
                // global hotkeys (Cmd-Shift-G, etc.) cannot collide with picker
                // navigation either.
                if event.state == ElementState::Pressed && self.codespaces_picker_is_key() {
                    if let Key::Character(s) = &event.logical_key {
                        match s.as_str() {
                            "s" | "S" if self.mods.cmd => {
                                self.codespaces_save_selected();
                                return;
                            }
                            _ => {}
                        }
                    }
                    if let Key::Named(named) = &event.logical_key {
                        match named {
                            NamedKey::Enter => {
                                // Available/Starting → Connect stub; Shutdown → Start.
                                if let Some(modal) = self.codespaces_modal.as_ref() {
                                    if let Some(cs) = modal.selected() {
                                        let label = crate::relative_time::state_label(cs.state);
                                        if label == "Shutdown" {
                                            self.codespaces_start_selected();
                                        } else {
                                            self.codespaces_connect_selected();
                                        }
                                    }
                                }
                                return;
                            }
                            NamedKey::Escape => {
                                self.codespaces_picker_dismiss();
                                return;
                            }
                            NamedKey::ArrowDown => {
                                if let (Some(modal), Some(mtm)) = (
                                    self.codespaces_modal.as_mut(),
                                    objc2::MainThreadMarker::new(),
                                ) {
                                    modal.select_next(mtm);
                                }
                                return;
                            }
                            NamedKey::ArrowUp => {
                                if let (Some(modal), Some(mtm)) = (
                                    self.codespaces_modal.as_mut(),
                                    objc2::MainThreadMarker::new(),
                                ) {
                                    modal.select_prev(mtm);
                                }
                                return;
                            }
                            _ => {}
                        }
                    }
                }
                // Cmd-V: read NSPasteboard + wrap in bracketed paste markers (D-53).
                if event.state == ElementState::Pressed && self.mods.cmd {
                    if let Key::Character(s) = &event.logical_key {
                        if s.as_str() == "v" {
                            let pasted = read_clipboard().unwrap_or_default();
                            let bytes = wrap_bracketed_paste(&pasted);
                            self.input_bridge.send_bytes(bytes);
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
                        self.input_bridge.send_bytes(bytes);
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
