#![allow(unsafe_code)]
//! D-57: foreground-process tracking at 1 Hz.
//!
//! Walks `Mux::panes_snapshot()` each tick, calls `tcgetpgrp(master_fd)` to find
//! each pane's foreground process group, resolves via `libproc::pidpath`,
//! and invokes the user-provided callback only on transitions (label changed).
//!
//! Generic over the emit callback so this crate stays free of `winit` / app
//! dependencies. vector-app wires the callback to `EventLoopProxy::send_event(
//! UserEvent::PaneTitleChanged { .. })`.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio::time::{interval, MissedTickBehavior};

use crate::ids::PaneId;
use crate::mux::Mux;

/// Run the polling loop forever. Returns when `emit` is dropped or the runtime exits.
pub async fn proc_name_poll_loop<F>(mut emit: F)
where
    F: FnMut(PaneId, String) + Send + 'static,
{
    let mut iv = interval(Duration::from_secs(1));
    iv.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut last_seen: HashMap<PaneId, String> = HashMap::new();
    loop {
        iv.tick().await;
        let snapshot = Mux::get().panes_snapshot();
        for (pane_id, master_fd, _pid) in snapshot {
            let Some(fd) = master_fd else { continue };
            if fd < 0 {
                continue;
            }
            // SAFETY: fd is owned by the Pane's LocalPty (closed on Drop);
            // tcgetpgrp is documented to be safe for any int fd (returns -1 on bad fd).
            let pgrp = unsafe { libc::tcgetpgrp(fd) };
            if pgrp <= 0 {
                continue;
            }
            let Some(name) = pidpath_basename(pgrp) else {
                continue;
            };
            if name.is_empty() {
                continue;
            }
            if last_seen.get(&pane_id) != Some(&name) {
                last_seen.insert(pane_id, name.clone());
                emit(pane_id, name);
            }
        }
    }
}

fn pidpath_basename(pid: i32) -> Option<String> {
    let path = libproc::proc_pid::pidpath(pid).ok()?;
    Path::new(&path)
        .file_name()
        .and_then(OsStr::to_str)
        .map(String::from)
}

/// Spawn the poll loop as a tokio task. Caller keeps the JoinHandle to manage
/// the task lifetime; dropping the handle is OK (task continues to run).
pub fn spawn_proc_tracker<F>(emit: F) -> JoinHandle<()>
where
    F: FnMut(PaneId, String) + Send + 'static,
{
    tokio::spawn(proc_name_poll_loop(emit))
}
