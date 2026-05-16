//! POLISH-01 + POLISH-02 apply pipeline.
//! D-69: classify each config delta as `LiveApply` or `RestartRequired`.
//! Pitfall 7: font-family changes require restart (CoreText glyph cache).

use crate::schema::{Appearance, ConfigFile, ProfileBlock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveChange {
    Theme(String),
    Appearance(Appearance),
    Tint(Option<String>),
    /// Font size carried as `size_mhz = (size * 1000) as u32` since `f32` doesn't impl `Eq`.
    FontSize(u32),
    Ligatures(bool),
    Keybinds,
    PerProfile(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestartReason {
    /// Pitfall 7: CoreText caches glyph atlases per font; family swap forces restart.
    FontFamily,
}

#[derive(Debug, Clone, Default)]
pub struct ApplyPlan {
    pub live: Vec<LiveChange>,
    pub restart: Vec<RestartReason>,
}

/// Diff `old` vs `new` `ConfigFile` per the D-69 classification table.
pub fn diff_config(old: &ConfigFile, new: &ConfigFile) -> ApplyPlan {
    let mut plan = ApplyPlan::default();

    // [default].theme
    if old.default.theme != new.default.theme {
        if let Some(t) = &new.default.theme {
            plan.live.push(LiveChange::Theme(t.clone()));
        }
    }
    // [default].appearance
    if old.default.appearance != new.default.appearance {
        if let Some(a) = new.default.appearance {
            plan.live.push(LiveChange::Appearance(a));
        }
    }

    // [default.font]
    let old_font = old.default.font.as_ref();
    let new_font = new.default.font.as_ref();

    // font.family: restart required (Pitfall 7)
    let old_family = old_font.and_then(|f| f.family.as_deref());
    let new_family = new_font.and_then(|f| f.family.as_deref());
    if old_family != new_family {
        plan.restart.push(RestartReason::FontFamily);
    }

    // font.size: live
    let old_size = old_font.and_then(|f| f.size);
    let new_size = new_font.and_then(|f| f.size);
    if old_size != new_size {
        if let Some(s) = new_size {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let size_mhz = (s.max(0.0) * 1000.0) as u32;
            plan.live.push(LiveChange::FontSize(size_mhz));
        }
    }

    // font.ligatures: live
    let old_lig = old_font.and_then(|f| f.ligatures);
    let new_lig = new_font.and_then(|f| f.ligatures);
    if old_lig != new_lig {
        if let Some(b) = new_lig {
            plan.live.push(LiveChange::Ligatures(b));
        }
    }

    // Keybinds: any change at all
    if old.keybind.len() != new.keybind.len()
        || old
            .keybind
            .iter()
            .zip(new.keybind.iter())
            .any(|(o, n)| o.key != n.key || o.action != n.action)
    {
        plan.live.push(LiveChange::Keybinds);
    }

    // Per-profile diffs
    for (name, new_block) in &new.profile {
        let old_block = old.profile.get(name);
        if old_block.is_none_or(|o| profile_per_pane_differs(o, new_block)) {
            plan.live.push(LiveChange::PerProfile(name.clone()));
        }
        if let Some(tint) = profile_tint_change(old_block, new_block) {
            plan.live.push(LiveChange::Tint(Some(tint)));
        }
    }
    // Profile removed: also per-profile event so callers can drop active pane's ref.
    for name in old.profile.keys() {
        if !new.profile.contains_key(name) {
            plan.live.push(LiveChange::PerProfile(name.clone()));
        }
    }

    plan
}

fn profile_per_pane_differs(o: &ProfileBlock, n: &ProfileBlock) -> bool {
    o.env != n.env
        || o.startup_command != n.startup_command
        || o.clipboard_write != n.clipboard_write
        || o.secure_keyboard_entry != n.secure_keyboard_entry
        || o.kind != n.kind
        || o.theme != n.theme
        || o.codespace_name != n.codespace_name
        || o.dev_tunnel_id != n.dev_tunnel_id
}

fn profile_tint_change(old: Option<&ProfileBlock>, new: &ProfileBlock) -> Option<String> {
    let old_tint = old.and_then(|o| o.tint.as_deref());
    let new_tint = new.tint.as_deref();
    if old_tint == new_tint {
        None
    } else {
        new_tint.map(String::from)
    }
}

/// Parse `source` and, on success, swap into `last_good` + return the diff plan
/// vs the previous good config. On parse error, `last_good` is NOT mutated.
/// D-69: keep last-good in memory; surface error to toast layer (caller).
pub fn try_load_or_keep(
    source: &str,
    last_good: &mut Option<ConfigFile>,
) -> Result<ApplyPlan, crate::error::ConfigError> {
    let new = crate::loader::parse(source)?;
    let old = last_good.clone().unwrap_or_default();
    let plan = diff_config(&old, &new);
    *last_good = Some(new);
    Ok(plan)
}
