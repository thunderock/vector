//! D-83 sub-item #2: every `path = "..."` dep in every Cargo.toml must also
//! carry a `version = "..."` key. Without it, `cargo publish` / `cargo deny`
//! ban-checks fail because path-only deps cannot be resolved from the
//! registry. Run across root + every workspace member.

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root above crates/vector-arch-tests")
        .to_path_buf()
}

fn parse_toml(path: &PathBuf) -> toml::Value {
    let body = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    toml::from_str(&body).unwrap_or_else(|e| panic!("parse {path:?}: {e}"))
}

fn check_section(manifest_label: &str, manifest: &toml::Value, section: &str) {
    let Some(deps) = manifest.get(section).and_then(toml::Value::as_table) else {
        return; // Missing section is fine — root has no [dependencies].
    };
    for (name, value) in deps {
        let Some(table) = value.as_table() else {
            // Plain `name = "1"` shorthand — no path, no problem.
            continue;
        };
        if table.contains_key("path") {
            assert!(
                table.contains_key("version"),
                "{manifest_label}: dep `{name}` in {section} has path but no version — cargo-deny bans will FAIL on publish. Add version = \"X.Y\".",
            );
        }
    }
}

#[test]
fn root_and_all_members_have_versioned_path_deps() {
    let root = workspace_root();
    let root_manifest = parse_toml(&root.join("Cargo.toml"));

    // Root: walk [workspace.dependencies] too (path overrides can appear there).
    if let Some(ws) = root_manifest.get("workspace") {
        if let Some(ws_table) = ws.as_table() {
            // Iterate workspace.dependencies via wrapped Value to share check_section.
            let wrapped = toml::Value::Table(ws_table.clone());
            check_section(
                "Cargo.toml [workspace.dependencies]",
                &wrapped,
                "dependencies",
            );
        }
    }
    // Direct sections on root are uncommon but handle them gracefully.
    check_section("Cargo.toml", &root_manifest, "dependencies");
    check_section("Cargo.toml", &root_manifest, "dev-dependencies");
    check_section("Cargo.toml", &root_manifest, "build-dependencies");

    let members = root_manifest
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .expect("workspace.members array");

    for member in members {
        let rel = member.as_str().expect("member path string");
        let path = root.join(rel).join("Cargo.toml");
        let manifest = parse_toml(&path);
        let label = format!("{rel}/Cargo.toml");
        check_section(&label, &manifest, "dependencies");
        check_section(&label, &manifest, "dev-dependencies");
        check_section(&label, &manifest, "build-dependencies");
    }
}
