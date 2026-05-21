//! macOS Keychain-backed Microsoft OAuth token persistence. Mirrors Phase 6
//! `vector-codespaces::TokenStore` against `MICROSOFT_REFRESH_ACCOUNT` /
//! `MICROSOFT_OAUTH_ACCOUNT`.
//!
//! Pitfall 14: manual Debug — secrets never enter logs.

use std::time::{Duration, UNIX_EPOCH};

use vector_secrets::Secrets;

use crate::auth::device_flow_microsoft::MicrosoftTokens;
use crate::auth::error::MicrosoftAuthError;

/// Packs access + refresh + expiry into a single JSON blob stored under
/// `Secrets::MICROSOFT_REFRESH_ACCOUNT`. v1 mirrors Phase 6 — one entry per user.
pub struct MicrosoftTokenStore {
    secrets: Secrets,
}

impl std::fmt::Debug for MicrosoftTokenStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MicrosoftTokenStore")
            .field("service", &self.secrets.service())
            .finish_non_exhaustive()
    }
}

impl MicrosoftTokenStore {
    pub fn new(secrets: Secrets) -> Self {
        Self { secrets }
    }

    pub fn for_vector() -> Self {
        Self::new(Secrets::for_vector())
    }

    pub fn save(&self, t: &MicrosoftTokens) -> Result<(), MicrosoftAuthError> {
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
            .set(Secrets::MICROSOFT_REFRESH_ACCOUNT, &blob)
            .map_err(|e| MicrosoftAuthError::Storage(e.to_string()))
    }

    pub fn load(&self) -> Result<Option<MicrosoftTokens>, MicrosoftAuthError> {
        match self.secrets.get(Secrets::MICROSOFT_REFRESH_ACCOUNT) {
            Ok(blob) => {
                let v: serde_json::Value = serde_json::from_str(&blob)
                    .map_err(|e| MicrosoftAuthError::Storage(format!("invalid blob: {e}")))?;
                let access = v["access_token"].as_str().unwrap_or("").to_string();
                let refresh = v["refresh_token"].as_str().map(String::from);
                let exp_unix = v["expires_at_unix"].as_u64().unwrap_or(0);
                Ok(Some(MicrosoftTokens {
                    access_token: access,
                    refresh_token: refresh,
                    expires_at: UNIX_EPOCH + Duration::from_secs(exp_unix),
                }))
            }
            Err(_) => Ok(None), // not-present is the common path
        }
    }

    pub fn clear(&self) -> Result<(), MicrosoftAuthError> {
        let _ = self.secrets.delete(Secrets::MICROSOFT_REFRESH_ACCOUNT);
        Ok(())
    }
}

impl Default for MicrosoftTokenStore {
    fn default() -> Self {
        Self::for_vector()
    }
}
