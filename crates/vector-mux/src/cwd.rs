//! D-63 / D-64: cwd inheritance for new tabs + splits.
//!
//! `inherit_cwd(parent_pid)` resolves the active pane's shell cwd via
//! `libproc::proc_pid::pidcwd`; on Err it falls back to `$HOME`; if HOME is
//! unset, falls back to `/`. Symlinks are kept resolved (matches tmux).

use std::path::PathBuf;

/// Resolve the new pane's cwd from the parent pane's PID.
#[must_use]
pub fn inherit_cwd(parent_pid: Option<i32>) -> PathBuf {
    inherit_cwd_with(parent_pid, std::env::var("HOME").ok().as_deref())
}

/// Test seam: same logic but `home_env` is injected so tests can drive the
/// fallback chain deterministically without mutating `std::env`.
#[must_use]
pub fn inherit_cwd_with(parent_pid: Option<i32>, home_env: Option<&str>) -> PathBuf {
    if let Some(pid) = parent_pid {
        match libproc::proc_pid::pidcwd(pid) {
            Ok(cwd) => return cwd,
            Err(err) => {
                tracing::warn!(
                    pid,
                    ?err,
                    "libproc::pidcwd failed; falling back to $HOME (D-64)"
                );
            }
        }
    }
    if let Some(home) = home_env {
        if !home.is_empty() {
            return PathBuf::from(home);
        }
    }
    tracing::warn!("HOME unset; falling back to /");
    PathBuf::from("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_home_when_pid_is_none_and_home_set() {
        let p = inherit_cwd_with(None, Some("/Users/test"));
        assert_eq!(p, PathBuf::from("/Users/test"));
    }

    #[test]
    fn returns_slash_when_pid_is_none_and_home_unset() {
        let p = inherit_cwd_with(None, None);
        assert_eq!(p, PathBuf::from("/"));
    }

    #[test]
    fn returns_slash_when_home_empty() {
        let p = inherit_cwd_with(None, Some(""));
        assert_eq!(p, PathBuf::from("/"));
    }

    #[test]
    fn pid_zero_falls_back_to_home() {
        // pid 0 is the kernel; pidcwd(0) returns Err on macOS → fallback to HOME.
        let p = inherit_cwd_with(Some(0), Some("/Users/test"));
        assert_eq!(p, PathBuf::from("/Users/test"));
    }
}
