//! Bracketed paste wrapping (xterm mode 2004). D-53.

/// Wrap a paste payload in xterm bracketed-paste markers.
/// Normalizes CRLF and lone CR to LF (xterm convention).
#[must_use]
pub fn wrap_bracketed_paste(s: &str) -> Vec<u8> {
    let normalized: String = s.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = Vec::with_capacity(normalized.len() + 12);
    out.extend_from_slice(b"\x1b[200~");
    out.extend_from_slice(normalized.as_bytes());
    out.extend_from_slice(b"\x1b[201~");
    out
}
