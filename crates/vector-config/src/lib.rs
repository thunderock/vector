//! vector-config — Phase 5 TOML config + hot reload (POLISH-01, POLISH-07).

pub mod apply;
pub mod error;
pub mod loader;
pub mod schema;
pub mod watcher;

pub use apply::{diff_config, try_load_or_keep, ApplyPlan, LiveChange, RestartReason};
pub use error::ConfigError;
pub use loader::{parse, resolve_profile, ResolvedProfile};
pub use schema::{
    Action, Appearance, ClipboardPolicy, ConfigFile, FontCfg, KeyBind, Kind, ProfileBlock,
};
pub use watcher::spawn_watcher;

/// FS-watcher signal emitted after debounce flush. Plan 05-08 pumps this into
/// `EventLoopProxy<UserEvent::ConfigReloaded>` on the main thread.
#[derive(Debug, Clone)]
pub enum ConfigEvent {
    Dirty { paths: Vec<std::path::PathBuf> },
    Error(String),
}
