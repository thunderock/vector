use anyhow::{Context, Result};
use xshell::{cmd, Shell};

pub fn generate_icns(sh: &Shell) -> Result<()> {
    let svg = "crates/vector-app/resources/icon.svg";
    let iconset_dir = "crates/vector-app/resources/icon.iconset";
    sh.remove_path(iconset_dir).ok();
    sh.create_dir(iconset_dir)?;
    let sizes = [
        (16, 1, "icon_16x16.png"),
        (16, 2, "icon_16x16@2x.png"),
        (32, 1, "icon_32x32.png"),
        (32, 2, "icon_32x32@2x.png"),
        (128, 1, "icon_128x128.png"),
        (128, 2, "icon_128x128@2x.png"),
        (256, 1, "icon_256x256.png"),
        (256, 2, "icon_256x256@2x.png"),
        (512, 1, "icon_512x512.png"),
        (512, 2, "icon_512x512@2x.png"),
    ];
    for (size, scale, fname) in sizes {
        let pixels = size * scale;
        let out = format!("{iconset_dir}/{fname}");
        let pixels_str = pixels.to_string();
        cmd!(
            sh,
            "rsvg-convert -w {pixels_str} -h {pixels_str} -o {out} {svg}"
        )
        .run()
        .context("rsvg-convert failed (brew install librsvg)")?;
    }
    let icns_out = "crates/vector-app/resources/icon.icns";
    cmd!(
        sh,
        "iconutil --convert icns --output {icns_out} {iconset_dir}"
    )
    .run()?;
    Ok(())
}
