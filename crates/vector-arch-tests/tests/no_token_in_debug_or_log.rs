//! Pitfall 14: blocks #[derive(Debug)] adjacent to token-bearing struct fields.
//! Also blocks `tracing::*!` calls whose argument list contains a token-named
//! local. Greps the source tree; passes if zero violations.

use std::fs;
use std::path::Path;

fn walk_rs(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
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

#[test]
fn no_derive_debug_on_token_bearing_types() {
    // Scan vector-codespaces source for any line containing both
    // #[derive(...Debug...)] and a struct field named *_token / *_secret.
    // We approximate by scanning for #[derive(...Debug...)] immediately
    // followed (within 30 lines) by a field named `access_token`,
    // `refresh_token`, `device_code`, `client_secret`.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // crates/
        .join("vector-codespaces")
        .join("src");
    let mut files = Vec::new();
    walk_rs(&root, &mut files);
    let banned_field_re = regex::Regex::new(
        r"\b(access_token|refresh_token|device_code|client_secret|user_code)\s*:",
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
    // Scan vector-codespaces for tracing::{info,warn,error,debug,trace,span}!
    // whose body references a token-named identifier.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("vector-codespaces")
        .join("src");
    let mut files = Vec::new();
    walk_rs(&root, &mut files);
    let tracing_re = regex::Regex::new(
        r"tracing::(info|warn|error|debug|trace|span)!\s*\([^)]*\b(access_token|refresh_token|device_code|client_secret|user_code)\b",
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
