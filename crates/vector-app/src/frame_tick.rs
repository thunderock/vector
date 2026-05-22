//! PTY-burst coalescing + frame-tick (D-44, D-47). One drain per ~8 ms tick
//! or when the buffer crosses a size threshold; LPM (D-46) extends the period
//! to ~33 ms (30 fps cap).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::BytesMut;
use parking_lot::Mutex;
use tokio::sync::Notify;
use tokio::time::{interval, MissedTickBehavior};
use vector_mux::PaneId;
use winit::event_loop::EventLoopProxy;

use crate::UserEvent;

/// 8 KiB byte threshold — when crossed mid-tick, wake the drain immediately.
pub const COALESCE_THRESHOLD: usize = 8 * 1024;
/// Default tick period at full vsync rate.
pub const TICK_FAST_MS: u64 = 8;
/// Tick period under Low Power Mode (D-46): ~30 fps.
pub const TICK_SLOW_MS: u64 = 33;

/// Shared coalesce buffer between the PTY reader and the frame_tick drain task.
pub struct CoalesceBuffer {
    buf: Mutex<BytesMut>,
    notify: Notify,
    threshold: usize,
}

impl CoalesceBuffer {
    pub fn new(threshold: usize) -> Self {
        Self {
            buf: Mutex::new(BytesMut::new()),
            notify: Notify::new(),
            threshold,
        }
    }

    /// Append a chunk; wake the drain task if the threshold is crossed.
    pub fn push(&self, chunk: &[u8]) {
        let mut g = self.buf.lock();
        g.extend_from_slice(chunk);
        if g.len() >= self.threshold {
            drop(g);
            self.notify.notify_one();
        }
    }

    /// Atomically take all pending bytes; returns empty Vec if nothing pending.
    pub fn drain(&self) -> Vec<u8> {
        let mut g = self.buf.lock();
        if g.is_empty() {
            return Vec::new();
        }
        g.split().freeze().to_vec()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.buf.lock().is_empty()
    }

    /// Plan 09-03: clone the current buffer for byte-integrity assertions.
    /// Production code MUST NOT use this — it allocates and breaks the
    /// frame-tick contract of "drain at most once per tick".
    #[doc(hidden)]
    pub fn peek_snapshot(&self) -> Vec<u8> {
        self.buf.lock().to_vec()
    }
}

/// Frame period in ms based on the LPM flag.
pub fn frame_period_ms(lpm: &Arc<AtomicBool>) -> u64 {
    if lpm.load(Ordering::Relaxed) {
        TICK_SLOW_MS
    } else {
        TICK_FAST_MS
    }
}

/// Per-pane frame-tick loop: drains this pane's coalesce buffer every ~8 ms
/// (or ~33 ms under LPM) and emits one `PaneOutput { pane_id, bytes }` per
/// non-empty drain. Empty drains emit nothing — idle CPU stays near zero
/// (RENDER-03). Plan 04-03 generalizes Phase 3's single-pane version.
pub async fn frame_tick_loop(
    pane_id: PaneId,
    coalesce: Arc<CoalesceBuffer>,
    proxy: EventLoopProxy<UserEvent>,
    lpm: Arc<AtomicBool>,
) {
    let mut iv = interval(Duration::from_millis(TICK_FAST_MS));
    iv.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut last_drain_at = Instant::now();
    loop {
        tokio::select! {
            _ = iv.tick() => {}
            () = coalesce.notify.notified() => {}
        }
        let period = Duration::from_millis(frame_period_ms(&lpm));
        if Instant::now().duration_since(last_drain_at) < period {
            tokio::task::yield_now().await;
            continue;
        }
        let bytes = coalesce.drain();
        last_drain_at = Instant::now();
        if !bytes.is_empty()
            && proxy
                .send_event(UserEvent::PaneOutput { pane_id, bytes })
                .is_err()
        {
            tracing::info!(?pane_id, "event loop closed; frame_tick exiting");
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesce_accumulates_then_drains() {
        let cb = CoalesceBuffer::new(8 * 1024);
        for _ in 0..1000 {
            cb.push(b"hello");
        }
        let out = cb.drain();
        assert_eq!(out.len(), 5000);
        assert!(out.starts_with(b"hello"));
        assert!(cb.is_empty());
    }

    #[test]
    fn drain_empty_returns_empty() {
        let cb = CoalesceBuffer::new(16);
        let out = cb.drain();
        assert!(out.is_empty());
    }

    #[test]
    fn period_off_8ms_on_33ms() {
        let lpm = Arc::new(AtomicBool::new(false));
        assert_eq!(frame_period_ms(&lpm), 8);
        lpm.store(true, Ordering::Relaxed);
        assert_eq!(frame_period_ms(&lpm), 33);
    }
}
