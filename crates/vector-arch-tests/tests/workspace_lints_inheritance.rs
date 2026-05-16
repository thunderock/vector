//! D-83 sub-item #1: asserts every workspace member crate inherits `[lints]
//! workspace = true`, EXCEPT vector-app which must opt out of the workspace's
//! `unsafe_code = "deny"` for AppKit FFI (NSTextInputClient/SKE/NSPasteboard).
//! vector-app instead carries `[lints.rust] unsafe_code = "allow"` + a mirror
//! of all other workspace lints — Cargo's `lints.workspace + [lints.rust]`
//! override pattern is rejected by the manifest parser, so full re-spec is the
//! only valid syntax. This test treats vector-app as an exception and asserts
//! its allowlist contract instead.

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

#[test]
fn every_member_inherits_workspace_lints_or_is_documented_exception() {
    let root = workspace_root();
    let root_manifest = parse_toml(&root.join("Cargo.toml"));
    let members = root_manifest
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .expect("workspace.members array");

    for member in members {
        let rel = member.as_str().expect("member path string");
        let path = root.join(rel).join("Cargo.toml");
        let manifest = parse_toml(&path);
        let name = manifest
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or(rel);

        if name == "vector-app" {
            // Documented exception — vector-app needs `unsafe_code = "allow"`.
            // Asserted in `vector_app_allows_unsafe_code` below.
            continue;
        }

        let lints = manifest
            .get("lints")
            .unwrap_or_else(|| panic!("crate {name} ({rel}) missing [lints] table"));
        let workspace_inherit = lints
            .get("workspace")
            .and_then(toml::Value::as_bool)
            .unwrap_or(false);
        assert!(
            workspace_inherit,
            "crate {name} ({rel}) missing `[lints] workspace = true` — D-83 #1 violation",
        );
    }
}

#[test]
fn vector_app_allows_unsafe_code() {
    let root = workspace_root();
    let manifest = parse_toml(&root.join("crates/vector-app/Cargo.toml"));
    let unsafe_setting = manifest
        .get("lints")
        .and_then(|l| l.get("rust"))
        .and_then(|r| r.get("unsafe_code"))
        .and_then(toml::Value::as_str)
        .expect("vector-app missing [lints.rust] unsafe_code (D-83 AppKit FFI exception)");
    assert_eq!(
        unsafe_setting, "allow",
        "vector-app must declare unsafe_code = \"allow\" (AppKit FFI exception)",
    );
}
