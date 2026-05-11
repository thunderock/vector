//! Mux trait surface (D-38). Phase 2 ships:
//!   - `PtyTransport` + `Domain` traits in FINAL shape (Phases 7/8/9 only fill bodies).
//!   - `LocalDomain` fully implemented atop `vector_pty::LocalPty`.
//!   - `CodespaceDomain` + `DevTunnelDomain` stubs that `unimplemented!()` at runtime.
//!
//! `Pane` / `Tab` / `Window` types land in Phase 4.

pub use codespace_domain::CodespaceDomain;
pub use devtunnel_domain::DevTunnelDomain;
pub use domain::{Domain, SpawnCommand};
pub use local_domain::{LocalDomain, LocalTransport};
pub use transport::{PtyTransport, TransportKind};

mod codespace_domain;
mod devtunnel_domain;
mod domain;
mod local_domain;
mod transport;
