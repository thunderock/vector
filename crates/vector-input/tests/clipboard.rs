//! POLISH-05 / D-71 / Pitfall 5 — outbound OSC 52 chunking at 58 base64 bytes.

use vector_input::{osc52_outbound, MAX_CHUNK_BASE64};

#[test]
fn outbound_58_byte_chunks() {
    // Short payload — single chunk.
    let short = osc52_outbound(b"hi");
    assert!(short.starts_with(b"\x1b]52;c;"));
    assert_eq!(short.last(), Some(&0x07));

    // Long payload — multi-chunk.
    let payload: Vec<u8> = vec![b'X'; 300];
    let out = osc52_outbound(&payload);

    // Walk the output, find every contiguous run of base64-alphabet bytes,
    // assert each run is <= MAX_CHUNK_BASE64.
    let mut current_run: usize = 0;
    let mut max_run: usize = 0;
    for &b in &out {
        let is_b64 = b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=';
        if is_b64 {
            current_run += 1;
            if current_run > max_run {
                max_run = current_run;
            }
        } else {
            current_run = 0;
        }
    }
    assert!(
        max_run <= MAX_CHUNK_BASE64,
        "Pitfall 5: tmux passthrough cap -> max base64 run must be <= {MAX_CHUNK_BASE64}, got {max_run}"
    );
    assert_eq!(MAX_CHUNK_BASE64, 58, "D-71: chunk size LOCKED at 58 bytes");
}
