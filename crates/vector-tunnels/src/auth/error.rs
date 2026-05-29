//! GitHub OAuth Device Flow error variants (Dev Tunnels GitHub App).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitHubAuthError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("device code expired before user completed sign-in")]
    DeviceCodeExpired,
    #[error("user denied authorization")]
    AccessDenied,
    #[error("refresh token expired or revoked — re-authentication required")]
    RefreshExpired,
    #[error("sign-in cancelled")]
    Cancelled,
    #[error("GitHub App does not have device flow enabled")]
    DeviceFlowDisabled,
    #[error("GitHub returned unexpected response: {0}")]
    Unexpected(String),
    #[error("token persistence error: {0}")]
    Storage(String),
}
