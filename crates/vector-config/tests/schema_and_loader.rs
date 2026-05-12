//! POLISH-01 + POLISH-07 schema + loader coverage.

use vector_config::{parse, resolve_profile, Kind};

#[test]
fn parse_rejects_unknown_field() {
    let toml = "[default]\nbogus = 1\n";
    let err = parse(toml).expect_err("must reject unknown field");
    assert!(err.message.contains("bogus"), "{:?}", err.message);
}

#[test]
fn profile_overrides_flat() {
    let toml = r#"
[default]
[default.font]
family = "JetBrains Mono"
size = 14.0

[profile.work]
[profile.work.font]
family = "Fira Code"
"#;
    let cfg = parse(toml).unwrap();
    let r = resolve_profile(&cfg, "work");
    let font = r.block.font.expect("font on resolved work profile");
    assert_eq!(font.family.as_deref(), Some("Fira Code"));
    assert_eq!(
        font.size, None,
        "D-68 flat-override: profile.work.font REPLACES default.font; size must be None"
    );
}

#[test]
fn profile_kinds_parse() {
    let toml = r#"
[profile.a]
kind = "local"

[profile.b]
kind = "codespace"

[profile.c]
kind = "dev_tunnel"
"#;
    let cfg = parse(toml).unwrap();
    assert_eq!(cfg.profile["a"].kind, Some(Kind::Local));
    assert_eq!(cfg.profile["b"].kind, Some(Kind::Codespace));
    assert_eq!(cfg.profile["c"].kind, Some(Kind::DevTunnel));
}

#[test]
fn error_line_col() {
    let toml = "ok = 1\nbad = !\n"; // line 2 is malformed
    let err = parse(toml).expect_err("malformed must fail");
    assert!(err.line >= 1, "line must be >= 1, got {}", err.line);
    assert!(err.col >= 1, "col must be >= 1, got {}", err.col);
    assert!(
        !err.message.contains("byte"),
        "Pitfall 2 — must not say 'byte', got: {}",
        err.message
    );
}

#[test]
fn profile_cwd_override_optional() {
    // cwd_override is #[serde(default)] — TOML without it parses fine.
    let toml = r#"
[profile.work]
kind = "local"
"#;
    let cfg = parse(toml).unwrap();
    let r = resolve_profile(&cfg, "work");
    assert!(
        r.block.cwd_override.is_none(),
        "cwd_override must default to None"
    );

    // And it parses correctly when present.
    let toml2 = r#"
[profile.work]
kind = "local"
cwd_override = "/Users/me/code"
"#;
    let cfg2 = parse(toml2).unwrap();
    let r2 = resolve_profile(&cfg2, "work");
    assert_eq!(
        r2.block.cwd_override.as_deref(),
        Some(std::path::Path::new("/Users/me/code"))
    );
}
