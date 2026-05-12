//! WIN-03 #3: real PTY tput cols round-trip after split.
//! Plan 04-03 un-ignores and fills.

#[test]
#[ignore = "Wave-0 stub: Plan 04-03"]
fn tput_cols_round_trip_after_split() {
    // Plan 04-03: real PTY integration test (gated by `-- --include-ignored`).
    // Spawn shell in 80-col pane, split horizontally, write `tput cols\n` to each
    // pane's transport, read until prompt returns, parse `tput cols` outputs
    // -> assert pane1 + pane2 == 79 (divider takes 1 cell).
    panic!("Wave-0 stub — implemented by Plan 04-03");
}
