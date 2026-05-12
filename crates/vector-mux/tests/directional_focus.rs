//! WIN-03: Cmd-Opt-Arrow get_pane_direction (D-59).
//! Plan 04-02 un-ignores and fills.

#[test]
#[ignore = "Wave-0 stub: Plan 04-02"]
fn get_pane_direction_right_returns_neighbor() {
    // Plan 04-02: construct HSplit{left:Leaf(p1), right:Leaf(p2), ratio:50:50};
    // viewport 80x24; get_pane_direction(p1, Direction::Right) -> Some(p2).
    // Edge cases: from rightmost pane Right -> None; nested splits; tie-break by lowest PaneId.
    panic!("Wave-0 stub — implemented by Plan 04-02");
}
