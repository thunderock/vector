//! WIN-04 arch-lint: vector-term must not discriminate on transport kind.
//! Plan 04-02 un-ignores. Live walk of `crates/vector-term/src/**/*.rs`
//! plus a negative meta-test that proves the walker actually fires on a violation.

use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN: &[&str] = &[
    "enum PaneSource",
    "TransportKind::Local",
    "TransportKind::Codespace",
    "TransportKind::DevTunnel",
    "transport.kind()",
    ".kind() == TransportKind",
    "match transport.kind",
];

#[test]
fn vector_term_does_not_discriminate_on_transport_kind() {
    let crate_root = env!("CARGO_MANIFEST_DIR");
    let src = Path::new(crate_root).join("src");
    let mut violations = vec![];
    walk(&src, &src, &mut violations);
    assert!(
        violations.is_empty(),
        "WIN-04 violation: vector-term must not discriminate on transport kind. Found:\n{}",
        violations.join("\n")
    );
}

#[test]
fn negative_meta_test_walker_detects_forbidden_pattern() {
    // Synthesize a temp directory containing a single .rs file with a forbidden
    // pattern, then assert the walker emits a violation. Proves the live test
    // isn't a no-op against the real vector-term/src/.
    let tmp = std::env::temp_dir().join(format!("vector-win04-meta-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).expect("create tmp dir");
    let path = tmp.join("violator.rs");
    fs::write(&path, "fn x() { let _ = TransportKind::Local; }\n").expect("write violator");

    let mut violations: Vec<String> = vec![];
    walk(&tmp, &tmp, &mut violations);
    let _ = fs::remove_dir_all(&tmp);

    assert!(
        !violations.is_empty(),
        "negative meta-test failed: walker did not detect the synthetic violation"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.contains("TransportKind::Local")),
        "expected `TransportKind::Local` in violations, got: {violations:?}"
    );
}

fn walk(root: &Path, dir: &Path, violations: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {dir:?}: {e}")) {
        let p: PathBuf = entry.expect("dir entry").path();
        if p.is_dir() {
            walk(root, &p, violations);
            continue;
        }
        if p.extension().is_some_and(|e| e == "rs") {
            let body = fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p:?}: {e}"));
            for f in FORBIDDEN {
                if body.contains(f) {
                    let rel = p.strip_prefix(root).unwrap_or(&p).display();
                    violations.push(format!("  {rel}: `{f}`"));
                }
            }
        }
    }
}
