//! CORE-04: spawn shell, SIGWINCH propagate, no zombies. Plan 02-03.

#[test]
#[ignore = "Plan 02-03"]
fn spawn_echo_and_collect_output() {
    unimplemented!("Plan 02-03");
}

#[test]
#[ignore = "Plan 02-03"]
fn resize_propagates_sigwinch_to_child() {
    // Spawn `sh -c "trap 'stty size' WINCH; sleep 5"`, write resize, read stty output.
    unimplemented!("Plan 02-03");
}

#[test]
#[ignore = "Plan 02-03"]
fn no_zombies_after_clean_exit() {
    // Spawn, exit child, drop LocalPty, assert ps shows no <defunct>.
    unimplemented!("Plan 02-03");
}

#[test]
#[ignore = "Plan 02-03"]
fn drop_master_terminates_child() {
    // drop(LocalPty) -> read returns EOF -> child receives SIGHUP -> exits.
    unimplemented!("Plan 02-03");
}
