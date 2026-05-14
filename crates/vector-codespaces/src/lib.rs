//! vector-codespaces — Phase 6: GitHub OAuth Device Flow + Codespaces REST.
//!
//! Pitfall 14: every token-bearing struct in this crate has a hand-written
//! `Debug` impl. NEVER `#[derive(Debug)]` near a field named `*_token`,
//! `*_secret`, `access_token`, `refresh_token`, `device_code`, `user_code`.
pub mod auth;
pub mod client;
pub mod model;

pub use auth::{AuthError, DeviceCodeDisplay, GitHubAuth, TokenStore};
pub use client::{ClientError, CodespacesClient};
pub use model::{Codespace, CodespaceState, GitStatus, RepositoryRef};
