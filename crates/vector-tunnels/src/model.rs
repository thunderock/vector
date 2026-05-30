//! Dev Tunnels REST model types + provider-aware auth header (D-06, D-09, D-10).

use chrono::{DateTime, Utc};
use serde::Deserialize;

/// On-wire tunnel record. Microsoft's contract; fields named per their API.
#[derive(Deserialize, Clone)]
pub struct TunnelRecord {
    #[serde(rename = "tunnelId")]
    pub tunnel_id: String,
    pub name: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(rename = "endpoints", default)]
    pub endpoints: Vec<TunnelEndpoint>,
    #[serde(rename = "lastUpdatedAt")]
    pub last_updated_at: Option<String>,
}

// Terse, no secrets to redact but match codebase Pitfall-14 manual-Debug discipline.
impl std::fmt::Debug for TunnelRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TunnelRecord")
            .field("tunnel_id", &self.tunnel_id)
            .field("name", &self.name)
            .field("labels_n", &self.labels.len())
            .field("endpoints_n", &self.endpoints.len())
            .finish_non_exhaustive()
    }
}

impl TunnelRecord {
    pub const VECTOR_AGENT_LABEL: &str = "vector-agent";
    pub const VECTOR_NAME_PREFIX: &str = "vector-";

    /// D-10: only tunnels labelled by our agent are surfaced to the picker.
    pub fn is_vector_agent(&self) -> bool {
        self.labels.iter().any(|l| l == Self::VECTOR_AGENT_LABEL)
    }

    /// Display name for the picker. Custom tunnel names are disabled on the relay,
    /// so `name` is usually absent — fall back to the auto-assigned `tunnel_id`.
    pub fn display_name(&self) -> String {
        match self.name.as_deref().filter(|n| !n.is_empty()) {
            Some(name) => name
                .strip_prefix(Self::VECTOR_NAME_PREFIX)
                .unwrap_or(name)
                .to_string(),
            None => self.tunnel_id.clone(),
        }
    }

    /// RFC 3339 `lastUpdatedAt` → DateTime<Utc>; None if missing/garbled.
    pub fn last_updated(&self) -> Option<DateTime<Utc>> {
        self.last_updated_at
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&Utc))
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct TunnelEndpoint {
    #[serde(rename = "hostId")]
    pub host_id: String,
    #[serde(rename = "clientRelayUri")]
    pub client_relay_uri: String,
    #[serde(rename = "hostPublicKeys", default)]
    pub host_public_keys: Vec<String>,
}

/// Provider tag — drives Authorization header format per D-06.
pub enum AuthProvider {
    GitHub(String),
    Microsoft(String),
}

impl AuthProvider {
    /// D-06: GitHub PAT uses non-standard `github <token>` scheme; Microsoft uses Bearer.
    pub fn format_header(&self) -> String {
        match self {
            Self::GitHub(t) => format!("github {t}"),
            Self::Microsoft(t) => format!("Bearer {t}"),
        }
    }
}

// Manual Debug — never leak token bytes (Pitfall 14).
impl std::fmt::Debug for AuthProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GitHub(_) => f.write_str("AuthProvider::GitHub(<token>)"),
            Self::Microsoft(_) => f.write_str("AuthProvider::Microsoft(<token>)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_strips_vector_prefix() {
        let t = TunnelRecord {
            tunnel_id: "tid".into(),
            name: Some("vector-corp-dev-box-42".into()),
            labels: vec![],
            endpoints: vec![],
            last_updated_at: None,
        };
        assert_eq!(t.display_name(), "corp-dev-box-42");
    }

    #[test]
    fn display_name_handles_bare_prefix() {
        let t = TunnelRecord {
            tunnel_id: "tid".into(),
            name: Some("vector-".into()),
            labels: vec![],
            endpoints: vec![],
            last_updated_at: None,
        };
        assert_eq!(t.display_name(), "");
    }

    #[test]
    fn display_name_passes_through_when_missing_prefix() {
        let t = TunnelRecord {
            tunnel_id: "tid".into(),
            name: Some("plain-host".into()),
            labels: vec![],
            endpoints: vec![],
            last_updated_at: None,
        };
        assert_eq!(t.display_name(), "plain-host");
    }

    #[test]
    fn auth_header_github() {
        let h = AuthProvider::GitHub("gho_xxx".into()).format_header();
        assert_eq!(h, "github gho_xxx");
    }

    #[test]
    fn auth_header_microsoft() {
        let h = AuthProvider::Microsoft("jwt".into()).format_header();
        assert_eq!(h, "Bearer jwt");
    }

    #[test]
    fn auth_debug_never_leaks_token() {
        let g = AuthProvider::GitHub("gho_secret".into());
        let m = AuthProvider::Microsoft("eyJ.secret.jwt".into());
        let gs = format!("{g:?}");
        let ms = format!("{m:?}");
        assert!(!gs.contains("gho_secret"));
        assert!(!ms.contains("eyJ.secret.jwt"));
        assert!(gs.contains("<token>"));
        assert!(ms.contains("<token>"));
    }

    #[test]
    fn last_updated_parses_rfc3339() {
        let t = TunnelRecord {
            tunnel_id: "tid".into(),
            name: None,
            labels: vec![],
            endpoints: vec![],
            last_updated_at: Some("2026-05-21T10:00:00Z".into()),
        };
        let lu = t.last_updated().expect("parsed");
        assert_eq!(lu.to_rfc3339(), "2026-05-21T10:00:00+00:00");
    }

    #[test]
    fn last_updated_none_on_garbled() {
        let t = TunnelRecord {
            tunnel_id: "tid".into(),
            name: None,
            labels: vec![],
            endpoints: vec![],
            last_updated_at: Some("not-a-date".into()),
        };
        assert!(t.last_updated().is_none());
    }

    #[test]
    fn last_updated_none_on_missing() {
        let t = TunnelRecord {
            tunnel_id: "tid".into(),
            name: None,
            labels: vec![],
            endpoints: vec![],
            last_updated_at: None,
        };
        assert!(t.last_updated().is_none());
    }

    #[test]
    fn vector_agent_label_match() {
        let t = TunnelRecord {
            tunnel_id: "tid".into(),
            name: None,
            labels: vec!["vector-agent".into(), "linux".into()],
            endpoints: vec![],
            last_updated_at: None,
        };
        assert!(t.is_vector_agent());
    }

    #[test]
    fn vector_agent_label_no_match_without() {
        let t = TunnelRecord {
            tunnel_id: "tid".into(),
            name: None,
            labels: vec!["linux".into()],
            endpoints: vec![],
            last_updated_at: None,
        };
        assert!(!t.is_vector_agent());
    }
}
