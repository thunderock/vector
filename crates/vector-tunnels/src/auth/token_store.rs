//! macOS Keychain-backed GitHub OAuth token persistence. Mirrors Phase 6
//! `vector-codespaces::TokenStore` against `GITHUB_REFRESH_ACCOUNT`.
//!
//! Pitfall 14: manual Debug — secrets never enter logs.

use std::time::{Duration, UNIX_EPOCH};

use vector_secrets::Secrets;

use crate::auth::device_flow_github::GitHubTokens;
use crate::auth::error::GitHubAuthError;

/// Packs access + refresh + expiry into a single JSON blob stored under
/// `Secrets::GITHUB_REFRESH_ACCOUNT`. v1 mirrors Phase 6 — one entry per user.
pub struct GitHubTokenStore {
    secrets: Secrets,
}

impl std::fmt::Debug for GitHubTokenStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitHubTokenStore")
            .field("service", &self.secrets.service())
            .finish_non_exhaustive()
    }
}

impl GitHubTokenStore {
    pub fn new(secrets: Secrets) -> Self {
        Self { secrets }
    }

    pub fn for_vector() -> Self {
        Self::new(Secrets::for_vector())
    }

    pub fn save(&self, t: &GitHubTokens) -> Result<(), GitHubAuthError> {
        let expires_at_secs = t
            .expires_at
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let blob = serde_json::json!({
            "access_token": t.access_token,
            "refresh_token": t.refresh_token,
            "expires_at_unix": expires_at_secs,
        })
        .to_string();
        self.secrets
            .set(Secrets::GITHUB_REFRESH_ACCOUNT, &blob)
            .map_err(|e| GitHubAuthError::Storage(e.to_string()))
    }

    pub fn load(&self) -> Result<Option<GitHubTokens>, GitHubAuthError> {
        match self.secrets.get(Secrets::GITHUB_REFRESH_ACCOUNT) {
            Ok(blob) => {
                let v: serde_json::Value = serde_json::from_str(&blob)
                    .map_err(|e| GitHubAuthError::Storage(format!("invalid blob: {e}")))?;
                let access = v["access_token"].as_str().unwrap_or("").to_string();
                let refresh = v["refresh_token"].as_str().map(String::from);
                let exp_unix = v["expires_at_unix"].as_u64().unwrap_or(0);
                Ok(Some(GitHubTokens {
                    access_token: access,
                    refresh_token: refresh,
                    expires_at: UNIX_EPOCH + Duration::from_secs(exp_unix),
                }))
            }
            Err(_) => Ok(None), // not-present is the common path
        }
    }

    pub fn clear(&self) -> Result<(), GitHubAuthError> {
        let _ = self.secrets.delete(Secrets::GITHUB_REFRESH_ACCOUNT);
        Ok(())
    }
}

impl Default for GitHubTokenStore {
    fn default() -> Self {
        Self::for_vector()
    }
}
