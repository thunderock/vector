//! Local PTY: spawns a child shell, bridges blocking read/write to tokio mpsc.
//! Trait impls (`PtyTransport`, `Domain`) live in `vector-mux` per Plan 02-04.

pub use error::PtyError;
pub use local::{LocalPty, SpawnCommand};

mod error;
mod local;
