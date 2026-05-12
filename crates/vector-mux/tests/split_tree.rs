//! WIN-03: Cmd-D / Cmd-Shift-D tree mutation.
//! Plan 04-02 un-ignores and fills.

#[test]
#[ignore = "Wave-0 stub: Plan 04-02"]
fn split_horizontal_then_vertical_mutates_tree() {
    // Plan 04-02: from PaneNode::Leaf(p1), call split_at_leaf(p1, p2, SplitDirection::Horizontal)
    // -> assert HSplit { left: Leaf(p1), right: Leaf(p2), ratio: ~half }; then split_at_leaf on
    // the right leaf vertically -> assert nested VSplit.
    panic!("Wave-0 stub — implemented by Plan 04-02");
}
