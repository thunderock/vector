//! Plan 05-11 (POLISH-07, gap #6) — pure-Rust test for the Switch Profile
//! submenu row builder. No AppKit calls. The dynamic NSMenu rebuild that
//! consumes these rows is tested manually (smoke) since headless winit-less
//! tests cannot construct an NSApplication on a worker thread.

use std::collections::BTreeMap;
use vector_app::menu::submenu_rows_for;
use vector_config::{ConfigFile, Kind, ProfileBlock};

fn profile(kind: Kind) -> ProfileBlock {
    ProfileBlock {
        kind: Some(kind),
        ..ProfileBlock::default()
    }
}

#[test]
fn rows_for_three_profiles_sorted_with_phase6_suffix() {
    let mut profiles: BTreeMap<String, ProfileBlock> = BTreeMap::new();
    profiles.insert("default".into(), profile(Kind::Local));
    profiles.insert("work-cs".into(), profile(Kind::Codespace));
    profiles.insert("prod".into(), profile(Kind::DevTunnel));

    let cfg = ConfigFile {
        default: ProfileBlock::default(),
        profile: profiles,
        keybind: Vec::new(),
    };

    let rows = submenu_rows_for(&cfg);
    // BTreeMap iteration is sorted alphabetically: default, prod, work-cs.
    assert_eq!(
        rows,
        vec![
            ("default".to_string(), true),
            ("prod (phase 6+)".to_string(), false),
            ("work-cs (phase 6+)".to_string(), false),
        ]
    );
}

#[test]
fn rows_for_empty_profiles_is_empty() {
    let cfg = ConfigFile::default();
    assert!(submenu_rows_for(&cfg).is_empty());
}

#[test]
fn local_without_kind_defaults_to_enabled() {
    let mut profiles: BTreeMap<String, ProfileBlock> = BTreeMap::new();
    // No `kind` set — treated as Local (default).
    profiles.insert("plain".into(), ProfileBlock::default());
    let cfg = ConfigFile {
        default: ProfileBlock::default(),
        profile: profiles,
        keybind: Vec::new(),
    };
    let rows = submenu_rows_for(&cfg);
    assert_eq!(rows, vec![("plain".to_string(), true)]);
}
