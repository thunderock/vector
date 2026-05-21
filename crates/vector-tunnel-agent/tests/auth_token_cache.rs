//! Tests for `token_cache` module. Plan 08-03 Task 1.
//!
//! Test 3: `token_path()` honors `$XDG_CONFIG_HOME` else `~/.config`.
//! Test 4: `save()` writes file with mode 0o600.
//! Test 5: round-trip preserves provider + tokens.
//! Test 6: `load()` on non-existent path returns `Ok(None)`.
//! Test 7: `load()` on garbled bytes returns `Err(Corrupted)`.

use std::fs;
use std::os::unix::fs::PermissionsExt;

use vector_tunnel_agent::token_cache::{self, AgentTokenError, CachedToken, Provider};

// XDG env is process-global — serialize access so tests don't race.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn with_temp_xdg<F: FnOnce()>(f: F) {
    let _g = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = tempfile::tempdir().unwrap();
    let prior = std::env::var_os("XDG_CONFIG_HOME");
    std::env::set_var("XDG_CONFIG_HOME", dir.path());
    let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    match prior {
        Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }
    if let Err(e) = unwind {
        std::panic::resume_unwind(e);
    }
}

#[test]
fn token_path_honors_xdg_config_home() {
    with_temp_xdg(|| {
        let p = token_cache::token_path();
        let xdg = std::env::var("XDG_CONFIG_HOME").unwrap();
        let expected = std::path::Path::new(&xdg)
            .join("vector")
            .join("agent-token");
        assert_eq!(p, expected);
    });
}

#[test]
fn save_writes_mode_0600() {
    with_temp_xdg(|| {
        let t = CachedToken {
            provider: Provider::GitHub,
            access_token: "gho_test_access".into(),
            refresh_token: Some("ghr_test_refresh".into()),
            expires_at_unix: 1_700_000_000,
        };
        token_cache::save(&t).expect("save");
        let path = token_cache::token_path();
        let meta = fs::metadata(&path).unwrap();
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "expected 0600 got {mode:o}");
    });
}

#[test]
fn save_then_load_round_trips() {
    with_temp_xdg(|| {
        let t = CachedToken {
            provider: Provider::Microsoft,
            access_token: "msft_access_jwt".into(),
            refresh_token: Some("msft_refresh".into()),
            expires_at_unix: 1_800_000_000,
        };
        token_cache::save(&t).expect("save");
        let loaded = token_cache::load().expect("load").expect("present");
        assert_eq!(loaded.provider, Provider::Microsoft);
        assert_eq!(loaded.access_token, "msft_access_jwt");
        assert_eq!(loaded.refresh_token.as_deref(), Some("msft_refresh"));
        assert_eq!(loaded.expires_at_unix, 1_800_000_000);
    });
}

#[test]
fn load_returns_ok_none_when_missing() {
    with_temp_xdg(|| {
        let got = token_cache::load().expect("load");
        assert!(got.is_none(), "expected None for missing file");
    });
}

#[test]
fn load_corrupted_returns_err() {
    with_temp_xdg(|| {
        let path = token_cache::token_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"not json {{{ garbled").unwrap();
        let err = token_cache::load().expect_err("should error");
        match err {
            AgentTokenError::Corrupted(_) => {}
            AgentTokenError::Io(e) => panic!("expected Corrupted, got Io: {e}"),
        }
    });
}
