//! LPM transition + frame-tick period contract (D-44/D-46, RENDER-02/RENDER-03).
//!
//! Mirrors the contract enforced by vector-app::frame_tick::frame_period_ms.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn frame_period_ms(lpm: &Arc<AtomicBool>) -> u64 {
    if lpm.load(Ordering::Relaxed) {
        33
    } else {
        8
    }
}

#[test]
fn lpm_off_uses_8ms_tick() {
    let lpm = Arc::new(AtomicBool::new(false));
    assert_eq!(frame_period_ms(&lpm), 8);
}

#[test]
fn lpm_on_uses_33ms_tick() {
    let lpm = Arc::new(AtomicBool::new(true));
    assert_eq!(frame_period_ms(&lpm), 33);
}

#[test]
fn lpm_transition_changes_period() {
    let lpm = Arc::new(AtomicBool::new(false));
    assert_eq!(frame_period_ms(&lpm), 8);
    lpm.store(true, Ordering::Relaxed);
    assert_eq!(frame_period_ms(&lpm), 33);
}
