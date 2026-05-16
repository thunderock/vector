//! CS-03 writer tests — append_codespace_profile + derive_profile_name.

use std::fs;
use vector_config::{append_codespace_profile, derive_profile_name, parse, Kind};

#[test]
fn append_codespace_profile_writes_correct_block() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    fs::write(&path, "[default]\ntheme = \"vector-dark\"\n").unwrap();
    let name = append_codespace_profile(
        &path,
        "hello-world",
        "octocat/hello-world-abc123",
        "#7a3aaf",
    )
    .expect("append");
    assert_eq!(name, "hello-world");

    let source = fs::read_to_string(&path).unwrap();
    let cfg = parse(&source).expect("parse");
    let block = cfg.profile.get("hello-world").expect("profile");
    assert_eq!(block.kind, Some(Kind::Codespace));
    assert_eq!(
        block.codespace_name.as_deref(),
        Some("octocat/hello-world-abc123")
    );
    assert_eq!(block.tint.as_deref(), Some("#7a3aaf"));
}

#[test]
fn derive_profile_name_strips_random_suffix() {
    assert_eq!(
        derive_profile_name("octocat/hello-world-abc123", &[]),
        "hello-world"
    );
    assert_eq!(
        derive_profile_name("colligo/vector-x7k2m1n8", &[]),
        "vector"
    );
}

#[test]
fn derive_profile_name_keeps_non_random_tail() {
    // 'v2' is 2 chars — below the 4-char regex threshold; keep verbatim.
    assert_eq!(
        derive_profile_name("adobe/design-system-v2", &[]),
        "design-system-v2"
    );
    // 'main' is 4 chars but lowercase alphanumeric; regex strips it.
    assert_eq!(derive_profile_name("o/some-r-main", &[]), "some-r");
}

#[test]
fn derive_profile_name_decollides() {
    assert_eq!(
        derive_profile_name("colligo/vector-x7k2m1n8", &["vector"]),
        "vector-2"
    );
    assert_eq!(
        derive_profile_name(
            "colligo/vector-x7k2m1n8",
            &["vector", "vector-2", "vector-3"]
        ),
        "vector-4"
    );
}

#[test]
fn append_preserves_existing_blocks() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let source =
        "# my config\n[default]\ntheme = \"x\"\n\n[profile.work-local]\nkind = \"local\"\n";
    fs::write(&path, source).unwrap();
    append_codespace_profile(&path, "test-cs", "o/test-cs-xyz12", "#7a3aaf").unwrap();
    let after = fs::read_to_string(&path).unwrap();
    assert!(after.contains("# my config"), "comment preserved");
    assert!(
        after.contains("[profile.work-local]"),
        "existing profile preserved"
    );
    assert!(after.contains("[profile.test-cs]"), "new profile present");
    assert!(after.contains("kind = \"codespace\""), "kind written");
    let cfg = parse(&after).expect("re-parses");
    assert_eq!(cfg.profile.len(), 2);
}

#[test]
fn append_atomic_rename_no_partial() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let tmp = dir.path().join("config.toml.tmp");
    fs::write(&path, "[default]\n").unwrap();
    append_codespace_profile(&path, "atomic", "o/atomic-q9p8m", "#7a3aaf").unwrap();
    assert!(
        !tmp.exists(),
        ".toml.tmp must not persist after successful append"
    );
    let body = fs::read_to_string(&path).unwrap();
    assert!(body.contains("[profile.atomic]"));
    parse(&body).expect("final file parses");
}
