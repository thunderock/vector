//! Microsoft Dev Tunnels client (Mac side) for Phase 8 Vector Tunnel Agent.

pub mod api;
pub mod auth;
pub mod domain;
pub mod model;
pub mod transport;

pub use api::{ApiError, DevTunnelsApi, TUNNELS_BASE_URL};
pub use domain::connect_tunnel;
pub use model::{AuthProvider, TunnelEndpoint, TunnelRecord};
pub use transport::{DevTunnelTransport, TransportError, AGENT_PORT};
