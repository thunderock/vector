//! Phase 5 / POLISH-08 — vector-secrets API surface lock.
//!
//! Phase 6 OAuth token storage is the first writer; this crate only locks the
//! shape and idempotently installs the macOS Keychain backend on first use.
//!
//! Pitfall 14: token-bearing types implement `Debug` manually so the secret
//! material never enters tracing or logs.

use std::sync::Once;

use keyring_core::Entry;

#[cfg(target_os = "macos")]
use apple_native_keyring_store::keychain::Store as KeychainStore;

#[derive(Debug, thiserror::Error)]
pub enum SecretsError {
    #[error("keyring error: {0}")]
    Keyring(#[from] keyring_core::Error),
    #[cfg(target_os = "macos")]
    #[error("keyring store init failed: {0}")]
    StoreInit(keyring_core::Error),
}

static INSTALL_STORE: Once = Once::new();

fn ensure_default_store() -> Result<(), SecretsError> {
    let mut err: Option<SecretsError> = None;
    INSTALL_STORE.call_once(|| {
        #[cfg(target_os = "macos")]
        {
            match KeychainStore::new() {
                Ok(store) => keyring_core::set_default_store(store),
                Err(e) => err = Some(SecretsError::StoreInit(e)),
            }
        }
    });
    match err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Vector's Keychain access surface. Phase 6's OAuth code is the first caller
/// of [`Secrets::set`] / [`Secrets::get`]; nothing in Phase 5 writes secrets.
pub struct Secrets {
    service: String,
}

impl Secrets {
    pub const VECTOR_SERVICE: &str = "vector";
    pub const GITHUB_OAUTH_ACCOUNT: &str = "github_oauth_token";
    pub const GITHUB_REFRESH_ACCOUNT: &str = "github_refresh_token";

    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    pub fn for_vector() -> Self {
        Self::new(Self::VECTOR_SERVICE)
    }

    pub fn service(&self) -> &str {
        &self.service
    }

    pub fn get(&self, account: &str) -> Result<String, SecretsError> {
        ensure_default_store()?;
        let entry = Entry::new(&self.service, account)?;
        Ok(entry.get_password()?)
    }

    pub fn set(&self, account: &str, secret: &str) -> Result<(), SecretsError> {
        ensure_default_store()?;
        let entry = Entry::new(&self.service, account)?;
        entry.set_password(secret)?;
        Ok(())
    }

    pub fn delete(&self, account: &str) -> Result<(), SecretsError> {
        ensure_default_store()?;
        let entry = Entry::new(&self.service, account)?;
        entry.delete_credential()?;
        Ok(())
    }
}

/// Pitfall 14: manual Debug — NEVER derive. The secret material never enters
/// tracing or logs through this type.
impl std::fmt::Debug for Secrets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Secrets")
            .field("service", &self.service)
            .finish_non_exhaustive()
    }
}

// `zeroize` is intentionally a direct dependency for Phase 6 callers that
// hold raw OAuth refresh-token bytes — they wrap them in `zeroize::Zeroizing<_>`
// at the call site. Surfaced here so the audit trail is one cargo tree away.
#[doc(hidden)]
pub use zeroize;
