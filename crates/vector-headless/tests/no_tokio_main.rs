//! Architecture-lint: prevents tokio-runtime ownership regressions per D-08.
//! vector-headless allows `block_on` in src/main.rs only — Plan 02-05 wires
//! `rt.block_on(...)` in src/main.rs per D-09 / D-36.

use std::fs;
use std::path::Path;

const FORBIDDEN: &[&str] = &[
    "#[tokio::main]",
    "#[tokio::test]",
    "Builder::new_current_thread()",
    "Runtime::new()",
];

// `rel` is computed as the relative path from `src/`, so `src/main.rs` shows up as `main.rs`.
const BLOCK_ON_ALLOWLIST: &[&str] = &["main.rs"];

#[test]
fn forbidden_tokio_patterns_absent_from_src() {
    let crate_root = env!("CARGO_MANIFEST_DIR");
    let src = Path::new(crate_root).join("src");
    if src.exists() {
        scan_dir(&src, &src);
    }
}

fn scan_dir(root: &Path, dir: &Path) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {dir:?}: {e}")) {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            scan_dir(root, &path);
            continue;
        }
        if path.extension().is_some_and(|e| e == "rs") {
            check_file(root, &path);
        }
    }
}

fn check_file(root: &Path, path: &Path) {
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string();
    let body = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    for pattern in FORBIDDEN {
        assert!(
            !body.contains(pattern),
            "{rel}: forbidden pattern `{pattern}` (D-08 architecture-lint).",
        );
    }
    if body.contains("block_on(") {
        let allowed = BLOCK_ON_ALLOWLIST
            .iter()
            .any(|a| rel.replace('\\', "/").ends_with(a));
        assert!(
            allowed,
            "{rel}: `block_on` outside allowlist (D-08). Allowlist: {BLOCK_ON_ALLOWLIST:?}.",
        );
    }
}
