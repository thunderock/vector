//! TOML loader + profile resolver.
//! D-68: flat-overlay inheritance — [profile.X] keys REPLACE [default] keys; tables do NOT deep-merge.
//! Pitfall 2: errors carry (line, col) — never byte offsets.

use crate::{
    error::ConfigError,
    schema::{ConfigFile, ProfileBlock},
};

#[derive(Debug, Clone)]
pub struct ResolvedProfile {
    pub name: String,
    pub block: ProfileBlock,
}

pub fn parse(source: &str) -> Result<ConfigFile, ConfigError> {
    toml::from_str::<ConfigFile>(source).map_err(|e| {
        let (line, col) = e
            .span()
            .map_or((0, 0), |s| byte_to_line_col(source, s.start));
        ConfigError {
            line,
            col,
            message: e.message().to_owned(),
        }
    })
}

pub fn resolve_profile(cfg: &ConfigFile, name: &str) -> ResolvedProfile {
    let default_block = &cfg.default;
    let profile_block = cfg.profile.get(name);

    let block = match profile_block {
        None => default_block.clone(),
        Some(p) => ProfileBlock {
            kind: p.kind.or(default_block.kind),
            theme: p.theme.clone().or_else(|| default_block.theme.clone()),
            tint: p.tint.clone().or_else(|| default_block.tint.clone()),
            appearance: p.appearance.or(default_block.appearance),
            font: p.font.clone().or_else(|| default_block.font.clone()),
            clipboard_write: p.clipboard_write.or(default_block.clipboard_write),
            secure_keyboard_entry: p
                .secure_keyboard_entry
                .or(default_block.secure_keyboard_entry),
            env: p.env.clone().or_else(|| default_block.env.clone()),
            startup_command: p
                .startup_command
                .clone()
                .or_else(|| default_block.startup_command.clone()),
            codespace_name: p
                .codespace_name
                .clone()
                .or_else(|| default_block.codespace_name.clone()),
            dev_tunnel_id: p
                .dev_tunnel_id
                .clone()
                .or_else(|| default_block.dev_tunnel_id.clone()),
            cwd_override: p
                .cwd_override
                .clone()
                .or_else(|| default_block.cwd_override.clone()),
        },
    };

    ResolvedProfile {
        name: name.to_owned(),
        block,
    }
}

fn byte_to_line_col(src: &str, byte: usize) -> (usize, usize) {
    let prefix = &src[..byte.min(src.len())];
    let line = prefix.chars().filter(|c| *c == '\n').count() + 1;
    let col = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
    (line, col)
}
