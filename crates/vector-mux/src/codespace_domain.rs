//! CodespaceDomain stub. Body lands in Phase 7 (SSH transport + Codespaces connect).
//! Trait shape is locked here (D-38) — Phase 7 only fills `spawn` + `reconnect`.

use anyhow::Result;
use async_trait::async_trait;

use crate::domain::{Domain, SpawnCommand};
use crate::transport::PtyTransport;

#[derive(Debug, Default)]
pub struct CodespaceDomain {
    // Fields TBD Phase 7 (codespace name, GitHub token handle, ssh keypair path, ...).
    _private: (),
}

impl CodespaceDomain {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Domain for CodespaceDomain {
    async fn spawn(&self, _cmd: SpawnCommand) -> Result<Box<dyn PtyTransport>> {
        unimplemented!("Phase 7: SSH transport + Codespaces connect")
    }
    fn label(&self) -> String {
        "codespace".into()
    }
    fn is_alive(&self) -> bool {
        false
    }
    async fn reconnect(&self) -> Result<()> {
        unimplemented!("Phase 9: Persistence + reconnect")
    }
}
