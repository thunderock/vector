//! Keychain-backed token persistence. Plan 06-02 fills in save/load/clear.
use vector_secrets::Secrets;
use zeroize::Zeroizing;

use crate::auth::AuthError;

pub struct TokenStore {
    secrets: Secrets,
}

// Pitfall 14: manual Debug.
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

    pub fn save_access(&self, _token: &Zeroizing<String>) -> Result<(), AuthError> {
        unimplemented!("Plan 06-02")
    }

    pub fn save_refresh(&self, _token: &Zeroizing<String>) -> Result<(), AuthError> {
        unimplemented!("Plan 06-02")
    }

    pub fn load_access(&self) -> Option<Zeroizing<String>> {
        None
    }

    pub fn load_refresh(&self) -> Option<Zeroizing<String>> {
        None
    }

    pub fn clear(&self) -> Result<(), AuthError> {
        unimplemented!("Plan 06-02")
    }
}

impl Default for TokenStore {
    fn default() -> Self {
        Self::new()
    }
}
