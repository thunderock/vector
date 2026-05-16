//! GitHub Codespaces REST client. Pitfall 2: `Arc<Octocrab>` swapped under
//! `RwLock` so token refresh never strands existing call sites.
//!
//! Pitfall 14: NEVER `#[derive(Debug)]` on structs holding access/refresh
//! tokens. Manual `Debug` impls live below.

use std::sync::Arc;
use std::time::{Duration, Instant};

use octocrab::Octocrab;
use parking_lot::RwLock;
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

use crate::model::{Codespace, CodespaceState};

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("octocrab error: {0}")]
    Octocrab(#[from] octocrab::Error),
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("serde: {0}")]
    Json(#[from] serde_json::Error),
    #[error("auth: {0}")]
    Auth(#[from] crate::auth::AuthError),
    #[error("octocrab builder: {0}")]
    Builder(String),
    #[error("start failed: status {status}")]
    StartFailed { status: u16 },
    #[error("unauthenticated (401 after refresh)")]
    Unauthenticated,
    #[error("poll timeout")]
    PollTimeout,
    #[error("cancelled")]
    Cancelled,
}

#[derive(serde::Deserialize)]
struct CodespacesPage {
    #[allow(dead_code)]
    total_count: u32,
    codespaces: Vec<Codespace>,
}

/// Build an `Arc<Octocrab>` with Vector's User-Agent/Accept headers and an
/// optional base URI override (used by tests against wiremock).
pub fn build_octocrab(
    access_token: &Zeroizing<String>,
    base_uri: Option<&str>,
) -> Result<Arc<Octocrab>, ClientError> {
    let mut builder = Octocrab::builder()
        .personal_token((**access_token).clone())
        .add_header(http::header::ACCEPT, "application/vnd.github+json".into())
        .add_header(http::header::USER_AGENT, "Vector/0.1".into());
    if let Some(uri) = base_uri {
        builder = builder
            .base_uri(uri)
            .map_err(|e| ClientError::Builder(e.to_string()))?;
    }
    let octo = builder
        .build()
        .map_err(|e| ClientError::Builder(e.to_string()))?;
    Ok(Arc::new(octo))
}

/// Minimal refresh-token POST against GitHub's OAuth endpoint (Pattern 2
/// from RESEARCH.md §"401 → Silent Refresh"). Lives inside the client so we
/// don't couple to Plan 06-02's `GitHubAuth` surface during parallel waves.
struct RefreshContext {
    refresh_token: Zeroizing<String>,
    refresh_endpoint: String,
    base_uri: Option<String>,
    http: reqwest::Client,
}

// Pitfall 14: manual Debug — never reveal token material.
impl std::fmt::Debug for RefreshContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RefreshContext")
            .field("refresh_endpoint", &self.refresh_endpoint)
            .field("base_uri", &self.base_uri)
            .finish_non_exhaustive()
    }
}

#[derive(serde::Deserialize)]
struct RefreshResponse {
    access_token: String,
}

impl RefreshContext {
    async fn refresh(&self) -> Result<Zeroizing<String>, ClientError> {
        let resp = self
            .http
            .post(&self.refresh_endpoint)
            .header(http::header::ACCEPT, "application/json")
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", self.refresh_token.as_str()),
            ])
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(ClientError::Unauthenticated);
        }
        let body: RefreshResponse = resp.json().await?;
        Ok(Zeroizing::new(body.access_token))
    }
}

pub struct CodespacesClient {
    inner: Arc<RwLock<Arc<Octocrab>>>,
    refresh: Option<RefreshContext>,
}

// Pitfall 14: manual Debug.
impl std::fmt::Debug for CodespacesClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodespacesClient")
            .field("has_refresh", &self.refresh.is_some())
            .finish_non_exhaustive()
    }
}

impl CodespacesClient {
    pub fn new(octocrab: Arc<Octocrab>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(octocrab)),
            refresh: None,
        }
    }

    /// Test-only constructor wiring an inline refresh endpoint. Production
    /// path will use `new_with_refresh` once Plan 06-02 lands.
    pub fn new_for_test(
        base_uri: &str,
        access: String,
        refresh: Option<String>,
        refresh_endpoint: &str,
    ) -> Result<Self, ClientError> {
        let octo = build_octocrab(&Zeroizing::new(access), Some(base_uri))?;
        let refresh_ctx = refresh.map(|r| RefreshContext {
            refresh_token: Zeroizing::new(r),
            refresh_endpoint: refresh_endpoint.to_string(),
            base_uri: Some(base_uri.to_string()),
            http: reqwest::Client::new(),
        });
        Ok(Self {
            inner: Arc::new(RwLock::new(octo)),
            refresh: refresh_ctx,
        })
    }

    fn octo(&self) -> Arc<Octocrab> {
        self.inner.read().clone()
    }

    pub async fn list(&self) -> Result<Vec<Codespace>, ClientError> {
        let octo = self.octo();
        let resp = octo._get("/user/codespaces?per_page=100").await?;
        if resp.status().as_u16() == 401 {
            return Err(ClientError::Unauthenticated);
        }
        let body = octo.body_to_string(resp).await?;
        let page: CodespacesPage = serde_json::from_str(&body)?;
        Ok(page.codespaces)
    }

    pub async fn get(&self, name: &str) -> Result<Codespace, ClientError> {
        let octo = self.octo();
        let path = format!("/user/codespaces/{}", urlencoding::encode(name));
        let resp = octo._get(path).await?;
        if resp.status().as_u16() == 401 {
            return Err(ClientError::Unauthenticated);
        }
        let body = octo.body_to_string(resp).await?;
        let cs: Codespace = serde_json::from_str(&body)?;
        Ok(cs)
    }

    /// POST /user/codespaces/{name}/start — Pitfall 5: treat 200/202/409 as
    /// success (409 = already starting).
    pub async fn start(&self, name: &str) -> Result<(), ClientError> {
        let octo = self.octo();
        let path = format!("/user/codespaces/{}/start", urlencoding::encode(name));
        let resp = octo._post(path, None::<&()>).await?;
        match resp.status().as_u16() {
            200 | 202 | 409 => Ok(()),
            401 => Err(ClientError::Unauthenticated),
            s => Err(ClientError::StartFailed { status: s }),
        }
    }

    /// Poll `get(name)` once per second until state is terminal
    /// (Available / Failed / Shutdown), the cancel token fires, or `deadline`
    /// elapses.
    pub async fn poll_until_available(
        &self,
        name: &str,
        cancel: CancellationToken,
        on_state: impl Fn(CodespaceState) + Send,
        deadline: Duration,
    ) -> Result<CodespaceState, ClientError> {
        let started = Instant::now();
        loop {
            tokio::select! {
                () = cancel.cancelled() => return Err(ClientError::Cancelled),
                () = tokio::time::sleep(Duration::from_secs(1)) => {}
            }
            if started.elapsed() >= deadline {
                return Err(ClientError::PollTimeout);
            }
            let cs = self.get(name).await?;
            on_state(cs.state);
            if matches!(
                cs.state,
                CodespaceState::Available | CodespaceState::Failed | CodespaceState::Shutdown
            ) {
                return Ok(cs.state);
            }
        }
    }

    /// AUTH-03 chain: `list`, on 401 refresh + retry once, on still-401
    /// emit `Unauthenticated`.
    pub async fn list_with_refresh(&self) -> Result<Vec<Codespace>, ClientError> {
        match self.list().await {
            Ok(v) => return Ok(v),
            Err(ClientError::Unauthenticated) => {}
            Err(e) => return Err(e),
        }
        let Some(ctx) = self.refresh.as_ref() else {
            return Err(ClientError::Unauthenticated);
        };
        tracing::info!("token_refresh_attempted");
        let Ok(new_access) = ctx.refresh().await else {
            tracing::warn!("token_refresh_failed");
            return Err(ClientError::Unauthenticated);
        };
        let new_octo = build_octocrab(&new_access, ctx.base_uri.as_deref())?;
        *self.inner.write() = new_octo;
        match self.list().await {
            Ok(v) => Ok(v),
            Err(ClientError::Unauthenticated) => Err(ClientError::Unauthenticated),
            Err(e) => Err(e),
        }
    }
}
