//! D-64: $HOME fallback when pidcwd errors. Unit-only (no real PTY).

use std::path::PathBuf;

use vector_mux::inherit_cwd_with;

#[test]
fn inherit_cwd_returns_home_when_pid_is_none() {
    let p = inherit_cwd_with(None, Some("/Users/test"));
    assert_eq!(p, PathBuf::from("/Users/test"));
}

#[test]
fn inherit_cwd_returns_slash_when_home_unset_and_pid_none() {
    let p = inherit_cwd_with(None, None);
    assert_eq!(p, PathBuf::from("/"));
}

#[test]
fn inherit_cwd_with_pid_zero_falls_back_to_home() {
    // pid 0 is the kernel; pidcwd(0) returns Err on macOS → fallback to HOME.
    let p = inherit_cwd_with(Some(0), Some("/Users/test"));
    assert_eq!(p, PathBuf::from("/Users/test"));
}

#[test]
fn falls_back_to_home_on_pidcwd_err() {
    // pid 999_999_999 is almost certainly invalid → pidcwd returns Err → $HOME.
    let p = inherit_cwd_with(Some(999_999_999), Some("/Users/test"));
    assert_eq!(p, PathBuf::from("/Users/test"));
}
