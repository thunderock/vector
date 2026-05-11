//! DevTunnelDomain stub. Body lands in Phase 8 (spike-gated). Trait shape locked here.

use anyhow::Result;
use async_trait::async_trait;

use crate::domain::{Domain, SpawnCommand};
use crate::transport::PtyTransport;

#[derive(Debug, Default)]
pub struct DevTunnelDomain {
    _private: (),
}

impl DevTunnelDomain {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Domain for DevTunnelDomain {
    async fn spawn(&self, _cmd: SpawnCommand) -> Result<Box<dyn PtyTransport>> {
        unimplemented!("Phase 8: Dev Tunnels Integration (spike-gated)")
    }
    fn label(&self) -> String {
        "dev_tunnel".into()
    }
    fn is_alive(&self) -> bool {
        false
    }
    async fn reconnect(&self) -> Result<()> {
        unimplemented!("Phase 9: Persistence + reconnect")
    }
}
