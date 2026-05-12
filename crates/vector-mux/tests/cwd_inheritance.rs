//! D-63: libproc::pidcwd happy path.
//! Plan 04-03 un-ignores and fills.

#[test]
#[ignore = "Wave-0 stub: Plan 04-03"]
fn pidcwd_returns_shell_pwd() {
    // Plan 04-03: real PTY integration. Spawn shell, send `cd /tmp\n`, wait for prompt,
    // call libproc::pidcwd(child_pid) -> assert returns PathBuf::from('/tmp') or canonical form.
    panic!("Wave-0 stub — implemented by Plan 04-03");
}
