//! WIN-04 arch-lint: vector-term must not discriminate on transport kind.
//! Plan 04-02 un-ignores once vector-term has been audited.

use std::fs;
use std::path::Path;

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
#[ignore = "Wave-0 stub: Plan 04-02 un-ignores"]
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

fn walk(root: &Path, dir: &Path, violations: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {dir:?}: {e}")) {
        let p = entry.expect("dir entry").path();
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
