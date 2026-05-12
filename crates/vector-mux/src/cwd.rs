#![allow(unsafe_code)]
//! D-63 / D-64: cwd inheritance for new tabs + splits.
//!
//! `inherit_cwd(parent_pid)` resolves the active pane's shell cwd. On macOS
//! we call `libc::proc_pidinfo(pid, PROC_PIDVNODEPATHINFO, ...)` directly
//! because `libproc 0.14`'s `pidcwd()` is documented as "not implemented for
//! macos". On Linux we delegate to `libproc::proc_pid::pidcwd`. On Err, we
//! fall back to `$HOME`; if HOME is unset, falls back to `/`. Symlinks are
//! kept resolved (matches tmux).

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
        match pidcwd(pid) {
            Ok(cwd) => return cwd,
            Err(err) => {
                tracing::warn!(
                    pid,
                    err = %err,
                    "pidcwd failed; falling back to $HOME (D-64)"
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

/// Cross-platform pidcwd. On macOS uses Darwin `proc_pidinfo` with
/// `PROC_PIDVNODEPATHINFO`; on Linux delegates to `libproc::proc_pid::pidcwd`.
#[cfg(target_os = "macos")]
pub fn pidcwd(pid: i32) -> Result<PathBuf, String> {
    use std::ffi::CStr;
    use std::mem;

    let mut info: libc::proc_vnodepathinfo = unsafe { mem::zeroed() };
    let size = libc::c_int::try_from(mem::size_of::<libc::proc_vnodepathinfo>())
        .expect("proc_vnodepathinfo size fits in c_int");
    // SAFETY: proc_pidinfo writes at most `size` bytes into `info`; we pass the
    // correct sized struct + flavor combination per Darwin's documentation.
    let ret = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDVNODEPATHINFO,
            0,
            std::ptr::addr_of_mut!(info).cast::<libc::c_void>(),
            size,
        )
    };
    if ret <= 0 {
        let err = std::io::Error::last_os_error();
        return Err(format!("proc_pidinfo(PROC_PIDVNODEPATHINFO) failed: {err}"));
    }
    // vip_path is stored as `[[c_char; 32]; 32]` to side-step libc rustc-MSRV;
    // it's contiguous memory aliasing the C `[c_char; MAXPATHLEN]` buffer.
    let ptr = std::ptr::addr_of!(info.pvi_cdir.vip_path).cast::<libc::c_char>();
    // SAFETY: contiguous `[[c_char; 32]; 32]` = 1024 c_chars, null-terminated by
    // the kernel; we read until the NUL via CStr::from_ptr.
    let c_str = unsafe { CStr::from_ptr(ptr) };
    let s = c_str.to_str().map_err(|e| format!("vip_path utf8: {e}"))?;
    Ok(PathBuf::from(s))
}

#[cfg(not(target_os = "macos"))]
pub fn pidcwd(pid: i32) -> Result<PathBuf, String> {
    libproc::proc_pid::pidcwd(pid)
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

    #[test]
    fn pidcwd_of_self_matches_current_dir() {
        // Sanity check: our pidcwd implementation should match env::current_dir
        // for our own pid on macOS.
        let pid = i32::try_from(std::process::id()).expect("pid fits in i32");
        let our_cwd = std::env::current_dir().expect("current_dir");
        match pidcwd(pid) {
            Ok(p) => {
                // /private/tmp vs /tmp etc — accept either by checking either path
                // is a suffix/prefix of the other or they're literally equal.
                let p_s = p.to_string_lossy().to_string();
                let our_s = our_cwd.to_string_lossy().to_string();
                assert!(
                    p_s == our_s
                        || p_s == format!("/private{our_s}")
                        || our_s == format!("/private{p_s}"),
                    "pidcwd({pid}) = {p_s:?} but current_dir = {our_s:?}"
                );
            }
            Err(e) => panic!("pidcwd(self) errored: {e}"),
        }
    }
}
