//! OAuth device flow + token cache. Pitfall 14 manual Debug throughout.
mod device_flow;
mod error;
mod token_store;

pub use device_flow::DeviceCodeDisplay;
pub use error::AuthError;
pub use token_store::TokenStore;

/// OAuth device-flow driver. Plan 06-02 fills in the implementation.
pub struct GitHubAuth {
    // Plan 06-02: oauth_client: oauth2::basic::BasicClient
    // Plan 06-02: http: reqwest::Client
    _placeholder: (),
}

// Pitfall 14: manual Debug — NEVER derive
impl std::fmt::Debug for GitHubAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitHubAuth").finish_non_exhaustive()
    }
}

impl GitHubAuth {
    pub fn new() -> Result<Self, AuthError> {
        Ok(Self { _placeholder: () })
    }
}
