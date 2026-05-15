//! D-82 Cmd-N + M4 Cmd-Shift-R reload-config (Plan 05-10 Task 3).

use vector_config::{parse, resolve_profile, Action};

#[test]
fn spawns_default_profile_home() {
    let toml = r#"
[default]
theme = "vector-dark"

[profile.work]
kind = "local"
startup_command = "cd /tmp && exec $SHELL"
"#;
    let cfg = parse(toml).unwrap();
    let resolved = resolve_profile(&cfg, "default");
    assert_eq!(resolved.name, "default");
    assert!(
        resolved.block.startup_command.is_none(),
        "D-82: default profile must NOT carry a startup_command (Cmd-N = clean slate)"
    );
}

#[test]
fn cmd_shift_r_reload_config_keybind() {
    // M4 / D-69: bundled default config ships a Cmd-Shift-R → reload-config keybind.
    let cfg = parse(vector_app::DEFAULT_CONFIG_TOML).expect("DEFAULT_CONFIG_TOML parses");
    let has_reload = cfg
        .keybind
        .iter()
        .any(|kb| kb.action == Action::ReloadConfig && kb.key.eq_ignore_ascii_case("cmd-shift-r"));
    assert!(
        has_reload,
        "M4 / D-69: bundled default config MUST ship a Cmd-Shift-R → reload-config keybind"
    );
}
