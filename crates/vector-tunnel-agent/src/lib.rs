//! vector-tunnel-agent library surface.
//! The binary lives in `main.rs`; modules are exposed here so integration tests
//! can import them directly.

pub mod auth;
pub mod cli;
pub mod host;
pub mod session;
pub mod token_cache;
