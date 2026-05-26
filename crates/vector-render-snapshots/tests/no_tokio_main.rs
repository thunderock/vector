//! Architecture-lint: prevents tokio-runtime ownership regressions per D-08.
//! Snapshot suite is sync-only; any block_on / tokio::main in src/ fails the test.

use std::fs;
use std::path::Path;

const FORBIDDEN: &[&str] = &[
    "#[tokio::main]",
    "#[tokio::test]",
    "Builder::new_current_thread()",
    "Runtime::new()",
];

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
    assert!(
        !body.contains("block_on("),
        "{rel}: `block_on` not allowed in vector-render-snapshots (D-08).",
    );
}
