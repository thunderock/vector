//! Low Power Mode observer (D-46). Polls `NSProcessInfo.isLowPowerModeEnabled`
//! at 1 Hz and emits `UserEvent::LpmChanged` on transitions. The block-based
//! NSNotificationCenter observer (`NSProcessInfoPowerStateDidChangeNotification`)
//! is the documented "right" path but requires a non-trivial objc2 block bridge;
//! 03-RESEARCH explicitly allows the polling fallback as a MEDIUM-confidence path.

use std::time::Duration;

use tokio::task::JoinHandle;
use tokio::time::interval;
use winit::event_loop::EventLoopProxy;

use crate::UserEvent;

/// Read the current Low Power Mode state from NSProcessInfo.
pub fn is_low_power_mode_now() -> bool {
    let info = objc2_foundation::NSProcessInfo::processInfo();
    info.isLowPowerModeEnabled()
}

/// Spawn a 1 Hz polling task that emits `UserEvent::LpmChanged(bool)` on each
/// state transition and logs the transition via `tracing::info!`.
pub fn spawn_lpm_observer(proxy: EventLoopProxy<UserEvent>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut last = is_low_power_mode_now();
        tracing::info!(lpm_enabled = last, "low power mode initial state");
        if proxy.send_event(UserEvent::LpmChanged(last)).is_err() {
            return;
        }
        let mut iv = interval(Duration::from_secs(1));
        loop {
            iv.tick().await;
            let now = is_low_power_mode_now();
            if now != last {
                tracing::info!(lpm_enabled = now, "low power mode transition");
                if proxy.send_event(UserEvent::LpmChanged(now)).is_err() {
                    return;
                }
                last = now;
            }
        }
    })
}
