//! RelayTunnelHost lifecycle. Task 2 fills in the run loop.

use anyhow::Result;

#[allow(clippy::unused_async)] // Task 2 fills in
pub async fn run() -> Result<()> {
    anyhow::bail!("host::run not yet wired — Plan 08-03 Task 2");
}

#[allow(clippy::unused_async)] // Task 2 may wire live tunnel-status query
pub async fn status() -> Result<()> {
    let tok = crate::token_cache::load()?;
    match tok {
        None => {
            println!("not registered (run `vector-tunnel-agent` to register)");
        }
        Some(t) => {
            println!("provider: {:?}", t.provider);
            println!("token expires_at_unix: {}", t.expires_at_unix);
        }
    }
    Ok(())
}
