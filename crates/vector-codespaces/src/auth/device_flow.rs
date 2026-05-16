//! OAuth Device Flow (RFC 8628) driver via `oauth2 5.0`.
//! Pitfall 14: every token-bearing struct has a hand-written Debug.

use std::time::{Duration, Instant};

use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, ClientId, DeviceAuthorizationUrl, RefreshToken, Scope,
    StandardDeviceAuthorizationResponse, TokenResponse, TokenUrl,
};
use zeroize::Zeroizing;

use crate::auth::AuthError;

// D-89: public per RFC 8628 §3.1. Replace with the registered
// vector-terminal client ID once OAuth App registration is complete.
pub const DEFAULT_CLIENT_ID: &str = "178c6fc778ccc68e1d6a"; // gh CLI fallback (D-89)
pub const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
pub const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

/// User-visible payload — safe to display in modals (no token material).
pub struct DeviceCodeDisplay {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_at: Instant,
    pub interval_secs: u64,
}

// user_code/verification_uri are public per RFC 8628 §3.1 — safe to Debug
// (but we still omit user_code to be conservative — see Plan 06-01 SUMMARY).
impl std::fmt::Debug for DeviceCodeDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceCodeDisplay")
            .field("verification_uri", &self.verification_uri)
            .field("expires_at", &self.expires_at)
            .field("interval_secs", &self.interval_secs)
            .finish_non_exhaustive()
    }
}

pub struct Tokens {
    pub access: Zeroizing<String>,
    pub refresh: Option<Zeroizing<String>>,
}

impl std::fmt::Debug for Tokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tokens")
            .field("has_refresh", &self.refresh.is_some())
            .finish_non_exhaustive()
    }
}

// Type-state alias matching `BasicClient<HasAuthUrl, HasDeviceAuthUrl, _, _, HasTokenUrl>`.
type ConfiguredClient = BasicClient<
    oauth2::EndpointSet,    // HasAuthUrl
    oauth2::EndpointSet,    // HasDeviceAuthUrl
    oauth2::EndpointNotSet, // HasIntrospectionUrl
    oauth2::EndpointNotSet, // HasRevocationUrl
    oauth2::EndpointSet,    // HasTokenUrl
>;

pub struct GitHubAuth {
    oauth_client: ConfiguredClient,
    http: reqwest::Client,
}

impl std::fmt::Debug for GitHubAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitHubAuth").finish_non_exhaustive()
    }
}

impl GitHubAuth {
    pub fn new() -> Result<Self, AuthError> {
        Self::new_with_endpoints(GITHUB_DEVICE_CODE_URL, GITHUB_TOKEN_URL, DEFAULT_CLIENT_ID)
    }

    /// Test-only constructor — points the device-flow + token endpoints at a
    /// wiremock base URL. Production code uses [`GitHubAuth::new`].
    pub fn new_with_endpoints(
        device_code_url: &str,
        token_url: &str,
        client_id: &str,
    ) -> Result<Self, AuthError> {
        let oauth_client = BasicClient::new(ClientId::new(client_id.to_string()))
            .set_auth_uri(AuthUrl::new(device_code_url.to_string())?)
            .set_token_uri(TokenUrl::new(token_url.to_string())?)
            .set_device_authorization_url(DeviceAuthorizationUrl::new(
                device_code_url.to_string(),
            )?);
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self { oauth_client, http })
    }

    pub async fn request_device_code(
        &self,
    ) -> Result<(DeviceCodeDisplay, StandardDeviceAuthorizationResponse), AuthError> {
        tracing::info!("device_flow_initiated");
        let details: StandardDeviceAuthorizationResponse = self
            .oauth_client
            .exchange_device_code()
            .add_scope(Scope::new("codespace".into()))
            .add_scope(Scope::new("read:user".into()))
            .request_async(&self.http)
            .await
            .map_err(|e| AuthError::OAuth(e.to_string()))?;

        let display = DeviceCodeDisplay {
            user_code: details.user_code().secret().clone(),
            verification_uri: details.verification_uri().to_string(),
            expires_at: Instant::now() + Duration::from_secs(details.expires_in().as_secs()),
            interval_secs: details.interval().as_secs(),
        };
        Ok((display, details))
    }

    pub async fn poll_for_token(
        &self,
        details: StandardDeviceAuthorizationResponse,
    ) -> Result<Tokens, AuthError> {
        tracing::info!("device_flow_polling");
        let resp = self
            .oauth_client
            .exchange_device_access_token(&details)
            .request_async(&self.http, tokio::time::sleep, None)
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("expired_token") {
                    AuthError::Expired
                } else if msg.contains("access_denied") {
                    AuthError::Cancelled
                } else {
                    AuthError::OAuth(msg)
                }
            })?;

        tracing::info!("device_flow_complete");
        Ok(Tokens {
            access: Zeroizing::new(resp.access_token().secret().clone()),
            refresh: resp
                .refresh_token()
                .map(|r: &RefreshToken| Zeroizing::new(r.secret().clone())),
        })
    }

    /// AUTH-03 helper. Returns new Tokens (rotated refresh if GitHub re-issues).
    pub async fn refresh_access_token(
        &self,
        refresh: &Zeroizing<String>,
    ) -> Result<Tokens, AuthError> {
        tracing::info!("token_refresh_attempted");
        let resp = self
            .oauth_client
            .exchange_refresh_token(&RefreshToken::new((**refresh).clone()))
            .request_async(&self.http)
            .await
            .map_err(|e| AuthError::OAuth(e.to_string()))?;
        tracing::info!("token_refresh_complete");
        Ok(Tokens {
            access: Zeroizing::new(resp.access_token().secret().clone()),
            refresh: resp
                .refresh_token()
                .map(|r: &RefreshToken| Zeroizing::new(r.secret().clone())),
        })
    }
}
