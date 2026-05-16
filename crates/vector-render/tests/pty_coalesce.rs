//! Coalesce buffer drains correctly under bursts (D-47).
//! Mirrors the CoalesceBuffer contract in vector-app::frame_tick.

use bytes::BytesMut;
use parking_lot::Mutex;

struct CB {
    buf: Mutex<BytesMut>,
    threshold: usize,
}

impl CB {
    fn new(threshold: usize) -> Self {
        Self {
            buf: Mutex::new(BytesMut::new()),
            threshold,
        }
    }
    fn push(&self, b: &[u8]) -> bool {
        let mut g = self.buf.lock();
        g.extend_from_slice(b);
        g.len() >= self.threshold
    }
    fn drain(&self) -> Vec<u8> {
        let mut g = self.buf.lock();
        g.split().freeze().to_vec()
    }
    fn is_empty(&self) -> bool {
        self.buf.lock().is_empty()
    }
}

#[test]
fn coalesce_accumulates_then_drains() {
    let cb = CB::new(8 * 1024);
    for _ in 0..1000 {
        cb.push(b"hello");
    }
    let out = cb.drain();
    assert_eq!(out.len(), 5000);
    assert!(out.starts_with(b"hello"));
    assert!(cb.is_empty());
}

#[test]
fn coalesce_threshold_crossed_signals_true() {
    let cb = CB::new(16);
    assert!(!cb.push(b"01234567")); // 8 bytes
    assert!(cb.push(b"89abcdef")); // 16 total >= 16
}
