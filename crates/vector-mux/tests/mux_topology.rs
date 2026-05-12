//! WIN-02: Cmd-T -> tab/pane allocation invariants.
//! Plan 04-02 un-ignores and fills.

#[test]
#[ignore = "Wave-0 stub: Plan 04-02"]
fn create_tab_allocates_unique_ids() {
    // Plan 04-02 fills: Mux::new() + create_tab() twice -> asserts pane_id_2 > pane_id_1,
    // tab_id_2 > tab_id_1, mux.window_count() == 1, mux.tab_count(window_id_1) == 2.
    panic!("Wave-0 stub — implemented by Plan 04-02");
}
