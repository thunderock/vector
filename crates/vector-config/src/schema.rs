//! TOML schema for ~/.config/vector/config.toml.
//! D-68: single file, [default] + [profile.X] flat-overlay inheritance, deny_unknown_fields.
//! D-74: Profile.kind = { Local, Codespace, DevTunnel }; only Local wired in Phase 5.

use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(serde::Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct ConfigFile {
    #[serde(default)]
    pub default: ProfileBlock,
    #[serde(default)]
    pub profile: BTreeMap<String, ProfileBlock>,
    #[serde(default)]
    pub keybind: Vec<KeyBind>,
}

#[derive(serde::Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct ProfileBlock {
    pub kind: Option<Kind>,
    pub theme: Option<String>,
    pub tint: Option<String>,
    pub appearance: Option<Appearance>,
    pub font: Option<FontCfg>,
    pub clipboard_write: Option<ClipboardPolicy>,
    pub secure_keyboard_entry: Option<bool>,
    pub env: Option<BTreeMap<String, String>>,
    pub startup_command: Option<String>,
    pub codespace_name: Option<String>,
    pub dev_tunnel_id: Option<String>,
    #[serde(default)]
    pub cwd_override: Option<PathBuf>,
}

#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Kind {
    Local,
    Codespace,
    DevTunnel,
}

#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum Appearance {
    System,
    Light,
    Dark,
}

#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum ClipboardPolicy {
    Allow,
    Block,
}

#[derive(serde::Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct FontCfg {
    pub family: Option<String>,
    pub size: Option<f32>,
    pub ligatures: Option<bool>,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct KeyBind {
    pub key: String,
    pub action: Action,
}

#[derive(serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Action {
    NewWindow,
    NewTab,
    SplitHorizontal,
    SplitVertical,
    ReloadConfig,
    OpenSearch,
    OpenProfilePicker,
    Copy,
    Paste,
    ToggleSecureKeyboardEntry,
}
