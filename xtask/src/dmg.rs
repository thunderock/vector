use anyhow::{Context, Result};
use std::path::PathBuf;
use xshell::{cmd, Shell};

// Workspace CalVer; kept in sync with [workspace.package].version.
const VERSION: &str = "2026.5.10";

pub fn dmg_local(sh: &Shell) -> Result<()> {
    // Host-arch only — for local Apple Silicon dev box.
    cmd!(sh, "cargo build --release -p vector-app").run()?;
    let host = "target/release/vector-app";
    finalize(sh, VERSION, /*tip=*/ false, host)
}

pub fn dmg_universal(
    sh: &Shell,
    arm64: Option<PathBuf>,
    x86_64: Option<PathBuf>,
) -> Result<()> {
    let arm64: PathBuf = match arm64 {
        Some(p) => p,
        None => {
            cmd!(
                sh,
                "cargo build --release --target aarch64-apple-darwin -p vector-app"
            )
            .run()?;
            "target/aarch64-apple-darwin/release/vector-app".into()
        }
    };
    let x86_64: PathBuf = match x86_64 {
        Some(p) => p,
        None => {
            cmd!(
                sh,
                "cargo build --release --target x86_64-apple-darwin -p vector-app"
            )
            .run()?;
            "target/x86_64-apple-darwin/release/vector-app".into()
        }
    };
    let merged = "target/universal-apple-darwin/release/vector-app";
    sh.create_dir("target/universal-apple-darwin/release")?;
    let arm64_str = arm64.to_string_lossy().into_owned();
    let x86_64_str = x86_64.to_string_lossy().into_owned();
    cmd!(sh, "lipo -create -output {merged} {arm64_str} {x86_64_str}").run()?;
    // Pitfall 3: refuse a secretly-thin universal binary.
    let info = cmd!(sh, "lipo -info {merged}").read()?;
    if !info.contains("x86_64") || !info.contains("arm64") {
        anyhow::bail!(
            "Pitfall 3: lipo did not produce a fat binary. lipo -info: {info}"
        );
    }
    // cargo-bundle reads target/release/{binary}; stage the merged binary there.
    sh.create_dir("target/release")?;
    sh.copy_file(merged, "target/release/vector-app")?;
    finalize(sh, VERSION, /*tip=*/ false, "target/release/vector-app")
}

fn finalize(sh: &Shell, version: &str, tip: bool, _staged_bin: &str) -> Result<()> {
    super::icon::generate_icns(sh).context("generate icon.icns")?;
    // cargo-bundle 0.10 only copies [package.metadata.bundle].resources when
    // invoked from the crate's own directory. Running from the workspace root
    // silently drops them — causing fonts to be missing and the terminal blank.
    sh.change_dir("crates/vector-app");
    cmd!(sh, "cargo bundle --release").run()?;
    sh.change_dir("../..");
    let app_path = "target/release/bundle/osx/Vector.app";
    let bundled_bin = format!("{app_path}/Contents/MacOS/vector-app");

    // Assumption A5 fallback: cargo-bundle 0.10 re-runs `cargo build` host-arch
    // and overwrites any pre-merged binary we staged. Restore by copying the
    // canonical universal binary over Vector.app's Mach-O when it exists.
    let merged = "target/universal-apple-darwin/release/vector-app";
    if std::path::Path::new(merged).exists() {
        sh.copy_file(merged, &bundled_bin)?;
    }

    // Pitfall 3 — secret-thin guard at the bundled Mach-O level.
    let info = cmd!(sh, "lipo -info {bundled_bin}").read()?;
    println!("Bundled Mach-O: {info}");

    let dmg_name = if tip {
        let sha = cmd!(sh, "git rev-parse --short HEAD")
            .read()?
            .trim()
            .to_string();
        format!("Vector-{version}-tip-{sha}-universal.dmg")
    } else {
        format!("Vector-{version}-universal.dmg")
    };
    sh.create_dir("target/dmg")?;
    let dmg_path = format!("target/dmg/{dmg_name}");
    let bg = "crates/vector-app/resources/dmg-background.png";
    let icns = "crates/vector-app/resources/icon.icns";
    cmd!(
        sh,
        "create-dmg
           --volname Vector
           --volicon {icns}
           --background {bg}
           --window-pos 200 120
           --window-size 640 400
           --icon-size 96
           --icon Vector.app 160 200
           --app-drop-link 480 200
           --hide-extension Vector.app
           --no-internet-enable
           --hdiutil-quiet
           {dmg_path} {app_path}"
    )
    .run()?;
    println!("Built: {dmg_path}");
    Ok(())
}
