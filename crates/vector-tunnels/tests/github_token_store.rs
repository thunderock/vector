//! DT-03 GitHub token store roundtrip. `#[ignore]`-gated by default because the
//! macOS Keychain prompts interactively and CI runners lack it (mirrors the
//! Microsoft token-store tests).
//!
//! Run locally with:
//!   cargo test -p vector-tunnels --test github_token_store -- --ignored
//!
//! Each test uses a UNIQUE service namespace so concurrent runs / leftover
//! state from prior runs don't collide.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use vector_secrets::Secrets;
use vector_tunnels::auth::{GitHubTokenStore, GitHubTokens};

fn unique_service() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("vector-test-gh-{}-{}", std::process::id(), nanos)
}

fn sample_tokens() -> GitHubTokens {
    GitHubTokens {
        access_token: "at_uat_value_local_only".into(),
        refresh_token: Some("rt_uat_value_local_only".into()),
        // Round to seconds — the JSON blob stores `expires_at_unix` as u64 seconds.
        expires_at: UNIX_EPOCH + Duration::from_secs(1_900_000_000),
    }
}

// Test 1 — round-trips access+refresh+expiry through GITHUB_REFRESH_ACCOUNT.
#[test]
#[ignore = "Manual UAT — requires real macOS Keychain"]
fn save_then_load_returns_identical_tokens() {
    let store = GitHubTokenStore::new(Secrets::new(unique_service()));
    let _ = store.clear();

    let tokens = sample_tokens();
    store.save(&tokens).expect("save");
    let loaded = store.load().expect("load").expect("present");
    assert_eq!(loaded.access_token, tokens.access_token);
    assert_eq!(loaded.refresh_token, tokens.refresh_token);
    assert_eq!(loaded.expires_at, tokens.expires_at);

    let _ = store.clear();
}

// Test 2 — clear deletes the slot.
#[test]
#[ignore = "Manual UAT — requires real macOS Keychain"]
fn clear_after_save_yields_none() {
    let store = GitHubTokenStore::new(Secrets::new(unique_service()));
    let _ = store.clear();

    store.save(&sample_tokens()).expect("save");
    store.clear().expect("clear");
    let loaded = store.load().expect("load");
    assert!(loaded.is_none(), "post-clear load should be None");
}

// Test 3 — load on a fresh store is Ok(None), not Err.
#[test]
#[ignore = "Manual UAT — requires real macOS Keychain"]
fn load_when_never_saved_returns_ok_none_not_err() {
    let store = GitHubTokenStore::new(Secrets::new(unique_service()));
    let _ = store.clear();

    let loaded = store.load();
    assert!(
        loaded.is_ok(),
        "load on fresh store should be Ok(None), got {loaded:?}"
    );
    assert!(loaded.unwrap().is_none());
}

// Test 4 — pure unit: Debug never leaks token field names.
#[test]
fn debug_format_never_leaks_token_bytes() {
    let store = GitHubTokenStore::new(Secrets::new("vector-test-gh-debug-check"));
    let rendered = format!("{store:?}");
    assert!(rendered.contains("GitHubTokenStore"));
    assert!(rendered.contains("service"));
    assert!(
        !rendered.contains("access_token") && !rendered.contains("refresh_token"),
        "Debug surface should not name token fields: {rendered}"
    );
}

// Sanity build-only check: sub-second resolution is lost across the JSON blob.
#[test]
fn save_load_drops_subsecond_resolution() {
    let with_nanos = GitHubTokens {
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
    assert!(rebuilt_expires_at <= with_nanos.expires_at);
}
