//! VT parser + grid + scrollback. Filled in Phase 2 atop `alacritty_terminal`.

use anyhow::Result;

pub trait Terminal: Send {}

#[allow(dead_code, unused_imports)]
fn _force_anyhow_use() -> Result<()> {
    Ok(())
}
