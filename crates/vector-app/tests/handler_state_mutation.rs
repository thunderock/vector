//! Plan 05-14 Task 2 TDD RED — App handler bodies: ToggleSearch + OpenProfilePicker.
//!
//! Tests drive the new pub(crate) helpers `do_toggle_search` and
//! `do_open_profile_picker` to verify real state mutations (not tracing::info! stubs).

use std::collections::BTreeMap;
use std::sync::{atomic::AtomicBool, Arc};
use tokio::sync::mpsc;
use vector_config::{ConfigFile, Kind, ProfileBlock};

fn make_app() -> vector_app::app::App {
    let (write_tx, _write_rx) = mpsc::channel::<Vec<u8>>(64);
    let (resize_tx, _resize_rx) = mpsc::channel::<(u16, u16)>(8);
    let lpm = Arc::new(AtomicBool::new(false));
    vector_app::app::App::new(write_tx, resize_tx, lpm)
}

fn cfg_two_profiles() -> ConfigFile {
    let mut profile = BTreeMap::new();
    profile.insert(
        "default".to_owned(),
        ProfileBlock {
            kind: Some(Kind::Local),
            ..ProfileBlock::default()
        },
    );
    profile.insert(
        "work-cs".to_owned(),
        ProfileBlock {
            kind: Some(Kind::Codespace),
            ..ProfileBlock::default()
        },
    );
    ConfigFile {
        default: ProfileBlock::default(),
        profile,
        keybind: Vec::new(),
    }
}

#[test]
fn toggle_search_opens_search_bar() {
    let mut app = make_app();
    assert!(!app.search_bar_open(), "precondition: starts closed");
    app.do_toggle_search();
    assert!(
        app.search_bar_open(),
        "ToggleSearch must open the search bar"
    );
}

#[test]
fn open_profile_picker_with_config() {
    let mut app = make_app();
    let cfg = std::sync::Arc::new(cfg_two_profiles());
    app.set_current_config(cfg);
    app.do_open_profile_picker();
    assert!(
        app.profile_picker_open(),
        "OpenProfilePicker must open the picker"
    );
    assert_eq!(
        app.profile_picker_entry_count(),
        2,
        "ProfilePicker must have 2 entries from the config"
    );
}

#[test]
fn toggle_search_twice_closes_search_bar() {
    let mut app = make_app();
    app.do_toggle_search();
    assert!(app.search_bar_open());
    app.do_toggle_search();
    assert!(
        !app.search_bar_open(),
        "second ToggleSearch must close the search bar"
    );
}
