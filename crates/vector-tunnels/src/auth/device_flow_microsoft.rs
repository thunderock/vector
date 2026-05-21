//! Microsoft OAuth Device Flow (RFC 8628) driver against the `common` authority.
//! Mirrors `vector-codespaces::auth::device_flow::GitHubAuth` one-to-one.
//!
//! Pitfall 14: every token-bearing struct has a hand-written Debug impl —
//! NEVER derive Debug here.

use std::time::{Duration, SystemTime};

use tokio_util::sync::CancellationToken;

use crate::auth::error::MicrosoftAuthError;

pub const MICROSOFT_DEVICE_CODE_ENDPOINT: &str =
    "https://login.microsoftonline.com/common/oauth2/v2.0/devicecode";
pub const MICROSOFT_TOKEN_ENDPOINT: &str =
    "https://login.microsoftonline.com/common/oauth2/v2.0/token";
pub const MICROSOFT_TUNNELS_SCOPE: &str = "46da2f7e-b5ef-422a-9a4e-fb5e1cb7da14/.default";

// VS Code's public-multi-tenant client ID (Microsoft Authentication Library
// public client). v1 piggybacks on VS Code's app registration — same trick
// `vector-codespaces` does with `gh` CLI's client ID (D-89).
pub const DEFAULT_MICROSOFT_CLIENT_ID: &str = "aebc6443-996d-45c2-90f0-388ff96faa56";

const MAX_POLL_INTERVAL_SECS: u64 = 60;

/// Endpoint override seam for tests.
struct EndpointsOverride {
    device_code: String,
    token: String,
    scope: String,
}

pub struct MicrosoftAuth {
    http: reqwest::Client,
    client_id: String,
    endpoints: EndpointsOverride,
}

impl std::fmt::Debug for MicrosoftAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MicrosoftAuth")
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

pub struct MicrosoftTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: SystemTime,
}

impl std::fmt::Debug for MicrosoftTokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MicrosoftTokens")
            .field("access_token_len", &self.access_token.len())
            .field("has_refresh", &self.refresh_token.is_some())
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

impl MicrosoftAuth {
    pub fn new(client_id: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            client_id: client_id.into(),
            endpoints: EndpointsOverride {
                device_code: MICROSOFT_DEVICE_CODE_ENDPOINT.to_string(),
                token: MICROSOFT_TOKEN_ENDPOINT.to_string(),
                scope: MICROSOFT_TUNNELS_SCOPE.to_string(),
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

    pub async fn start_device_flow(&self) -> Result<DeviceFlowStart, MicrosoftAuthError> {
        tracing::info!("microsoft_device_flow_initiated");
        let resp = self
            .http
            .post(&self.endpoints.device_code)
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("scope", self.endpoints.scope.as_str()),
            ])
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            MicrosoftAuthError::Unexpected(format!(
                "non-JSON devicecode response (status {status}): {e}: {body}"
            ))
        })?;
        let device_code = v
            .get("device_code")
            .and_then(|s| s.as_str())
            .ok_or_else(|| MicrosoftAuthError::Unexpected(format!("missing device_code: {body}")))?
            .to_string();
        let user_code = v
            .get("user_code")
            .and_then(|s| s.as_str())
            .ok_or_else(|| MicrosoftAuthError::Unexpected(format!("missing user_code: {body}")))?
            .to_string();
        let verification_uri = v
            .get("verification_uri")
            .and_then(|s| s.as_str())
            .ok_or_else(|| {
                MicrosoftAuthError::Unexpected(format!("missing verification_uri: {body}"))
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
    ) -> Result<MicrosoftTokens, MicrosoftAuthError> {
        tracing::info!("microsoft_device_flow_polling");
        let started = std::time::Instant::now();
        let mut current_interval = interval.max(Duration::from_millis(1));
        let interval_cap = Duration::from_secs(MAX_POLL_INTERVAL_SECS);

        loop {
            if cancel.is_cancelled() {
                return Err(MicrosoftAuthError::Cancelled);
            }
            if started.elapsed() >= expires_in {
                return Err(MicrosoftAuthError::DeviceCodeExpired);
            }

            let resp = self
                .http
                .post(&self.endpoints.token)
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
                MicrosoftAuthError::Unexpected(format!(
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
                        // Double the interval, capped at MAX_POLL_INTERVAL_SECS.
                        current_interval = (current_interval * 2).min(interval_cap);
                        sleep_or_cancel(current_interval, &cancel).await?;
                        continue;
                    }
                    "expired_token" => return Err(MicrosoftAuthError::DeviceCodeExpired),
                    "access_denied" => return Err(MicrosoftAuthError::AccessDenied),
                    other => {
                        let desc = parsed
                            .get("error_description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        return Err(MicrosoftAuthError::Unexpected(format!("{other}: {desc}")));
                    }
                }
            }

            return parse_token_response(&parsed, &body);
        }
    }

    pub async fn refresh(
        &self,
        refresh_token: &str,
    ) -> Result<MicrosoftTokens, MicrosoftAuthError> {
        tracing::info!("microsoft_token_refresh_attempted");
        let resp = self
            .http
            .post(&self.endpoints.token)
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
                ("scope", self.endpoints.scope.as_str()),
            ])
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            MicrosoftAuthError::Unexpected(format!(
                "non-JSON refresh response (status {status}): {e}: {body}"
            ))
        })?;
        if let Some(err) = parsed.get("error").and_then(|v| v.as_str()) {
            return match err {
                "invalid_grant" => Err(MicrosoftAuthError::RefreshExpired),
                other => {
                    let desc = parsed
                        .get("error_description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    Err(MicrosoftAuthError::Unexpected(format!("{other}: {desc}")))
                }
            };
        }
        parse_token_response(&parsed, &body)
    }
}

async fn sleep_or_cancel(
    interval: Duration,
    cancel: &CancellationToken,
) -> Result<(), MicrosoftAuthError> {
    tokio::select! {
        () = tokio::time::sleep(interval) => Ok(()),
        () = cancel.cancelled() => Err(MicrosoftAuthError::Cancelled),
    }
}

fn parse_token_response(
    parsed: &serde_json::Value,
    body: &str,
) -> Result<MicrosoftTokens, MicrosoftAuthError> {
    let access_token = parsed
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| MicrosoftAuthError::Unexpected(format!("missing access_token: {body}")))?
        .to_string();
    let refresh_token = parsed
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(std::string::ToString::to_string);
    let expires_in_secs = parsed
        .get("expires_in")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(3600);
    let expires_at = SystemTime::now() + Duration::from_secs(expires_in_secs);
    tracing::info!("microsoft_device_flow_complete");
    Ok(MicrosoftTokens {
        access_token,
        refresh_token,
        expires_at,
    })
}
