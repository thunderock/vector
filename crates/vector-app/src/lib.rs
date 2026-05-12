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
pub mod clipboard_router;
pub mod frame_tick;
pub mod hyperlink_dispatch;
pub mod input_bridge;
pub mod lpm;
pub mod menu;
pub mod mux_commands;
pub mod overlay;
pub mod profile_picker;
pub mod pty_actor;
pub mod render_host;
pub mod search_bar;
pub mod ske;
pub mod tab_window;
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
    SpawnNewWindow,                   // D-82 Cmd-N
    ReloadConfig,                     // M4 — Cmd-Shift-R + View → Reload Config
    HyperlinkClicked { url: String }, // B1
    ToastInfo(String),                // M2 helper for one-shot info toasts
}
