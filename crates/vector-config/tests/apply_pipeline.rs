//! POLISH-01 + POLISH-02 apply pipeline: live-vs-restart classification + parse-error-keep-last-good (D-69).

use vector_config::{diff_config, parse, try_load_or_keep, ConfigFile, RestartReason};

#[test]
fn parse_error_keeps_last_good() {
    let good = parse("[default]\ntheme = \"vector-dark\"\n").unwrap();
    let mut last_good: Option<ConfigFile> = Some(good);

    // Invalid TOML — must NOT mutate last_good.
    let err = try_load_or_keep("bad = !\n", &mut last_good).expect_err("invalid TOML must fail");
    assert!(err.line >= 1);
    assert_eq!(
        last_good.as_ref().unwrap().default.theme.as_deref(),
        Some("vector-dark"),
        "D-69: parse error must KEEP last-good unchanged"
    );

    // Valid TOML — must update.
    let _plan =
        try_load_or_keep("[default.font]\nfamily = \"Fira Code\"\n", &mut last_good).unwrap();
    assert_eq!(
        last_good
            .as_ref()
            .unwrap()
            .default
            .font
            .as_ref()
            .unwrap()
            .family
            .as_deref(),
        Some("Fira Code"),
    );
}

#[test]
fn font_family_change_requires_restart() {
    let old = parse("[default.font]\nfamily = \"JetBrains Mono\"\n").unwrap();
    let new = parse("[default.font]\nfamily = \"Fira Code\"\n").unwrap();
    let plan = diff_config(&old, &new);
    assert!(
        plan.restart.contains(&RestartReason::FontFamily),
        "Pitfall 7: font-family change MUST require restart (CoreText cache)"
    );
}
