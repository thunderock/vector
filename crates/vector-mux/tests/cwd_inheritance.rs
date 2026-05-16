//! D-63: cwd inheritance via libproc::pidcwd. Real-PTY integration.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use vector_mux::cwd::pidcwd;
use vector_mux::{LocalDomain, Mux, SplitDirection};

#[tokio::test(flavor = "multi_thread")]
#[ignore = "real-PTY integration; run with --include-ignored"]
async fn pidcwd_returns_shell_pwd() {
    let mux = Mux::new(Arc::new(LocalDomain::new().expect("LocalDomain::new")));
    let window_id = mux.create_window();
    let (_t, p1) = mux
        .create_tab_async(window_id, Some(PathBuf::from("/tmp")), 24, 80)
        .await
        .expect("create_tab_async with cwd=/tmp");

    let p1_pid = mux.pane(p1).expect("pane").shell_pid().expect("pid");
    // Give the shell a moment to land in /tmp before reading its cwd.
    tokio::time::sleep(Duration::from_millis(300)).await;
    let cwd = pidcwd(p1_pid).expect("pidcwd");
    let path_str = cwd.to_string_lossy().to_string();
    // On macOS /tmp is a symlink to /private/tmp — accept either resolution.
    assert!(
        path_str == "/tmp" || path_str == "/private/tmp",
        "p1 cwd should be /tmp or /private/tmp, got {path_str}"
    );

    // Split — split_pane_async should call inherit_cwd(p1.shell_pid()) and spawn
    // p2 with the same cwd.
    let p2 = mux
        .split_pane_async(p1, SplitDirection::Horizontal, None)
        .await
        .expect("split_pane_async");
    let p2_pid = mux.pane(p2).expect("pane").shell_pid().expect("pid");
    tokio::time::sleep(Duration::from_millis(300)).await;
    let cwd2 = pidcwd(p2_pid).expect("pidcwd p2");
    let path2 = cwd2.to_string_lossy().to_string();
    assert!(
        path2 == "/tmp" || path2 == "/private/tmp",
        "p2 inherited cwd should be /tmp or /private/tmp, got {path2}"
    );
}
