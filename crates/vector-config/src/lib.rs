//! vector-config — Phase 5 TOML config + hot reload (POLISH-01, POLISH-07).

pub mod error;
pub mod loader;
pub mod schema;

pub use error::ConfigError;
pub use loader::{parse, resolve_profile, ResolvedProfile};
pub use schema::{
    Action, Appearance, ClipboardPolicy, ConfigFile, FontCfg, KeyBind, Kind, ProfileBlock,
};
