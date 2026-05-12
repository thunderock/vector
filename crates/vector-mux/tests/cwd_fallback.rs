//! D-64: $HOME fallback when pidcwd errors.
//! Plan 04-03 un-ignores and fills.

#[test]
#[ignore = "Wave-0 stub: Plan 04-03"]
fn falls_back_to_home_on_pidcwd_err() {
    // Plan 04-03: unit test with a mocked pidcwd that returns Err
    // -> assert inherit_cwd() returns env::var('HOME').
    panic!("Wave-0 stub — implemented by Plan 04-03");
}
