use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use toml_edit::{value, DocumentMut};
use xshell::{cmd, Shell};

pub fn release(sh: &Shell) -> Result<()> {
    // Cargo SemVer rejects leading zeros in any version component, so months
    // and days must be unpadded: `2026.5.10`, not `2026.05.10`.
    let version = Local::now().format("%Y.%-m.%-d").to_string();
    let tag = format!("v{version}");

    // CalVer permits one release per day (D-27). Refuse to overwrite an
    // existing tag for today; user must wait until tomorrow or bump manually.
    if cmd!(sh, "git rev-parse --verify {tag}").read().is_ok() {
        anyhow::bail!("tag {tag} already exists. CalVer permits one release per day.");
    }

    bump_workspace_version(&version).context("bump workspace version")?;
    cmd!(sh, "git-cliff -t {tag} -o CHANGELOG.md")
        .run()
        .context("git-cliff")?;
    cmd!(sh, "git add Cargo.toml CHANGELOG.md").run()?;
    let msg = format!("chore(release): {tag}");
    cmd!(sh, "git commit -m {msg}").run()?;
    let tag_msg = format!("Release {tag}");
    // Annotated tag — lightweight tags are silently skipped by `git push --follow-tags`.
    cmd!(sh, "git tag -a {tag} -m {tag_msg}").run()?;
    // NEVER invoke the push command here — per CLAUDE.md the user reviews
    // diffs and pushes asynchronously.
    println!("Tagged {tag} (annotated). Run the push command with --follow-tags when ready.");
    Ok(())
}

fn bump_workspace_version(version: &str) -> Result<()> {
    let cargo_toml = "Cargo.toml";
    let body = fs::read_to_string(cargo_toml)?;
    let mut doc = body.parse::<DocumentMut>()?;
    doc["workspace"]["package"]["version"] = value(version);
    fs::write(cargo_toml, doc.to_string())?;
    Ok(())
}
