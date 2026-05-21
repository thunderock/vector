//! Async SSH client built atop `russh 0.60`.
//!
//! Provides `SshClient` (connect + open PTY shell), `SshChannelTransport`
//! (russh channel adapter implementing `vector_mux::PtyTransport`),
//! `ChildStdioStream` (AsyncRead+AsyncWrite over a subprocess), and a
//! `Handler` that pins host-key fingerprints. Consumed by future remote
//! transports (Dev Tunnels in Phase 8+).

pub mod client;
pub mod error;
pub mod handler;
pub mod stdio_stream;
pub mod transport;

pub use client::SshClient;
pub use error::SshError;
pub use stdio_stream::ChildStdioStream;
pub use transport::SshChannelTransport;
