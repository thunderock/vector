//! Per-channel session. Task 2 fills in the JSON-protocol pump.

use anyhow::Result;

/// Placeholder — Task 2 replaces with the real per-channel pump.
#[allow(clippy::unused_async)] // Task 2 fills in
pub async fn run<S>(_stream: S) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    anyhow::bail!("session::run not yet wired — Plan 08-03 Task 2")
}
