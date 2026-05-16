//! macOS Keychain-backed OAuth token persistence (AUTH-02).
//! Pitfall 14: manual Debug.

use vector_secrets::Secrets;
use zeroize::Zeroizing;

use crate::auth::AuthError;

pub struct TokenStore {
    secrets: Secrets,
}

impl std::fmt::Debug for TokenStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenStore")
            .field("service", &self.secrets.service())
            .finish_non_exhaustive()
    }
}

impl TokenStore {
    pub fn new() -> Self {
        Self {
            secrets: Secrets::for_vector(),
        }
    }

    pub fn save_access(&self, token: &Zeroizing<String>) -> Result<(), AuthError> {
        self.secrets.set(Secrets::GITHUB_OAUTH_ACCOUNT, token)?;
        Ok(())
    }

    pub fn save_refresh(&self, token: &Zeroizing<String>) -> Result<(), AuthError> {
        self.secrets.set(Secrets::GITHUB_REFRESH_ACCOUNT, token)?;
        Ok(())
    }

    pub fn load_access(&self) -> Option<Zeroizing<String>> {
        self.secrets
            .get(Secrets::GITHUB_OAUTH_ACCOUNT)
            .ok()
            .map(Zeroizing::new)
    }

    pub fn load_refresh(&self) -> Option<Zeroizing<String>> {
        self.secrets
            .get(Secrets::GITHUB_REFRESH_ACCOUNT)
            .ok()
            .map(Zeroizing::new)
    }

    /// Best-effort clear. Missing entries are not errors.
    pub fn clear(&self) -> Result<(), AuthError> {
        let _ = self.secrets.delete(Secrets::GITHUB_OAUTH_ACCOUNT);
        let _ = self.secrets.delete(Secrets::GITHUB_REFRESH_ACCOUNT);
        Ok(())
    }
}

impl Default for TokenStore {
    fn default() -> Self {
        Self::new()
    }
}
