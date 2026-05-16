//! POLISH-05 D-71 — real tmux 3.4+ DCS passthrough round-trip.
//! Enabled by CI tmux-smoke job (`brew install tmux` first) or manual `--ignored`.

use base64::Engine as _;

#[test]
#[ignore = "Requires tmux 3.4+; enabled by CI tmux-smoke or manual --ignored"]
fn dcs_round_trip_through_tmux() {
    // Verify tmux 3.4+ is present.
    let v = std::process::Command::new("tmux")
        .arg("-V")
        .output()
        .expect("tmux must be installed (brew install tmux)");
    let ver = String::from_utf8_lossy(&v.stdout);
    let parts: Vec<&str> = ver.split_whitespace().collect();
    assert!(parts.len() >= 2, "unexpected tmux -V output: {ver}");
    let v_str = parts[1];
    let (maj, min) = {
        let mut it = v_str.split('.');
        let maj: u32 = it
            .next()
            .and_then(|s| s.parse().ok())
            .expect("tmux version major");
        let min: u32 = it
            .next()
            .and_then(|s| {
                s.trim_end_matches(|c: char| !c.is_ascii_digit())
                    .parse()
                    .ok()
            })
            .unwrap_or(0);
        (maj, min)
    };
    assert!(
        maj > 3 || (maj == 3 && min >= 4),
        "tmux >= 3.4 required, got {v_str}"
    );

    let session = "vector-osc52-smoke";
    let _ = std::process::Command::new("tmux")
        .args(["kill-session", "-t", session])
        .status();

    std::process::Command::new("tmux")
        .args(["new-session", "-d", "-s", session, "-x", "80", "-y", "24"])
        .status()
        .expect("tmux new-session");
    std::process::Command::new("tmux")
        .args(["set-option", "-t", session, "-g", "allow-passthrough", "on"])
        .status()
        .expect("tmux set-option");

    let payload = "tmux passthrough OK";
    let b64 = base64::engine::general_purpose::STANDARD.encode(payload.as_bytes());
    let cmd = format!(r#"printf "\eP\e]52;c;{b64}\a\e\\""#);
    std::process::Command::new("tmux")
        .args(["send-keys", "-t", session, &cmd, "Enter"])
        .status()
        .expect("tmux send-keys");

    std::thread::sleep(std::time::Duration::from_millis(500));

    // DCS passthrough requires an interactive outer terminal to receive the sequence
    // and write to the system clipboard. In headless CI (no $TERM_PROGRAM / $TMUX /
    // WindowServer), pbpaste is always empty — skip the clipboard assertion.
    #[cfg(target_os = "macos")]
    if std::env::var("CI").is_err() {
        let out = std::process::Command::new("pbpaste")
            .output()
            .expect("pbpaste");
        let clip = String::from_utf8_lossy(&out.stdout);
        assert!(
            clip.contains(payload),
            "tmux DCS passthrough failed: pbpaste = {clip:?}, expected to contain {payload:?}"
        );
    }

    let _ = std::process::Command::new("tmux")
        .args(["kill-session", "-t", session])
        .status();
}
