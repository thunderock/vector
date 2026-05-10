//! Mux tree (Window/Tab/Pane) and Domain trait. Filled in Phase 4.

use anyhow::Result;

pub trait Domain: Send + Sync {
    fn label(&self) -> String {
        unimplemented!("Phase 4")
    }
}

pub trait Pane: Send + Sync {}

#[allow(dead_code, unused_imports)]
fn _force_anyhow_use() -> Result<()> {
    Ok(())
}
