//! Dev Tunnels Management API client (D-06): list_tunnels + get_access_token.
//!
//! Pitfall 14: no `#[derive(Debug)]` near token-bearing fields; manual Debug below.

use crate::model::{AuthProvider, TunnelRecord};
use serde::Deserialize;
use thiserror::Error;

#[derive(Deserialize)]
struct ListBody {
    value: Vec<TunnelRecord>,
}

#[derive(Deserialize)]
struct TokenBody {
    token: String,
}

pub const TUNNELS_BASE_URL: &str = "https://global.rel.tunnels.api.visualstudio.com";

#[derive(Error)]
pub enum ApiError {
    #[error("HTTP: {0}")]
    Http(#[from] reqwest::Error),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("api: {status}: {body}")]
    Other { status: u16, body: String },
}

// Manual Debug — pattern-match codebase Pitfall-14 discipline (api.rs grep gate).
impl std::fmt::Debug for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(e) => f.debug_tuple("Http").field(e).finish(),
            Self::Unauthorized => f.write_str("Unauthorized"),
            Self::Forbidden => f.write_str("Forbidden"),
            Self::NotFound => f.write_str("NotFound"),
            Self::Other { status, body } => f
                .debug_struct("Other")
                .field("status", status)
                .field("body", body)
                .finish(),
        }
    }
}

pub struct DevTunnelsApi {
    http: reqwest::Client,
    base_url: String,
}

// Manual Debug — http client + base_url; base_url is documentation, not secret.
impl std::fmt::Debug for DevTunnelsApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DevTunnelsApi")
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}

impl Default for DevTunnelsApi {
    fn default() -> Self {
        Self::new()
    }
}

impl DevTunnelsApi {
    #[must_use]
    pub fn new() -> Self {
        Self::with_base_url(TUNNELS_BASE_URL.into())
    }

    #[must_use]
    pub fn with_base_url(base_url: String) -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent(concat!("Vector/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("reqwest client"),
            base_url,
        }
    }

    /// List tunnels under the auth identity, filtered to `vector-agent: true` (D-10).
    pub async fn list_tunnels(&self, auth: &AuthProvider) -> Result<Vec<TunnelRecord>, ApiError> {
        let url = format!("{}/api/v1/tunnels", self.base_url);
        let resp = self
            .http
            .get(&url)
            .header("Authorization", auth.format_header())
            .send()
            .await?;
        match resp.status().as_u16() {
            401 => Err(ApiError::Unauthorized),
            403 => Err(ApiError::Forbidden),
            404 => Err(ApiError::NotFound),
            s if (200..300).contains(&s) => {
                let b: ListBody = resp.json().await?;
                Ok(b.value
                    .into_iter()
                    .filter(TunnelRecord::is_vector_agent)
                    .collect())
            }
            s => Err(ApiError::Other {
                status: s,
                body: resp.text().await.unwrap_or_default(),
            }),
        }
    }

    /// Fetch a connect-scope access token for a tunnel. Plan 08-06's actor refreshes
    /// upstream (Microsoft/GitHub) on 401; this method returns Unauthorized as a signal.
    pub async fn get_access_token(
        &self,
        auth: &AuthProvider,
        tunnel_id: &str,
    ) -> Result<String, ApiError> {
        let url = format!(
            "{}/api/v1/tunnels/{}/access?scopes=connect",
            self.base_url, tunnel_id
        );
        let resp = self
            .http
            .post(&url)
            .header("Authorization", auth.format_header())
            .send()
            .await?;
        let status = resp.status();
        if status.as_u16() == 401 {
            return Err(ApiError::Unauthorized);
        }
        if !status.is_success() {
            return Err(ApiError::Other {
                status: status.as_u16(),
                body: resp.text().await.unwrap_or_default(),
            });
        }
        Ok(resp.json::<TokenBody>().await?.token)
    }

    /// Fetch a single tunnel by ID. Used by the picker to refresh endpoint state.
    pub async fn get_tunnel(
        &self,
        auth: &AuthProvider,
        tunnel_id: &str,
    ) -> Result<TunnelRecord, ApiError> {
        let url = format!("{}/api/v1/tunnels/{}", self.base_url, tunnel_id);
        let resp = self
            .http
            .get(&url)
            .header("Authorization", auth.format_header())
            .send()
            .await?;
        match resp.status().as_u16() {
            401 => Err(ApiError::Unauthorized),
            403 => Err(ApiError::Forbidden),
            404 => Err(ApiError::NotFound),
            s if (200..300).contains(&s) => Ok(resp.json::<TunnelRecord>().await?),
            s => Err(ApiError::Other {
                status: s,
                body: resp.text().await.unwrap_or_default(),
            }),
        }
    }
}
