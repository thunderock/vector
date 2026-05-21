//! On-disk OAuth token cache for the agent.
//! Path: `$XDG_CONFIG_HOME/vector/agent-token` (or `~/.config/vector/agent-token`).
//! Atomic write via temp+rename; mode 0600; parent dir 0700.
//! Pitfall 14: manual `Debug` so token bytes never leak through `{:?}`.

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub fn token_path() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("vector").join("agent-token");
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".config")
        .join("vector")
        .join("agent-token")
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CachedToken {
    pub provider: Provider,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at_unix: u64,
}

// Pitfall-14 manual Debug — never includes token bytes.
impl std::fmt::Debug for CachedToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedToken")
            .field("provider", &self.provider)
            .field("has_refresh", &self.refresh_token.is_some())
            .field("expires_at_unix", &self.expires_at_unix)
            .finish_non_exhaustive()
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    GitHub,
    Microsoft,
}

// Manual Debug — strict acceptance gate forbids any `derive(Debug)` in this file.
impl std::fmt::Debug for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::GitHub => "GitHub",
            Self::Microsoft => "Microsoft",
        })
    }
}

#[derive(thiserror::Error)]
pub enum AgentTokenError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("corrupted token file: {0}")]
    Corrupted(String),
}

// Manual Debug — same strict acceptance gate as above.
impl std::fmt::Debug for AgentTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

pub fn save(t: &CachedToken) -> Result<(), AgentTokenError> {
    let path = token_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
        // Tighten directory mode 0700.
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    let json = serde_json::to_string(t).map_err(|e| AgentTokenError::Corrupted(e.to_string()))?;
    // Atomic write: temp + chmod + rename.
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, json)?;
    fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn load() -> Result<Option<CachedToken>, AgentTokenError> {
    let path = token_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)?;
    let t: CachedToken =
        serde_json::from_str(&content).map_err(|e| AgentTokenError::Corrupted(e.to_string()))?;
    Ok(Some(t))
}

pub fn clear() -> Result<(), AgentTokenError> {
    let path = token_path();
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}
