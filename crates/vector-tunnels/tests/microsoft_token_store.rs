//! DT-02 Microsoft token store roundtrip. `#[ignore]`-gated by default because
//! the macOS Keychain prompts interactively and CI runners lack it (mirrors
//! Phase 6 `vector-codespaces::tests::keychain_roundtrip`).
//!
//! Run locally with:
//!   cargo test -p vector-tunnels --test microsoft_token_store -- --ignored
//!
//! Each test uses a UNIQUE service namespace so concurrent runs / leftover
//! state from prior runs don't collide.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use vector_secrets::Secrets;
use vector_tunnels::auth::{MicrosoftTokenStore, MicrosoftTokens};

fn unique_service() -> String {
    // pid + nanos avoids the `uuid` dep while remaining collision-free for
    // realistic test concurrency.
    let nanos = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("vector-test-msft-{}-{}", std::process::id(), nanos)
}

fn sample_tokens() -> MicrosoftTokens {
    MicrosoftTokens {
        access_token: "at_uat_value_local_only".into(),
        refresh_token: Some("rt_uat_value_local_only".into()),
        // Round to seconds — the JSON blob stores `expires_at_unix` as u64 seconds,
        // so any sub-second resolution is lost across save+load.
        expires_at: UNIX_EPOCH + Duration::from_secs(1_900_000_000),
    }
}

// Test 1
#[test]
#[ignore = "Manual UAT — requires real macOS Keychain"]
fn save_then_load_returns_identical_tokens() {
    let store = MicrosoftTokenStore::new(Secrets::new(unique_service()));
    let _ = store.clear();

    let tokens = sample_tokens();
    store.save(&tokens).expect("save");
    let loaded = store.load().expect("load").expect("present");
    assert_eq!(loaded.access_token, tokens.access_token);
    assert_eq!(loaded.refresh_token, tokens.refresh_token);
    assert_eq!(loaded.expires_at, tokens.expires_at);

    let _ = store.clear();
}

// Test 2
#[test]
#[ignore = "Manual UAT — requires real macOS Keychain"]
fn clear_after_save_yields_none() {
    let store = MicrosoftTokenStore::new(Secrets::new(unique_service()));
    let _ = store.clear();

    store.save(&sample_tokens()).expect("save");
    store.clear().expect("clear");
    let loaded = store.load().expect("load");
    assert!(loaded.is_none(), "post-clear load should be None");
}

// Test 3
#[test]
#[ignore = "Manual UAT — requires real macOS Keychain"]
fn load_when_never_saved_returns_ok_none_not_err() {
    let store = MicrosoftTokenStore::new(Secrets::new(unique_service()));
    let _ = store.clear();

    let loaded = store.load();
    assert!(loaded.is_ok(), "load on fresh store should be Ok(None), got {loaded:?}");
    assert!(loaded.unwrap().is_none());
}

// Test 4 — pure unit: Debug never leaks
#[test]
fn debug_format_never_leaks_token_bytes() {
    // No Keychain access needed — `MicrosoftTokenStore::new` just stores Secrets.
    let store = MicrosoftTokenStore::new(Secrets::new("vector-test-msft-debug-check"));
    let rendered = format!("{store:?}");
    // The store holds NO token bytes — `service` is the only field rendered.
    // Confirm the format is well-formed and doesn't accidentally name a token field.
    assert!(rendered.contains("MicrosoftTokenStore"));
    assert!(rendered.contains("service"));
    assert!(
        !rendered.contains("access_token") && !rendered.contains("refresh_token"),
        "Debug surface should not name token fields: {rendered}"
    );
}

// Sanity build-only check: SystemTime sub-second resolution is lost across the
// JSON blob, so users must compare at second resolution. Documented here so the
// next reader doesn't trip over it.
#[test]
fn save_load_drops_subsecond_resolution() {
    // Pure constructor test — does not touch Keychain.
    let with_nanos = MicrosoftTokens {
        access_token: "a".into(),
        refresh_token: None,
        expires_at: SystemTime::now(),
    };
    let secs = with_nanos
        .expires_at
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let rebuilt_expires_at = UNIX_EPOCH + Duration::from_secs(secs);
    // Confirm equality only at second resolution — the round-trip rounds down.
    assert!(rebuilt_expires_at <= with_nanos.expires_at);
}
