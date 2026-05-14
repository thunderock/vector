//! Profile writer — D-87 / CS-03. Plan 06-04 fills in append_codespace_profile.

use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum WriterError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse: {0}")]
    TomlParse(#[from] toml_edit::TomlError),
}

/// Append a `[profile.X]` block of kind="codespace". Plan 06-04 fills in.
pub fn append_codespace_profile(
    _config_path: &Path,
    _profile_name: &str,
    _codespace_name: &str,
    _tint: &str,
) -> Result<String, WriterError> {
    unimplemented!("Plan 06-04 — derive name + toml_edit append + atomic rename")
}

/// Derive a profile name from a codespace name per §5.3 of UI-SPEC.
/// Strip owner prefix and trailing `-[a-z0-9]{4,}` random suffix.
/// Plan 06-04 fills in.
pub fn derive_profile_name(_codespace_name: &str, _existing_profiles: &[&str]) -> String {
    unimplemented!("Plan 06-04 — regex strip + de-collide loop")
}
