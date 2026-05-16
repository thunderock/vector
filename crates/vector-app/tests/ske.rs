//! POLISH-08 / D-80 / Pitfall 6 — Secure Keyboard Entry RAII guard tests.

use std::sync::atomic::Ordering;
use std::sync::{LazyLock, Mutex};
use vector_app::ske::{test_hooks, SecureInputGuard};

// Tests share global atomics — serialize to prevent count interference.
static TEST_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn reset_counters() {
    test_hooks::ENABLE_COUNT.store(0, Ordering::SeqCst);
    test_hooks::DISABLE_COUNT.store(0, Ordering::SeqCst);
}

#[test]
fn toggle_calls_carbon() {
    let _lock = TEST_MUTEX.lock().unwrap();
    reset_counters();
    let mut g = SecureInputGuard::new();
    g.toggle();
    assert_eq!(test_hooks::ENABLE_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(test_hooks::DISABLE_COUNT.load(Ordering::SeqCst), 0);
    g.toggle();
    assert_eq!(test_hooks::ENABLE_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(test_hooks::DISABLE_COUNT.load(Ordering::SeqCst), 1);
    drop(g);
    assert_eq!(test_hooks::DISABLE_COUNT.load(Ordering::SeqCst), 1);
}

#[test]
fn raii_disables_on_drop() {
    let _lock = TEST_MUTEX.lock().unwrap();
    reset_counters();
    {
        let mut g = SecureInputGuard::new();
        g.enable();
        assert_eq!(test_hooks::ENABLE_COUNT.load(Ordering::SeqCst), 1);
    }
    assert_eq!(
        test_hooks::DISABLE_COUNT.load(Ordering::SeqCst),
        1,
        "Pitfall 6: RAII drop MUST call DisableSecureEventInput"
    );
}
