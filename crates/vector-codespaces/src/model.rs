//! Codespace REST response types. Pitfall 4: #[serde(other)] Unrecognized.
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Codespace {
    pub name: String,
    pub state: CodespaceState,
    pub repository: RepositoryRef,
    pub git_status: GitStatus,
    pub last_used_at: chrono::DateTime<chrono::Utc>,
    pub display_name: Option<String>,
    // Survive GitHub adding fields:
    #[serde(flatten)]
    _rest: serde_json::Map<String, serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum CodespaceState {
    Available,
    Starting,
    ShuttingDown,
    Shutdown,
    Archived,
    Failed,
    Provisioning,
    Queued,
    Updating,
    Rebuilding,
    Unknown,
    Created,
    #[serde(other)]
    Unrecognized,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RepositoryRef {
    pub full_name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GitStatus {
    #[serde(rename = "ref")]
    pub ref_name: String,
}
