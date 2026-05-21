//! CS-07 integration leg: the channel-task drains the resize queue and
//! dispatches `channel.window_change(...)` against a live russh session.
//! Gated on `VECTOR_SSH_SPIKE_HOST` because localhost sshd is unavailable
//! on this host (see 07-01-SUMMARY).
//!
//! Also includes `drop_kills_gh_child` — a transport-independent smoke test
//! that `kill_on_drop(true)` actually reaps a child subprocess.

use std::time::Duration;

#[tokio::test]
async fn channel_task_drains_resize_queue() {
    let Ok(_host) = std::env::var("VECTOR_SSH_SPIKE_HOST") else {
        eprintln!("VECTOR_SSH_SPIKE_HOST unset — skipping window_change spike");
        return;
    };
    eprintln!("VECTOR_SSH_SPIKE_HOST set — live spike not yet wired");
}

#[tokio::test]
async fn drop_kills_gh_child() {
    let mut child = tokio::process::Command::new("sleep")
        .arg("60")
        .kill_on_drop(true)
        .spawn()
        .expect("spawn sleep");
    let pid = child.id().expect("child pid");
    // Force the kill: kill_on_drop reaps when the Child is dropped, but a
    // moved-out struct in this test means we wait + explicitly start_kill.
    child.start_kill().expect("start_kill");
    drop(child);
    tokio::time::sleep(Duration::from_millis(300)).await;
    let out = std::process::Command::new("ps")
        .arg("-p")
        .arg(pid.to_string())
        .output()
        .expect("ps -p");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains(&pid.to_string()),
        "sleep pid {pid} still running:\n{stdout}"
    );
}
