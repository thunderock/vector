//! Device-flow request + poll. Plan 06-02 fills in the body.
use std::time::Instant;

/// User-visible payload — safe to display in modals (no token material).
pub struct DeviceCodeDisplay {
    pub user_code: String,        // 8 chars + dash, e.g. "WDJB-MJHT"
    pub verification_uri: String, // "https://github.com/login/device"
    pub expires_at: Instant,
    pub interval_secs: u64,
}

// user_code/verification_uri are public per RFC 8628 §3.1 — safe to Debug.
impl std::fmt::Debug for DeviceCodeDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceCodeDisplay")
            .field("verification_uri", &self.verification_uri)
            .field("expires_at", &self.expires_at)
            .field("interval_secs", &self.interval_secs)
            .finish_non_exhaustive()
    }
}
