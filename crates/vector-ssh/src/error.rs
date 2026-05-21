//! Error surface for vector-ssh.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SshError {
    #[error("gh subprocess spawn failed: {0}")]
    GhSpawn(#[from] std::io::Error),

    #[error("russh: {0}")]
    Russh(#[from] russh::Error),

    #[error("authentication failed")]
    AuthFailed,

    #[error("host key fingerprint mismatch: expected {expected}, got {actual}")]
    HostKeyMismatch { expected: String, actual: String },

    #[error("channel closed unexpectedly")]
    ChannelClosed,

    #[error("other: {0}")]
    Other(#[from] anyhow::Error),
}
