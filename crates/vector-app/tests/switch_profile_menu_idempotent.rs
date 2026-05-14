//! Plan 05-14 Task 2 TDD — Test 5 (LOW-3): submenu_rows_for idempotency.
//!
//! Calling `submenu_rows_for(&cfg)` twice on the same config must produce
//! equal output, proving FSEvents + Cmd-Shift-R concurrent invocations are safe.

use std::collections::BTreeMap;
use vector_app::menu::submenu_rows_for;
use vector_config::{ConfigFile, Kind, ProfileBlock};

fn make_cfg() -> ConfigFile {
    let mut profile = BTreeMap::new();
    profile.insert(
        "default".to_owned(),
        ProfileBlock { kind: Some(Kind::Local), ..ProfileBlock::default() },
    );
    profile.insert(
        "work-cs".to_owned(),
        ProfileBlock { kind: Some(Kind::Codespace), ..ProfileBlock::default() },
    );
    profile.insert(
        "prod".to_owned(),
        ProfileBlock { kind: Some(Kind::DevTunnel), ..ProfileBlock::default() },
    );
    ConfigFile {
        default: ProfileBlock::default(),
        profile,
        keybind: Vec::new(),
    }
}

#[test]
fn submenu_rows_for_is_idempotent() {
    let cfg = make_cfg();
    let rows_a = submenu_rows_for(&cfg);
    let rows_b = submenu_rows_for(&cfg);
    assert_eq!(
        rows_a, rows_b,
        "submenu_rows_for must be referentially transparent (LOW-3)"
    );
}

#[test]
fn submenu_rows_for_clone_equals_original() {
    let cfg = make_cfg();
    let cfg_clone = cfg.clone();
    let rows_a = submenu_rows_for(&cfg);
    let rows_b = submenu_rows_for(&cfg_clone);
    assert_eq!(
        rows_a, rows_b,
        "submenu_rows_for on a clone must equal the original (LOW-3)"
    );
}
