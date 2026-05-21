//! Pitfall 14: blocks #[derive(Debug)] adjacent to token-bearing struct fields.
//! Also blocks `tracing::*!` calls whose argument list contains a token-named
//! local. Greps the source tree; passes if zero violations.

use std::fs;
use std::path::{Path, PathBuf};

/// Source subtrees scanned by Pitfall-14. Order: alphabetical by crate name
/// for readable diffs when new crates are added.
const SCAN_PATHS: &[&str] = &[
    "vector-codespaces/src",
    "vector-tunnel-agent/src",
    "vector-tunnel-protocol/src",
    "vector-tunnels/src",
];

fn crates_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has a parent (crates/)")
        .to_path_buf()
}

fn walk_rs(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk_rs(&p, files);
            } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                files.push(p);
            }
        }
    }
}

fn collect_sources() -> Vec<PathBuf> {
    let root = crates_root();
    let mut files = Vec::new();
    for sub in SCAN_PATHS {
        let p = root.join(sub);
        if p.exists() {
            walk_rs(&p, &mut files);
        }
    }
    files
}

#[test]
fn no_derive_debug_on_token_bearing_types() {
    // Scan all Phase-6+ token-handling crates for any line containing both
    // #[derive(...Debug...)] and a struct field named *_token / *_secret.
    // We approximate by scanning for #[derive(...Debug...)] immediately
    // followed (within 30 lines) by a token-bearing field name. Phase 8
    // extends the identifier set with `agent_token` + `tunnel_access_token`.
    let files = collect_sources();
    let banned_field_re = regex::Regex::new(
        r"\b(access_token|refresh_token|device_code|client_secret|user_code|agent_token|tunnel_access_token)\s*:",
    )
    .unwrap();
    let derive_debug_re = regex::Regex::new(r"#\[derive\([^\)]*Debug[^\)]*\)\]").unwrap();
    let mut violations = Vec::new();
    for f in &files {
        let body = fs::read_to_string(f).unwrap();
        let lines: Vec<&str> = body.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if derive_debug_re.is_match(line) {
                let window_end = (i + 30).min(lines.len());
                let window = lines[i..window_end].join("\n");
                if banned_field_re.is_match(&window) {
                    violations.push(format!(
                        "{}:{}: derive(Debug) near token field",
                        f.display(),
                        i + 1
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "Pitfall 14 violation(s):\n{}",
        violations.join("\n")
    );
}

#[test]
fn no_token_in_tracing_calls() {
    // Scan all Phase-6+ token-handling crates for tracing::{info,warn,error,debug,trace,span}!
    // whose body references a token-named identifier.
    let files = collect_sources();
    let tracing_re = regex::Regex::new(
        r"tracing::(info|warn|error|debug|trace|span)!\s*\([^)]*\b(access_token|refresh_token|device_code|client_secret|user_code|agent_token|tunnel_access_token)\b",
    )
    .unwrap();
    let mut violations = Vec::new();
    for f in &files {
        let body = fs::read_to_string(f).unwrap();
        for (i, line) in body.lines().enumerate() {
            if tracing_re.is_match(line) {
                violations.push(format!(
                    "{}:{}: tracing! references token-named field",
                    f.display(),
                    i + 1
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "Pitfall 14 tracing violation(s):\n{}",
        violations.join("\n")
    );
}

#[test]
fn scan_paths_include_new_phase_8_crates() {
    // Asserts Phase 8 crates were added to the SCAN_PATHS list (Plan 08-01 Task 2).
    let expected = [
        "vector-tunnels/src",
        "vector-tunnel-agent/src",
        "vector-tunnel-protocol/src",
    ];
    for needle in &expected {
        assert!(
            SCAN_PATHS.iter().any(|p| p == needle),
            "Pitfall 14 SCAN_PATHS missing Phase-8 crate: {needle}"
        );
    }
    // Phase 6 crate must still be scanned.
    assert!(SCAN_PATHS.iter().any(|p| p == &"vector-codespaces/src"));
}
