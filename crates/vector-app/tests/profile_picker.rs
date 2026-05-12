//! POLISH-07 / D-75 / UI-SPEC §5.3 — profile picker tests.

use vector_app::profile_picker::{match_profiles, PickerEntry, ProfilePicker};
use vector_config::Kind;

fn e(name: &str, kind: Kind) -> PickerEntry {
    PickerEntry { name: name.to_owned(), kind }
}

#[test]
fn fuzzy_ranking() {
    let entries = vec![
        e("default", Kind::Local),
        e("work-cs", Kind::Local),
        e("rust-codespace", Kind::Codespace),
        e("adobe-vpn", Kind::Local),
    ];
    let ranked = match_profiles(&entries, "rs");
    let names: Vec<&str> = ranked.iter().map(|e| e.name.as_str()).collect();
    let pos_rust = names.iter().position(|n| *n == "rust-codespace").unwrap();
    let pos_work = names.iter().position(|n| *n == "work-cs").unwrap();
    assert!(
        pos_rust < pos_work,
        "fuzzy 'rs' must rank rust-codespace above work-cs; got {:?}",
        names
    );
}

#[test]
fn codespace_warning_label() {
    let entries = vec![
        e("local-1", Kind::Local),
        e("work-cs", Kind::Codespace),
        e("tunnel-prod", Kind::DevTunnel),
    ];
    let mut picker = ProfilePicker::new(entries);
    picker.open();
    for fi in 0..picker.filtered.len() {
        let label = picker.row_label(fi);
        let kind = picker.entries[picker.filtered[fi]].kind;
        match kind {
            Kind::Codespace | Kind::DevTunnel => {
                assert!(
                    label.contains("Phase 6+"),
                    "UI-SPEC §5.3: kind={:?} must show `Phase 6+`; got {:?}",
                    kind, label
                );
            }
            Kind::Local => {
                assert!(
                    !label.contains("Phase 6+"),
                    "Local must NOT show Phase 6+ suffix"
                );
            }
        }
    }
}
