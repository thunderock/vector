//! Vector Tunnel Agent wire protocol. JSON, newline-delimited per D-12.
//! See 08-CONTEXT.md §<decisions> D-12 through D-15.

use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 1;

/// Wire messages. `op` is the JSON tag; `bytes` fields are base64-encoded
/// in the on-wire form via the `serde_bytes_b64` module.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum AgentMessage {
    OpenPty {
        protocol_version: u32,
        rows: u16,
        cols: u16,
        shell: Option<String>,
    },
    Opened {
        protocol_version: u32,
        session: String,
    },
    Data {
        session: String,
        #[serde(with = "serde_bytes_b64")]
        bytes: Vec<u8>,
    },
    Resize {
        session: String,
        rows: u16,
        cols: u16,
    },
    Exit {
        session: String,
        code: i32,
    },
    Error {
        reason: String,
    },
    #[serde(other)]
    Unknown,
}

// Manual Debug — never include `bytes` content (could contain shell output / secrets).
impl std::fmt::Debug for AgentMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenPty {
                rows, cols, shell, ..
            } => f
                .debug_struct("OpenPty")
                .field("rows", rows)
                .field("cols", cols)
                .field("shell", shell)
                .finish(),
            Self::Opened { session, .. } => {
                f.debug_struct("Opened").field("session", session).finish()
            }
            Self::Data { session, bytes } => f
                .debug_struct("Data")
                .field("session", session)
                .field("bytes_len", &bytes.len())
                .finish(),
            Self::Resize {
                session,
                rows,
                cols,
            } => f
                .debug_struct("Resize")
                .field("session", session)
                .field("rows", rows)
                .field("cols", cols)
                .finish(),
            Self::Exit { session, code } => f
                .debug_struct("Exit")
                .field("session", session)
                .field("code", code)
                .finish(),
            Self::Error { reason } => f.debug_struct("Error").field("reason", reason).finish(),
            Self::Unknown => f.write_str("Unknown"),
        }
    }
}

mod serde_bytes_b64 {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&STANDARD.encode(v))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        STANDARD.decode(s).map_err(serde::de::Error::custom)
    }
}
