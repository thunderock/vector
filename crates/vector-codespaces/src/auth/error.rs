#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("oauth2 error: {0}")]
    OAuth(String),
    #[error("reqwest error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("secrets error: {0}")]
    Secrets(#[from] vector_secrets::SecretsError),
    #[error("url parse error: {0}")]
    Url(#[from] oauth2::url::ParseError),
    #[error("user cancelled sign-in")]
    Cancelled,
    #[error("device code expired")]
    Expired,
    #[error("refresh token absent — must re-run device flow")]
    NoRefreshToken,
    #[error("token rejected (401) — must re-run device flow")]
    Unauthorized,
}
