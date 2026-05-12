//! Internal Phase-4 return shape for Mux callers of `LocalDomain::spawn_local`.
//! Keeps the D-38 `Domain` trait surface untouched while exposing the child PID +
//! master fd that D-57 fg-process tracking and D-63 cwd inheritance both require.

use std::os::fd::RawFd;

use crate::transport::PtyTransport;

pub struct SpawnedPane {
    pub transport: Box<dyn PtyTransport>,
    /// Child shell PID. None for Codespace/DevTunnel (Phases 7/8) or after wait().
    pub pid: Option<i32>,
    /// Master PTY fd for `tcgetpgrp` (D-57). None when portable-pty can't expose it.
    pub master_fd: Option<RawFd>,
}
