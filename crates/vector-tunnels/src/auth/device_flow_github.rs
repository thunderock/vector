//! GitHub OAuth Device Flow (RFC 8628) driver against the Dev Tunnels GitHub App.
//! Two GitHub-specific divergences from the generic OAuth device flow:
//!   1. `Accept: application/json` on BOTH POSTs (GitHub defaults to form-encoded).
//!   2. `slow_down` ADDS 5s to the interval — it does NOT double it.
//!
//! Pitfall 14: every token-bearing struct has a hand-written Debug impl —
//! NEVER derive Debug here.

use std::time::{Duration, SystemTime};

use tokio_util::sync::CancellationToken;

use crate::auth::error::GitHubAuthError;

pub const GITHUB_DEVICE_CODE_ENDPOINT: &str = "https://github.com/login/device/code";
pub const GITHUB_TOKEN_ENDPOINT: &str = "https://github.com/login/oauth/access_token";

// Dev Tunnels GitHub App client ID. Spike-validated (09.2-01): device flow is
// enabled and its token is accepted by `list_tunnels` (200) with a refresh_token
// + 8h expiry. The gh-CLI ID is rejected (403) by the relay.
pub const GITHUB_DEVTUNNELS_CLIENT_ID: &str = "Iv1.e7b89e013f801f03";

const MAX_POLL_INTERVAL_SECS: u64 = 60;

// Sentinel expiry for tokens that arrive without `expires_in`: ~100 years out
// so a non-expiring token is never treated as stale (Pitfall 4 — NOT
// `unwrap_or(3600)`). The Dev Tunnels GitHub App DOES issue `expires_in` (8h),
// so this branch is a safety net, not the live path.
const NO_EXPIRY_SENTINEL_SECS: u64 = 100 * 365 * 24 * 3600;

/// Endpoint override seam for tests.
struct EndpointsOverride {
    device_code: String,
    token: String,
    scope: String,
}

pub struct GitHubAuth {
    http: reqwest::Client,
    client_id: String,
    endpoints: EndpointsOverride,
}

impl std::fmt::Debug for GitHubAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitHubAuth")
            .field("client_id", &self.client_id)
            .finish_non_exhaustive()
    }
}

pub struct DeviceFlowStart {
    pub device_code: String, // SECRET — never derive Debug
    pub user_code: String,
    pub verification_uri: String,
    pub interval: Duration,
    pub expires_in: Duration,
}

impl std::fmt::Debug for DeviceFlowStart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceFlowStart")
            .field("user_code", &self.user_code)
            .field("verification_uri", &self.verification_uri)
            .field("interval", &self.interval)
            .field("expires_in", &self.expires_in)
            // device_code intentionally omitted (Pitfall 14)
            .finish_non_exhaustive()
    }
}

pub struct GitHubTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: SystemTime,
}

impl std::fmt::Debug for GitHubTokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitHubTokens")
            .field("access_token_len", &self.access_token.len())
            .field("has_refresh", &self.refresh_token.is_some())
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

impl GitHubAuth {
    pub fn new(client_id: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            client_id: client_id.into(),
            endpoints: EndpointsOverride {
                device_code: GITHUB_DEVICE_CODE_ENDPOINT.to_string(),
                token: GITHUB_TOKEN_ENDPOINT.to_string(),
                // GitHub Apps ignore OAuth scopes — send none (spike-validated).
                scope: String::new(),
            },
        }
    }

    /// Test-only constructor — swaps device + token endpoints for wiremock URLs.
    pub fn with_endpoints(
        client_id: impl Into<String>,
        device_endpoint: impl Into<String>,
        token_endpoint: impl Into<String>,
        scope: impl Into<String>,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            client_id: client_id.into(),
            endpoints: EndpointsOverride {
                device_code: device_endpoint.into(),
                token: token_endpoint.into(),
                scope: scope.into(),
            },
        }
    }

    pub async fn start_device_flow(&self) -> Result<DeviceFlowStart, GitHubAuthError> {
        tracing::info!("github_device_flow_initiated");
        let mut form: Vec<(&str, &str)> = vec![("client_id", self.client_id.as_str())];
        if !self.endpoints.scope.is_empty() {
            form.push(("scope", self.endpoints.scope.as_str()));
        }
        let resp = self
            .http
            .post(&self.endpoints.device_code)
            .header(reqwest::header::ACCEPT, "application/json")
            .form(&form)
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            GitHubAuthError::Unexpected(format!(
                "non-JSON devicecode response (status {status}): {e}: {body}"
            ))
        })?;
        if let Some(err) = v.get("error").and_then(|v| v.as_str()) {
            if err == "device_flow_disabled" {
                return Err(GitHubAuthError::DeviceFlowDisabled);
            }
            let desc = v
                .get("error_description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            return Err(GitHubAuthError::Unexpected(format!("{err}: {desc}")));
        }
        let device_code = v
            .get("device_code")
            .and_then(|s| s.as_str())
            .ok_or_else(|| GitHubAuthError::Unexpected(format!("missing device_code: {body}")))?
            .to_string();
        let user_code = v
            .get("user_code")
            .and_then(|s| s.as_str())
            .ok_or_else(|| GitHubAuthError::Unexpected(format!("missing user_code: {body}")))?
            .to_string();
        // GitHub returns `verification_uri`; defend against `verification_url`.
        let verification_uri = v
            .get("verification_uri")
            .or_else(|| v.get("verification_url"))
            .and_then(|s| s.as_str())
            .ok_or_else(|| {
                GitHubAuthError::Unexpected(format!("missing verification_uri: {body}"))
            })?
            .to_string();
        let interval_secs = v
            .get("interval")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(5);
        let expires_secs = v
            .get("expires_in")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(900);
        Ok(DeviceFlowStart {
            device_code,
            user_code,
            verification_uri,
            interval: Duration::from_secs(interval_secs.max(1)),
            expires_in: Duration::from_secs(expires_secs),
        })
    }

    pub async fn poll_until_authorized(
        &self,
        device_code: &str,
        interval: Duration,
        expires_in: Duration,
        cancel: CancellationToken,
    ) -> Result<GitHubTokens, GitHubAuthError> {
        tracing::info!("github_device_flow_polling");
        let started = std::time::Instant::now();
        let mut current_interval = interval.max(Duration::from_millis(1));
        let interval_cap = Duration::from_secs(MAX_POLL_INTERVAL_SECS);

        loop {
            if cancel.is_cancelled() {
                return Err(GitHubAuthError::Cancelled);
            }
            if started.elapsed() >= expires_in {
                return Err(GitHubAuthError::DeviceCodeExpired);
            }

            let resp = self
                .http
                .post(&self.endpoints.token)
                .header(reqwest::header::ACCEPT, "application/json")
                .form(&[
                    ("client_id", self.client_id.as_str()),
                    ("device_code", device_code),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ])
                .send()
                .await?;
            let status = resp.status();
            let body = resp.text().await?;
            let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
                GitHubAuthError::Unexpected(format!(
                    "non-JSON token response (status {status}): {e}: {body}"
                ))
            })?;

            if let Some(err) = parsed.get("error").and_then(|v| v.as_str()) {
                match err {
                    "authorization_pending" => {
                        sleep_or_cancel(current_interval, &cancel).await?;
                        continue;
                    }
                    "slow_down" => {
                        // GitHub: ADD 5s (not doubling), capped at MAX.
                        current_interval =
                            (current_interval + Duration::from_secs(5)).min(interval_cap);
                        sleep_or_cancel(current_interval, &cancel).await?;
                        continue;
                    }
                    "expired_token" => return Err(GitHubAuthError::DeviceCodeExpired),
                    "access_denied" => return Err(GitHubAuthError::AccessDenied),
                    "device_flow_disabled" => return Err(GitHubAuthError::DeviceFlowDisabled),
                    other => {
                        let desc = parsed
                            .get("error_description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        return Err(GitHubAuthError::Unexpected(format!("{other}: {desc}")));
                    }
                }
            }

            return parse_token_response(&parsed, &body);
        }
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<GitHubTokens, GitHubAuthError> {
        tracing::info!("github_token_refresh_attempted");
        let resp = self
            .http
            .post(&self.endpoints.token)
            .header(reqwest::header::ACCEPT, "application/json")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            GitHubAuthError::Unexpected(format!(
                "non-JSON refresh response (status {status}): {e}: {body}"
            ))
        })?;
        if let Some(err) = parsed.get("error").and_then(|v| v.as_str()) {
            return match err {
                // GitHub names the expired/invalid refresh case `bad_refresh_token`.
                "bad_refresh_token" => Err(GitHubAuthError::RefreshExpired),
                other => {
                    let desc = parsed
                        .get("error_description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    Err(GitHubAuthError::Unexpected(format!("{other}: {desc}")))
                }
            };
        }
        parse_token_response(&parsed, &body)
    }
}

async fn sleep_or_cancel(
    interval: Duration,
    cancel: &CancellationToken,
) -> Result<(), GitHubAuthError> {
    tokio::select! {
        () = tokio::time::sleep(interval) => Ok(()),
        () = cancel.cancelled() => Err(GitHubAuthError::Cancelled),
    }
}

fn parse_token_response(
    parsed: &serde_json::Value,
    body: &str,
) -> Result<GitHubTokens, GitHubAuthError> {
    let access_token = parsed
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| GitHubAuthError::Unexpected(format!("missing access_token: {body}")))?
        .to_string();
    let refresh_token = parsed
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(std::string::ToString::to_string);
    // Absent expires_in → far-future sentinel, NOT 3600 (Pitfall 4).
    let expires_in_secs = parsed
        .get("expires_in")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(NO_EXPIRY_SENTINEL_SECS);
    let expires_at = SystemTime::now() + Duration::from_secs(expires_in_secs);
    tracing::info!("github_device_flow_complete");
    Ok(GitHubTokens {
        access_token,
        refresh_token,
        expires_at,
    })
}
