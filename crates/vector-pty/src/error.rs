//! Library error type. anyhow is for binaries; this is matchable by callers.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PtyError {
    #[error("openpty failed: {0}")]
    OpenPty(String),
    #[error("spawn_command failed: {0}")]
    Spawn(String),
    #[error("resize failed: {0}")]
    Resize(String),
    #[error("write channel closed")]
    WriteClosed,
    #[error("child already waited")]
    AlreadyWaited,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
