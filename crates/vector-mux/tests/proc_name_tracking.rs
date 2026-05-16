//! D-57: foreground-process tracking via tcgetpgrp + libproc::pidpath.
//!
//! Exercises the primitives used by `proc_tracker::proc_name_poll_loop` directly:
//!   - spawn shell via LocalDomain::spawn_local
//!   - send `exec sleep 30\n` to replace the shell with sleep (pid unchanged)
//!   - poll `tcgetpgrp(master_fd)` + `libproc::pidpath` every 200ms
//!   - assert a sh/zsh/bash -> sleep transition

#![allow(unsafe_code)]

use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;

use vector_mux::{LocalDomain, SpawnCommand};

#[tokio::test(flavor = "multi_thread")]
#[ignore = "real-PTY integration; run with --include-ignored"]
async fn fg_process_name_transitions_zsh_to_sleep() {
    let domain = LocalDomain::new().expect("LocalDomain::new");
    let mut spawned = domain
        .spawn_local(SpawnCommand {
            argv: None,
            cwd: None,
            rows: 24,
            cols: 80,
            env: vec![],
        })
        .await
        .expect("spawn_local");
    let master_fd = spawned.master_fd.expect("master_fd Some on macOS");
    let mut reader = spawned.transport.take_reader().expect("take_reader");

    // Drain banner/prompt.
    let _ = drain(&mut reader, Duration::from_millis(300)).await;

    // Initial fg process name should be one of sh/zsh/bash/dash.
    let initial = fg_process_name(master_fd).unwrap_or_default();
    let shell_names = ["sh", "zsh", "bash", "dash"];
    assert!(
        shell_names.contains(&initial.as_str()),
        "initial fg process should be a shell, got {initial:?}"
    );

    // Replace shell with sleep via exec; pid stays the same per exec semantics.
    spawned
        .transport
        .write(b"exec sleep 30\n")
        .await
        .expect("write exec sleep");

    // Poll for the transition for up to 3s.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    let mut observed: Option<String> = None;
    while tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(200)).await;
        if let Some(name) = fg_process_name(master_fd) {
            if name == "sleep" {
                observed = Some(name);
                break;
            }
        }
    }
    assert_eq!(
        observed.as_deref(),
        Some("sleep"),
        "expected sh/zsh/bash -> sleep transition; final name = {observed:?}"
    );

    // Drop spawned -> kill+wait via Plan 02-03 Drop impl.
}

fn fg_process_name(master_fd: std::os::fd::RawFd) -> Option<String> {
    // SAFETY: master_fd is owned by LocalPty (closed on its Drop); tcgetpgrp is
    // documented to return -1 on a bad fd, so the call is safe for any int.
    let pgrp = unsafe { libc::tcgetpgrp(master_fd) };
    if pgrp <= 0 {
        return None;
    }
    let path = libproc::proc_pid::pidpath(pgrp).ok()?;
    Path::new(&path)
        .file_name()
        .and_then(OsStr::to_str)
        .map(String::from)
}

async fn drain(reader: &mut tokio::sync::mpsc::Receiver<Vec<u8>>, total: Duration) -> Vec<u8> {
    let deadline = tokio::time::Instant::now() + total;
    let mut buf: Vec<u8> = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, reader.recv()).await {
            Ok(Some(chunk)) => buf.extend_from_slice(&chunk),
            Ok(None) | Err(_) => break,
        }
    }
    buf
}
