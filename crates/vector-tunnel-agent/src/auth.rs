//! Agent-side OAuth Device Flow (RFC 8628).
//! Two providers (GitHub + Microsoft); pick at first run via stdin prompt.
//! Token persisted via `token_cache` (mode 0600).
//! Pitfall 14: manual Debug on every token-bearing struct.

use std::io::{self, BufRead, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::token_cache::{self, CachedToken, Provider};

// GitHub endpoints — public per gh CLI; reused per D-89.
pub const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
pub const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
// D-4: bumped to Dev Tunnels GitHub App per 09.2-01 spike — relay rejected gh-CLI token.
pub const GITHUB_CLIENT_ID: &str = "Iv1.e7b89e013f801f03";
// GitHub Apps ignore OAuth scopes — no scope param (matches 09.2-02 driver).
pub const GITHUB_SCOPES: &str = "";

// Microsoft endpoints — `common` authority, multi-tenant (D-04).
pub const MICROSOFT_DEVICE_CODE_URL: &str =
    "https://login.microsoftonline.com/common/oauth2/v2.0/devicecode";
pub const MICROSOFT_TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
// Public Microsoft client for Dev Tunnels (matches VS Code / dev-tunnels SDK).
pub const MICROSOFT_CLIENT_ID: &str = "46da2f7e-b5ef-422a-9a4e-fb5e1cb7da14";
pub const MICROSOFT_SCOPES: &str = "46da2f7e-b5ef-422a-9a4e-fb5e1cb7da14/.default offline_access";

#[derive(thiserror::Error)]
pub enum AgentAuthError {
    #[error("network: {0}")]
    Network(#[from] reqwest::Error),
    #[error("oauth: {0}")]
    OAuth(String),
    #[error("device code expired before user completed sign-in")]
    DeviceCodeExpired,
    #[error("access denied by user or provider")]
    AccessDenied,
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("token cache: {0}")]
    Cache(#[from] token_cache::AgentTokenError),
}

// Manual Debug — derive(Debug) within 30 lines of a `device_code:` field trips
// Pitfall-14 arch-lint even though no token material is in scope here.
impl std::fmt::Debug for AgentAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

// User-visible payload — verification URL + code (RFC 8628 §3.1 public).
// `pub` so integration tests can drive `poll_token` (D-08: async tests live in tests/).
pub struct DeviceCodeReply {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval_secs: u64,
    pub expires_in_secs: u64,
}

impl std::fmt::Debug for DeviceCodeReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceCodeReply")
            .field("verification_uri", &self.verification_uri)
            .field("interval_secs", &self.interval_secs)
            .field("expires_in_secs", &self.expires_in_secs)
            .finish_non_exhaustive()
    }
}

/// Drive device flow for the user-chosen provider; persist on success.
///
/// On stdin: prompt for provider selection (G/M). Print device code + URL.
/// Poll the token endpoint until success, then write to disk via `token_cache::save`.
pub async fn run_first_run_device_flow() -> Result<CachedToken, AgentAuthError> {
    let provider = prompt_provider()?;
    let token = match provider {
        Provider::GitHub => drive_device_flow_github().await?,
        Provider::Microsoft => drive_device_flow_microsoft().await?,
    };
    token_cache::save(&token)?;
    Ok(token)
}

fn prompt_provider() -> Result<Provider, AgentAuthError> {
    let mut stdout = io::stdout();
    write!(stdout, "Sign in with [G]itHub or [M]icrosoft? ")?;
    stdout.flush()?;
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    match line.trim().chars().next() {
        Some('m' | 'M') => Ok(Provider::Microsoft),
        _ => Ok(Provider::GitHub),
    }
}

async fn drive_device_flow_github() -> Result<CachedToken, AgentAuthError> {
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let reply = request_device_code(
        &http,
        GITHUB_DEVICE_CODE_URL,
        GITHUB_CLIENT_ID,
        GITHUB_SCOPES,
    )
    .await?;
    print_user_prompt(&reply);
    let (access, refresh, expires_in) =
        poll_token(&http, GITHUB_TOKEN_URL, GITHUB_CLIENT_ID, &reply).await?;
    Ok(CachedToken {
        provider: Provider::GitHub,
        access_token: access,
        refresh_token: refresh,
        expires_at_unix: now_unix().saturating_add(expires_in),
    })
}

async fn drive_device_flow_microsoft() -> Result<CachedToken, AgentAuthError> {
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let reply = request_device_code(
        &http,
        MICROSOFT_DEVICE_CODE_URL,
        MICROSOFT_CLIENT_ID,
        MICROSOFT_SCOPES,
    )
    .await?;
    print_user_prompt(&reply);
    let (access, refresh, expires_in) =
        poll_token(&http, MICROSOFT_TOKEN_URL, MICROSOFT_CLIENT_ID, &reply).await?;
    Ok(CachedToken {
        provider: Provider::Microsoft,
        access_token: access,
        refresh_token: refresh,
        expires_at_unix: now_unix().saturating_add(expires_in),
    })
}

async fn request_device_code(
    http: &reqwest::Client,
    url: &str,
    client_id: &str,
    scopes: &str,
) -> Result<DeviceCodeReply, AgentAuthError> {
    // Omit scope when empty — GitHub Apps ignore scopes (D-4 / 09.2-02 pattern).
    let mut form: Vec<(&str, &str)> = vec![("client_id", client_id)];
    if !scopes.is_empty() {
        form.push(("scope", scopes));
    }
    let resp = http
        .post(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&form)
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await?;
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        AgentAuthError::OAuth(format!(
            "non-JSON device-code response (status {status}): {e}: {body}"
        ))
    })?;
    if let Some(err) = v.get("error").and_then(|v| v.as_str()) {
        return Err(AgentAuthError::OAuth(format!(
            "{err}: {}",
            v.get("error_description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
        )));
    }
    let device_code = v
        .get("device_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AgentAuthError::OAuth(format!("missing device_code: {body}")))?
        .to_string();
    let user_code = v
        .get("user_code")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let verification_uri = v
        .get("verification_uri")
        .or_else(|| v.get("verification_url"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let interval_secs = v
        .get("interval")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(5);
    let expires_in_secs = v
        .get("expires_in")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(900);
    Ok(DeviceCodeReply {
        device_code,
        user_code,
        verification_uri,
        interval_secs,
        expires_in_secs,
    })
}

fn print_user_prompt(reply: &DeviceCodeReply) {
    let mins = reply.expires_in_secs / 60;
    let secs = reply.expires_in_secs % 60;
    println!(
        "To sign in, open {} in a browser and enter:\n\n    {}\n\nWaiting for sign-in… (code expires in {:02}:{:02})",
        reply.verification_uri, reply.user_code, mins, secs
    );
}

/// Poll the device-code token endpoint until success. Returns
/// `(access_token, refresh_token, expires_in_secs)`. `pub` for integration tests.
pub async fn poll_token(
    http: &reqwest::Client,
    token_url: &str,
    client_id: &str,
    reply: &DeviceCodeReply,
) -> Result<(String, Option<String>, u64), AgentAuthError> {
    // GitHub intermittently returns `incorrect_device_code` on a poll issued before
    // the just-issued code has propagated. Tolerate a bounded number as transient.
    const MAX_INCORRECT_CODE_RETRIES: u32 = 6;
    let mut interval = Duration::from_secs(reply.interval_secs.max(1));
    let deadline = SystemTime::now() + Duration::from_secs(reply.expires_in_secs);
    let mut incorrect_code_retries: u32 = 0;
    loop {
        if SystemTime::now() >= deadline {
            return Err(AgentAuthError::DeviceCodeExpired);
        }
        // RFC 8628 §3.4: wait `interval` before every poll, including the first —
        // polling at t=0 races GitHub's code propagation (→ incorrect_device_code).
        tokio::time::sleep(interval).await;
        let resp = http
            .post(token_url)
            .header(reqwest::header::ACCEPT, "application/json")
            .form(&[
                ("client_id", client_id),
                ("device_code", reply.device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
            AgentAuthError::OAuth(format!(
                "non-JSON token response (status {status}): {e}: {body}"
            ))
        })?;
        if let Some(err) = v.get("error").and_then(|v| v.as_str()) {
            match err {
                "authorization_pending" => continue,
                "slow_down" => {
                    interval += Duration::from_secs(5);
                    continue;
                }
                "incorrect_device_code" if incorrect_code_retries < MAX_INCORRECT_CODE_RETRIES => {
                    incorrect_code_retries += 1;
                    continue;
                }
                "expired_token" => return Err(AgentAuthError::DeviceCodeExpired),
                "access_denied" => return Err(AgentAuthError::AccessDenied),
                other => {
                    let desc = v
                        .get("error_description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    return Err(AgentAuthError::OAuth(format!("{other}: {desc}")));
                }
            }
        }
        let access = v
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentAuthError::OAuth(format!("missing access_token: {body}")))?
            .to_string();
        let refresh = v
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string);
        let expires_in = v
            .get("expires_in")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(3600);
        return Ok((access, refresh, expires_in));
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
