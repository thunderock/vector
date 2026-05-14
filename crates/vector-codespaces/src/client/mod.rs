//! Codespaces REST client. Plan 06-03 fills in list/get/start/poll.
use octocrab::Octocrab;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("octocrab error: {0}")]
    Octocrab(#[from] octocrab::Error),
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("serde: {0}")]
    Json(#[from] serde_json::Error),
    #[error("start failed: status {status}")]
    StartFailed { status: u16 },
    #[error("unauthenticated (401 after refresh)")]
    Unauthenticated,
    #[error("poll timeout")]
    PollTimeout,
    #[error("cancelled")]
    Cancelled,
}

pub struct CodespacesClient {
    #[allow(dead_code)]
    octocrab: Arc<Octocrab>,
}

impl std::fmt::Debug for CodespacesClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodespacesClient").finish_non_exhaustive()
    }
}

impl CodespacesClient {
    pub fn new(octocrab: Arc<Octocrab>) -> Self {
        Self { octocrab }
    }

    pub async fn list(&self) -> Result<Vec<crate::model::Codespace>, ClientError> {
        unimplemented!("Plan 06-03")
    }

    pub async fn get(&self, _name: &str) -> Result<crate::model::Codespace, ClientError> {
        unimplemented!("Plan 06-03")
    }

    pub async fn start(&self, _name: &str) -> Result<(), ClientError> {
        unimplemented!("Plan 06-03")
    }
}
