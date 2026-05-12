//! POLISH-05 / D-71: OSC 52 outbound emitter with tmux-passthrough-safe 58-byte chunking.
//!
//! Pitfall 5: tmux 3.4's allow-passthrough writes the inner sequence to the host PTY
//! in single `write()` calls capped at ~60 chars. Anything longer is truncated.
//! D-71 fixes the chunk size at 58 (2-byte safety margin).
//!
//! D-71: "Vector never re-wraps outbound — that's tmux's job." We emit RAW OSC 52
//! (no DCS wrap). Each chunk is itself a valid OSC 52 envelope.

use base64::{engine::general_purpose::STANDARD, Engine as _};

const TMUX_CHUNK_MAX: usize = 58;

/// Maximum base64-chars per OSC envelope.
pub const MAX_CHUNK_BASE64: usize = TMUX_CHUNK_MAX;

/// Encode `payload` into one or more OSC 52 envelopes. Each envelope's base64
/// content is capped at 58 bytes so tmux 3.4 passthrough cannot truncate it.
pub fn osc52_outbound(payload: &[u8]) -> Vec<u8> {
    let b64 = STANDARD.encode(payload);
    let bytes = b64.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() + 64);

    if bytes.len() <= TMUX_CHUNK_MAX {
        out.extend_from_slice(b"\x1b]52;c;");
        out.extend_from_slice(bytes);
        out.push(0x07);
        return out;
    }

    for chunk in bytes.chunks(TMUX_CHUNK_MAX) {
        out.extend_from_slice(b"\x1b]52;c;");
        out.extend_from_slice(chunk);
        out.push(0x07);
    }
    out
}
