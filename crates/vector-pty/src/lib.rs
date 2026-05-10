//! Local PTY transport (read/write/resize). Filled in Phase 2 via `portable-pty`.

use anyhow::Result;

pub trait PtyTransport: Send + 'static {
    fn resize(&mut self, _rows: u16, _cols: u16, _px_w: u16, _px_h: u16) -> Result<()> {
        unimplemented!("Phase 2")
    }
}

#[allow(dead_code, unused_imports)]
fn _force_anyhow_use() -> Result<()> {
    Ok(())
}
