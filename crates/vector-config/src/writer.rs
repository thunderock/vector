//! Profile writer — D-87 / CS-03 / UI-SPEC §5.3.
//! Atomic round-trip via toml_edit + tempfile rename.

use std::path::Path;

use regex::Regex;
use toml_edit::{DocumentMut, Item, Table};

#[derive(Debug, thiserror::Error)]
pub enum WriterError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse: {0}")]
    TomlParse(#[from] toml_edit::TomlError),
    #[error("config path has no parent directory")]
    NoParent,
}

/// Derive a profile name per UI-SPEC §5.3.
/// 1. Strip `owner/` prefix.
/// 2. Strip trailing `-[a-z0-9]{4,}` (case-insensitive).
/// 3. If empty, fall back to owner (or first 6 chars when slash missing).
/// 4. De-collide against `existing` by appending `-2`, `-3`, ...
pub fn derive_profile_name(codespace_name: &str, existing: &[&str]) -> String {
    let (owner, rest) = match codespace_name.split_once('/') {
        Some((o, r)) => (o, r),
        None => ("", codespace_name),
    };
    let random_suffix = Regex::new(r"(?i)-[a-z0-9]{4,}$").expect("static regex");
    let stripped: String = random_suffix.replace(rest, "").to_string();
    let base = if stripped.is_empty() {
        if owner.is_empty() {
            codespace_name.chars().take(6).collect::<String>()
        } else {
            owner.to_string()
        }
    } else {
        stripped
    };

    if !existing.iter().any(|e| *e == base) {
        return base;
    }
    let mut n: u32 = 2;
    loop {
        let candidate = format!("{base}-{n}");
        if !existing.iter().any(|e| *e == candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// Append `[profile.{profile_name}]` to the config file, preserving formatting.
/// Returns the actual profile name written (may differ from input if collision).
pub fn append_codespace_profile(
    config_path: &Path,
    profile_name: &str,
    codespace_name: &str,
    tint: &str,
) -> Result<String, WriterError> {
    let source = match std::fs::read_to_string(config_path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
    let mut doc: DocumentMut = if source.is_empty() {
        DocumentMut::new()
    } else {
        source.parse()?
    };

    let existing = collect_profile_names(&doc);
    let refs: Vec<&str> = existing.iter().map(String::as_str).collect();
    let final_name: String = if refs.contains(&profile_name) {
        let mut n: u32 = 2;
        loop {
            let candidate = format!("{profile_name}-{n}");
            if !refs.contains(&candidate.as_str()) {
                break candidate;
            }
            n += 1;
        }
    } else {
        profile_name.to_string()
    };

    let mut tbl = Table::new();
    tbl["kind"] = toml_edit::value("codespace");
    tbl["codespace_name"] = toml_edit::value(codespace_name);
    tbl["tint"] = toml_edit::value(tint);

    let profiles_item = doc.entry("profile").or_insert(Item::Table(Table::new()));
    if let Item::Table(profiles) = profiles_item {
        profiles.insert(&final_name, Item::Table(tbl));
    }

    atomic_write(config_path, &doc.to_string())?;
    tracing::info!(profile = %final_name, "codespace_profile_written");
    Ok(final_name)
}

fn collect_profile_names(doc: &DocumentMut) -> Vec<String> {
    match doc.get("profile") {
        Some(Item::Table(t)) => t.iter().map(|(k, _)| k.to_string()).collect(),
        _ => Vec::new(),
    }
}

fn atomic_write(path: &Path, content: &str) -> Result<(), WriterError> {
    let parent = path.parent().ok_or(WriterError::NoParent)?;
    let file_name = path
        .file_name()
        .map_or_else(|| "config.toml".to_string(), |s| s.to_string_lossy().into_owned());
    let tmp = parent.join(format!("{file_name}.tmp"));
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
