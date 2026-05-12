//! POLISH-08 / D-80 / Pitfall 6 — Secure Keyboard Entry RAII guard.
//!
//! Wraps Carbon's `EnableSecureEventInput` / `DisableSecureEventInput`. The
//! Carbon SKE flag is process-level on macOS; orphaning it strands keyboard
//! input for OTHER apps until the user logs out. RAII Drop + a panic hook
//! provide belt-and-braces disable on every exit path.

#[cfg(not(feature = "test-hooks"))]
#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn EnableSecureEventInput();
    fn DisableSecureEventInput();
    #[allow(dead_code)]
    fn IsSecureEventInputEnabled() -> u8;
}

#[cfg(feature = "test-hooks")]
pub mod test_hooks {
    use std::sync::atomic::AtomicUsize;
    pub static ENABLE_COUNT: AtomicUsize = AtomicUsize::new(0);
    pub static DISABLE_COUNT: AtomicUsize = AtomicUsize::new(0);
}

#[cfg(feature = "test-hooks")]
fn enable_impl() {
    test_hooks::ENABLE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
}
#[cfg(feature = "test-hooks")]
fn disable_impl() {
    test_hooks::DISABLE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
}
#[cfg(not(feature = "test-hooks"))]
fn enable_impl() {
    unsafe { EnableSecureEventInput() }
}
#[cfg(not(feature = "test-hooks"))]
fn disable_impl() {
    unsafe { DisableSecureEventInput() }
}

pub struct SecureInputGuard {
    enabled: bool,
}

impl Default for SecureInputGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureInputGuard {
    pub fn new() -> Self {
        Self { enabled: false }
    }

    pub fn enable(&mut self) {
        if !self.enabled {
            enable_impl();
            self.enabled = true;
        }
    }

    pub fn disable(&mut self) {
        if self.enabled {
            disable_impl();
            self.enabled = false;
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Toggle SKE; returns the new state.
    pub fn toggle(&mut self) -> bool {
        if self.enabled {
            self.disable();
        } else {
            self.enable();
        }
        self.enabled
    }
}

impl Drop for SecureInputGuard {
    /// Pitfall 6: ALWAYS disable on drop. Orphan secure-event state strands
    /// other apps' keyboards until logout.
    fn drop(&mut self) {
        self.disable();
    }
}

/// Install a panic hook that best-effort disables SKE on panic. Call once at
/// app startup so a panic mid-secure-input never leaves the flag set.
pub fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        #[cfg(not(feature = "test-hooks"))]
        unsafe {
            DisableSecureEventInput();
        }
        prev(info);
    }));
}
