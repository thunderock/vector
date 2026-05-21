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
    api_base_url: String,
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
        Self::new_with_endpoints_and_api(
            device_code_url,
            token_url,
            client_id,
            "https://api.github.com",
        )
    }

    /// Test-only constructor — also overrides the REST API base URL so tests
    /// can point `/user` and `/user/codespaces` at a wiremock server.
    pub fn new_with_endpoints_and_api(
        device_code_url: &str,
        token_url: &str,
        client_id: &str,
        api_base_url: &str,
    ) -> Result<Self, AuthError> {
        let oauth_client = BasicClient::new(ClientId::new(client_id.to_string()))
            .set_auth_uri(AuthUrl::new(device_code_url.to_string())?)
            .set_token_uri(TokenUrl::new(token_url.to_string())?)
            .set_device_authorization_url(DeviceAuthorizationUrl::new(
                device_code_url.to_string(),
            )?);
        // GitHub's /login/oauth/access_token returns application/x-www-form-urlencoded
        // unless the client explicitly asks for JSON. oauth2 5.x only parses JSON.
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        let http = reqwest::Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self {
            oauth_client,
            http,
            api_base_url: api_base_url.trim_end_matches('/').to_string(),
        })
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
        // GitHub returns HTTP 200 with `{"error":"authorization_pending"}` while
        // waiting — RFC 8628 says HTTP 400. oauth2 5.x can't reconcile this and
        // bails with "Failed to parse server response". Drive the poll loop
        // directly against reqwest instead.
        let token_url = self.oauth_client.token_uri().as_str().to_string();
        let client_id = self.oauth_client.client_id().as_str().to_string();
        let device_code = details.device_code().secret().clone();
        let mut interval = Duration::from_secs(details.interval().as_secs().max(1));
        let deadline = Instant::now() + Duration::from_secs(details.expires_in().as_secs());

        loop {
            if Instant::now() >= deadline {
                return Err(AuthError::Expired);
            }
            let resp = self
                .http
                .post(&token_url)
                .header(reqwest::header::ACCEPT, "application/json")
                .form(&[
                    ("client_id", client_id.as_str()),
                    ("device_code", device_code.as_str()),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ])
                .send()
                .await?;

            let status = resp.status();
            let body = resp.text().await?;
            let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
                AuthError::OAuth(format!(
                    "non-JSON token response (status {status}): {e}: {body}"
                ))
            })?;

            if let Some(err) = parsed.get("error").and_then(|v| v.as_str()) {
                match err {
                    "authorization_pending" => {
                        tokio::time::sleep(interval).await;
                        continue;
                    }
                    "slow_down" => {
                        interval += Duration::from_secs(5);
                        tokio::time::sleep(interval).await;
                        continue;
                    }
                    "expired_token" => return Err(AuthError::Expired),
                    "access_denied" => return Err(AuthError::Cancelled),
                    other => {
                        let desc = parsed
                            .get("error_description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        return Err(AuthError::OAuth(format!("{other}: {desc}")));
                    }
                }
            }

            let access = parsed
                .get("access_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    AuthError::OAuth(format!("missing access_token in response: {body}"))
                })?
                .to_string();
            let refresh = parsed
                .get("refresh_token")
                .and_then(|v| v.as_str())
                .map(|s| Zeroizing::new(s.to_string()));

            tracing::info!("device_flow_complete");
            return Ok(Tokens {
                access: Zeroizing::new(access),
                refresh,
            });
        }
    }

    /// Fetch the authenticated user's GitHub login via the REST API. Uses the
    /// crate's reqwest client directly to avoid octocrab/tower's buffer
    /// service, which has panicked in the field on this single call site.
    pub async fn fetch_user_login(
        &self,
        access_token: &Zeroizing<String>,
    ) -> Result<String, AuthError> {
        let resp = self
            .http
            .get(format!("{}/user", self.api_base_url))
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", access_token.as_str()),
            )
            .header(reqwest::header::USER_AGENT, "Vector/0.1")
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(AuthError::OAuth(format!(
                "/user fetch failed (status {status}): {body}"
            )));
        }
        let parsed: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| AuthError::OAuth(format!("/user non-JSON response: {e}: {body}")))?;
        parsed
            .get("login")
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string)
            .ok_or_else(|| AuthError::OAuth(format!("/user response missing login: {body}")))
    }

    /// Direct GET /user/codespaces using reqwest — bypasses octocrab/tower so
    /// callers don't need to build an `Octocrab` (which spawns a buffer worker
    /// requiring an entered tokio runtime). Returns:
    ///   Ok(list)            — token valid, list parsed (may be empty)
    ///   Err(Unauthorized)   — 401 or 403 (insufficient scope) — caller re-auth
    ///   Err(other AuthError) — network / parse failure
    pub async fn list_codespaces_direct(
        &self,
        access_token: &Zeroizing<String>,
    ) -> Result<Vec<crate::model::Codespace>, AuthError> {
        #[derive(serde::Deserialize)]
        struct Page {
            #[serde(default)]
            total_count: u64,
            codespaces: Vec<crate::model::Codespace>,
        }
        tracing::info!("list_codespaces_direct: GET /user/codespaces");
        let resp = self
            .http
            .get(format!(
                "{}/user/codespaces?per_page=100",
                self.api_base_url
            ))
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", access_token.as_str()),
            )
            .header(reqwest::header::USER_AGENT, "Vector/0.1")
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .send()
            .await?;
        let status = resp.status();
        // Echo the granted scopes so we can confirm `codespace` was granted.
        let scopes = resp
            .headers()
            .get("x-oauth-scopes")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("<none>")
            .to_string();
        tracing::info!(status = %status, scopes = %scopes, "list_codespaces_direct: response headers");
        if status.as_u16() == 401 {
            tracing::warn!("list_codespaces_direct: 401 — token rejected");
            return Err(AuthError::Unauthorized);
        }
        if status.as_u16() == 403 {
            tracing::warn!(scopes = %scopes, "list_codespaces_direct: 403 — likely missing `codespace` scope");
            return Err(AuthError::Unauthorized);
        }
        let body = resp.text().await?;
        tracing::info!(
            body_len = body.len(),
            "list_codespaces_direct: body received"
        );
        if !status.is_success() {
            return Err(AuthError::OAuth(format!(
                "/user/codespaces failed (status {status}): {body}"
            )));
        }
        let page: Page = serde_json::from_str(&body).map_err(|e| {
            AuthError::OAuth(format!("/user/codespaces non-JSON response: {e}: {body}"))
        })?;
        tracing::info!(
            total_count = page.total_count,
            parsed_count = page.codespaces.len(),
            "list_codespaces_direct: parsed"
        );
        // If GitHub says there are codespaces (total_count > 0) but we parsed
        // zero, it means individual rows failed serde — surface as an error.
        if page.total_count > 0 && page.codespaces.is_empty() {
            return Err(AuthError::OAuth(format!(
                "/user/codespaces total_count={} but parsed 0 — schema drift? body: {body}",
                page.total_count
            )));
        }
        Ok(page.codespaces)
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
