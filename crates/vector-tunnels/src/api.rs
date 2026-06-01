//! Dev Tunnels Management API client (D-06): list_tunnels + get_access_token.
//!
//! Pitfall 14: no `#[derive(Debug)]` near token-bearing fields; manual Debug below.

use crate::model::{AuthProvider, TunnelRecord};
use serde::Deserialize;
use thiserror::Error;

// Dev Tunnels has no POST `/access` endpoint; a connect token is returned
// inline on the tunnel object when fetched with `?tokenScopes=connect`.
#[derive(Deserialize)]
struct TunnelAccessBody {
    #[serde(rename = "accessTokens")]
    access_tokens: Option<AccessTokens>,
}

#[derive(Deserialize)]
struct AccessTokens {
    connect: Option<String>,
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
                // The live relay returns a top-level JSON array, not a {"value":[...]} envelope.
                let v: Vec<TunnelRecord> = resp.json().await?;
                Ok(v.into_iter()
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
            "{}/api/v1/tunnels/{}?tokenScopes=connect",
            self.base_url, tunnel_id
        );
        let resp = self
            .http
            .get(&url)
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
        resp.json::<TunnelAccessBody>()
            .await?
            .access_tokens
            .and_then(|t| t.connect)
            .ok_or_else(|| ApiError::Other {
                status: status.as_u16(),
                body: "tunnel response carried no connect access token".into(),
            })
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
