//! Vector app shell crate. The binary entry is `main.rs`; this library exposes
//! the modules so integration tests (and Plan 04-04's multi_window_tabbing test)
//! can drive them without spinning up a real event loop.

#![allow(unsafe_code)]

use vector_mux::PaneId;

/// M4 / D-69 — bundled default config TOML. Written to `~/.config/vector/config.toml`
/// on first launch when the file is absent. Ships the Cmd-Shift-R reload-config keybind
/// as the FSEvents-missed fallback (Plan 05-04 watcher; Plan 05-10 menu fallback).
pub const DEFAULT_CONFIG_TOML: &str = "[default]\ntheme = \"vector-dark\"\n\n# M4 / D-69: Cmd-Shift-R fallback for FSEvents-missed config reloads.\n[[keybind]]\nkey = \"cmd-shift-r\"\naction = \"reload-config\"\n";

pub mod app;
pub mod chrome;
pub mod clipboard_router;
// Phase 8 / Plan 08-05 — DevTunnels picker + Microsoft auth + actor.
pub mod devtunnels_actor;
pub mod devtunnels_modal;
pub mod frame_tick;
pub mod hyperlink_dispatch;
pub mod ime;
pub mod input_bridge;
pub mod lpm;
pub mod menu;
pub mod microsoft_auth_modal;
pub mod mux_commands;
pub mod overlay;
pub mod profile_picker;
pub mod pty_actor;
pub mod relative_time;
pub mod render_host;
pub mod search_bar;
pub mod ske;
pub mod tab_window;
pub mod term_grid_access;
pub mod toast;

pub use mux_commands::{WindowFactory, WinitWindowFactory, VECTOR_TABBING_IDENTIFIER};
pub use tab_window::TabWindow;

/// Phase-4 cross-thread event variants. Plan 04-03 keyed PtyOutput / Resized by PaneId.
/// Plan 05-10 extends with chrome / config / hyperlink / Cmd-N variants.
#[derive(Debug, Clone)]
pub enum UserEvent {
    PaneOutput {
        pane_id: PaneId,
        bytes: Vec<u8>,
    },
    PaneResized {
        pane_id: PaneId,
        rows: u16,
        cols: u16,
    },
    PaneExited(PaneId),
    PaneTitleChanged {
        pane_id: PaneId,
        label: String,
    },
    LpmChanged(bool),
    // Plan 05-10 additions (additive; no Phase 1-4 renames):
    ConfigReloaded(std::sync::Arc<vector_config::ConfigFile>),
    ConfigError(String),
    OpenProfilePicker,
    ProfileSelected(String),
    ToggleSearch,
    ToggleSecureKeyboardEntry,
    SpawnNewWindow, // D-82 Cmd-N
    ReloadConfig,   // M4 — Cmd-Shift-R + View → Reload Config
    HyperlinkClicked {
        url: String,
    }, // B1
    ToastInfo(String), // M2 helper for one-shot info toasts
    /// Plan 05-12 (POLISH-05 gap-closure): OSC 52 Store routed from the
    /// I/O thread's `clipboard_rx` drain task to App.clipboard_router.
    /// `kind_is_selection` keeps alacritty_terminal::ClipboardType out of this enum.
    ClipboardStore {
        kind_is_selection: bool,
        data: String,
    },
    // Phase 9.1 Gap B: GitHub OAuth + Codespaces UserEvent variants removed
    // (AuthSignInRequested, AuthDisplayCode, AuthCompleted, AuthFailed,
    // AuthRequired, SignOut, OpenCodespacesPicker, CodespacesLoaded,
    // CodespacesLoadFailed, CodespaceStateChanged). Microsoft auth path
    // below is the sole sign-in surface in v1.
    // ───── Phase 8 (DT-02/03/04) — appended; never reorder ─────
    /// Microsoft device flow obtained user_code; main thread shows MicrosoftAuthDeviceFlowModal.
    MicrosoftDeviceFlowStarted {
        user_code: String,
        verification_uri: String,
        expires_in: std::time::Duration,
        cancel: tokio_util::sync::CancellationToken,
    },
    /// Microsoft device flow completed; tokens persisted to Keychain.
    MicrosoftSignedIn,
    /// Microsoft sign-in ended with a non-user terminal error.
    MicrosoftSignInFailed(String),
    /// Microsoft sign-in cancelled by the user (Cancel button or modal close).
    MicrosoftSignInCancelled,
    /// DevTunnels picker REST list arrived (already filtered to vector-agent tunnels).
    DevTunnelsLoaded(Vec<devtunnels_actor::TunnelView>),
    /// DevTunnels list call failed (non-401 error).
    DevTunnelsLoadFailed(String),
    /// DevTunnels list returned 401 (or no Microsoft token present); picker should prompt sign-in.
    DevTunnelsAuthRequired,
    /// Picker requests a connect for tunnel_id.
    DevTunnelConnectRequested {
        tunnel_id: String,
    },
    /// Actor began the connect dance (relay handshake + transport install).
    DevTunnelConnectStarted(String),
    /// New DevTunnel pane installed via Mux::create_tab_async_with_transport.
    DevTunnelPaneReady {
        window_id: vector_mux::WindowId,
        tab_id: vector_mux::TabId,
        pane_id: PaneId,
    },
    /// Connect failed; toast UI-SPEC §Connection error & toast copy.
    DevTunnelConnectFailed {
        tunnel_id: String,
        reason: String,
    },
    /// Menu "Sign in with Microsoft" clicked.
    MicrosoftSignInRequested,
    /// Menu "Sign out of Microsoft" clicked.
    MicrosoftSignOutRequested,
    /// Menu "Dev Tunnels…" clicked (equivalent to Cmd-Shift-T keypress).
    OpenDevTunnelsPickerMenu,
    // ───── Phase 9 (PERSIST-01/02) — appended; never reorder ─────
    /// Per-pane I/O actor observed transport EOF; UI overlays inline status bar
    /// and updates tab badge to `[reconnecting]`. Fires on initial entry to the
    /// Reconnecting state AND on every subsequent attempt (UI uses `attempt`
    /// for the `(attempt N)` substring per UI-SPEC §Copywriting).
    PaneReconnecting {
        pane_id: PaneId,
        attempt: u32,
        profile_label: String,
    },
    /// Per-pane I/O actor successfully hot-swapped a fresh transport. UI removes
    /// the inline status bar (200 ms fade-out) and reverts tab badge to `[remote]`.
    PaneReconnected {
        pane_id: PaneId,
    },
    /// Picker actor installs a per-pane cancel token after a successful Dev Tunnel
    /// connect. App stores it; pane-close handler invokes `.cancel()` to exit
    /// the reconnect loop promptly.
    DevTunnelPaneCancelToken {
        pane_id: PaneId,
        cancel: tokio_util::sync::CancellationToken,
    },
    /// Cmd-T new-tab spawn ack: I/O thread finished `mux.create_window` +
    /// `create_tab_async` + `spawn_pane`. Main thread now binds the new winit
    /// window to the fresh mux WindowId and seeds the AppWindow's active pane.
    NewTabReady {
        winit_window_id: winit::window::WindowId,
        mux_window_id: vector_mux::WindowId,
        pane_id: PaneId,
    },
}
