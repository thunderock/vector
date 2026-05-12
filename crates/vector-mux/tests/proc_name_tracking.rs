//! D-57: foreground process tracking via tcgetpgrp + libproc::pidpath.
//! Plan 04-03 un-ignores and fills.

#[test]
#[ignore = "Wave-0 stub: Plan 04-03"]
fn fg_process_name_transitions_zsh_to_sleep() {
    // Plan 04-03: spawn sh, send `exec sleep 5\n`, poll fg-process name every 100ms
    // for 3s -> expect a 'sh' -> 'sleep' transition. Real PTY (--include-ignored).
    panic!("Wave-0 stub — implemented by Plan 04-03");
}
