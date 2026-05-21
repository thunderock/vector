//! Microsoft OAuth Device Flow error variants. Mirrors `vector-codespaces::AuthError`
//! shape against Microsoft Entra `common` endpoints.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MicrosoftAuthError {
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
    #[error("Microsoft returned unexpected response: {0}")]
    Unexpected(String),
    #[error("token persistence error: {0}")]
    Storage(String),
}
