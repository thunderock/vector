//! CORE-04: spawn shell, propagate SIGWINCH on resize, exit cleanly with no zombies.

use std::path::Path;
use std::time::Duration;

use tokio::runtime::Builder;

use vector_pty::{LocalPty, SpawnCommand};

fn rt() -> tokio::runtime::Runtime {
    Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("build runtime")
}

async fn read_for(rx: &mut tokio::sync::mpsc::Receiver<Vec<u8>>, dur: Duration) -> Vec<u8> {
    let mut out = Vec::new();
    let deadline = tokio::time::Instant::now() + dur;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(chunk)) => out.extend_from_slice(&chunk),
            Ok(None) | Err(_) => break,
        }
    }
    out
}

#[test]
fn spawn_echo_and_collect_output() {
    rt().block_on(async {
        let mut pty = LocalPty::spawn(
            Path::new("/bin/sh"),
            SpawnCommand {
                argv: Some(vec!["/bin/sh".into(), "-c".into(), "echo hi".into()]),
                cwd: None,
                rows: 24,
                cols: 80,
                env: vec![],
            },
        )
        .expect("spawn");

        let mut rx = pty.take_reader().expect("first take_reader");
        let out = read_for(&mut rx, Duration::from_secs(3)).await;
        let s = String::from_utf8_lossy(&out);
        assert!(
            s.contains("hi"),
            "expected 'hi' in child output, got: {s:?}"
        );

        let status = pty.wait().await.expect("wait");
        assert_eq!(status, Some(0));
    });
}

#[test]
fn resize_propagates_sigwinch_to_child() {
    // Many shells (notably bash 3.2 on macOS) do NOT interrupt `sleep` on
    // SIGWINCH+trap. Use a polling loop in the child so the trap fires
    // between iterations of the short sleep.
    rt().block_on(async {
        let mut pty = LocalPty::spawn(
            Path::new("/bin/sh"),
            SpawnCommand {
                argv: Some(vec![
                    "/bin/sh".into(),
                    "-c".into(),
                    "trap 'stty size; exit 0' WINCH; \
                     i=0; while [ $i -lt 50 ]; do sleep 0.1; i=$((i+1)); done"
                        .into(),
                ]),
                cwd: None,
                rows: 24,
                cols: 80,
                env: vec![],
            },
        )
        .expect("spawn");

        let mut rx = pty.take_reader().expect("take_reader");
        // Give the trap a moment to install.
        tokio::time::sleep(Duration::from_millis(300)).await;
        pty.resize(50, 100, 0, 0).expect("resize");

        let out = read_for(&mut rx, Duration::from_secs(4)).await;
        let s = String::from_utf8_lossy(&out);
        assert!(
            s.contains("50 100"),
            "expected 'rows cols' after SIGWINCH, got: {s:?}"
        );
    });
}

#[test]
fn no_zombies_after_clean_exit() {
    rt().block_on(async {
        let mut pty = LocalPty::spawn(
            Path::new("/bin/sh"),
            SpawnCommand {
                argv: Some(vec!["/bin/sh".into(), "-c".into(), "exit 0".into()]),
                cwd: None,
                rows: 24,
                cols: 80,
                env: vec![],
            },
        )
        .expect("spawn");

        let status = pty.wait().await.expect("wait");
        assert_eq!(status, Some(0));
        drop(pty);

        // Best-effort: assert no <defunct> sh in this user's process list.
        let ps = std::process::Command::new("ps")
            .args(["-o", "stat,command"])
            .output()
            .expect("ps");
        let out = String::from_utf8_lossy(&ps.stdout);
        let zombies: Vec<&str> = out
            .lines()
            .filter(|l| l.contains("<defunct>") && l.contains("sh"))
            .collect();
        assert!(zombies.is_empty(), "found zombie sh: {zombies:?}");
    });
}

#[test]
fn drop_master_terminates_child() {
    rt().block_on(async {
        let mut pty = LocalPty::spawn(
            Path::new("/bin/sh"),
            SpawnCommand {
                argv: Some(vec![
                    "/bin/sh".into(),
                    "-c".into(),
                    "echo PID=$$; sleep 30".into(),
                ]),
                cwd: None,
                rows: 24,
                cols: 80,
                env: vec![],
            },
        )
        .expect("spawn");

        let mut rx = pty.take_reader().expect("take_reader");
        let out = read_for(&mut rx, Duration::from_secs(2)).await;
        let s = String::from_utf8_lossy(&out);
        let pid: i32 = s
            .lines()
            .find_map(|line| {
                line.strip_prefix("PID=")
                    .and_then(|p| p.trim().parse().ok())
            })
            .unwrap_or_else(|| panic!("expected PID=N in output, got: {s:?}"));

        drop(pty); // Drop kicks child via kill+wait.

        // Give the kernel a beat.
        tokio::time::sleep(Duration::from_millis(500)).await;

        let alive = std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .expect("kill -0")
            .success();
        assert!(!alive, "child PID {pid} still alive after drop");
    });
}
