use std::process::Command;

fn main() {
    let sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map_or_else(
            || "unknown".into(),
            |o| String::from_utf8_lossy(&o.stdout).trim().to_string(),
        );

    println!("cargo:rustc-env=VECTOR_BUILD_SHA={sha}");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    println!("cargo:rerun-if-env-changed=VECTOR_BUILD_SHA_OVERRIDE");

    // POLISH-08 / D-80 — Carbon framework for EnableSecureEventInput /
    // DisableSecureEventInput. macOS-only; harmless link directive on other
    // hosts because we never build the SKE module off-target.
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=framework=Carbon");
}
