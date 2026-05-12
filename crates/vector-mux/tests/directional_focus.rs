//! WIN-03: Cmd-Opt-Arrow get_pane_direction (D-59). Plan 04-02.

use vector_mux::{get_pane_direction, Direction, PaneId, PaneNode, SplitRatio, Tab, TabId};

fn hsplit(left: PaneId, right: PaneId, first: u16, second: u16) -> PaneNode {
    PaneNode::HSplit {
        left: Box::new(PaneNode::Leaf(left)),
        right: Box::new(PaneNode::Leaf(right)),
        ratio: SplitRatio { first, second },
    }
}

fn vsplit(top: PaneId, bottom: PaneId, first: u16, second: u16) -> PaneNode {
    PaneNode::VSplit {
        top: Box::new(PaneNode::Leaf(top)),
        bottom: Box::new(PaneNode::Leaf(bottom)),
        ratio: SplitRatio { first, second },
    }
}

fn tab_from(root: PaneNode, active: PaneId) -> Tab {
    Tab {
        id: TabId(1),
        root,
        active_pane_id: active,
        last_rows: 24,
        last_cols: 80,
    }
}

#[test]
fn right_from_left_pane_in_hsplit() {
    let p1 = PaneId(1);
    let p2 = PaneId(2);
    let tab = tab_from(hsplit(p1, p2, 40, 39), p1);
    assert_eq!(get_pane_direction(&tab, p1, Direction::Right), Some(p2));
    assert_eq!(get_pane_direction(&tab, p2, Direction::Right), None);
}

#[test]
fn down_from_top_pane_in_vsplit() {
    let p1 = PaneId(1);
    let p2 = PaneId(2);
    let tab = tab_from(vsplit(p1, p2, 12, 11), p1);
    assert_eq!(get_pane_direction(&tab, p1, Direction::Down), Some(p2));
    assert_eq!(get_pane_direction(&tab, p2, Direction::Up), Some(p1));
}

#[test]
fn wrong_direction_returns_none() {
    let p1 = PaneId(1);
    let p2 = PaneId(2);
    let tab = tab_from(hsplit(p1, p2, 40, 39), p1);
    assert_eq!(get_pane_direction(&tab, p1, Direction::Up), None);
    assert_eq!(get_pane_direction(&tab, p1, Direction::Down), None);
    assert_eq!(get_pane_direction(&tab, p1, Direction::Left), None);
}

#[test]
fn nested_splits_overlap_scoring() {
    // HSplit{ Leaf(1), VSplit{ Leaf(2), Leaf(3) } } on 80x24.
    // Outer HSplit ratio 40:39 => left rect (0,0,40,24); right rect (41,0,39,24).
    // Inner VSplit on right side ratio 12:11 => p2 rect (41,0,39,12); p3 rect (41,13,39,11).
    // From p1 Right: both p2 and p3 share the x=41 left-edge; p2 has 12 rows overlap with
    // p1's 24-row span; p3 has 11 rows. p2 (larger overlap) wins.
    let p1 = PaneId(1);
    let p2 = PaneId(2);
    let p3 = PaneId(3);
    let root = PaneNode::HSplit {
        left: Box::new(PaneNode::Leaf(p1)),
        right: Box::new(vsplit(p2, p3, 12, 11)),
        ratio: SplitRatio {
            first: 40,
            second: 39,
        },
    };
    let tab = tab_from(root, p1);
    assert_eq!(get_pane_direction(&tab, p1, Direction::Right), Some(p2));
}

#[test]
fn tie_break_by_lowest_pane_id() {
    // VSplit{Leaf(5), VSplit{Leaf(2), Leaf(3)}} where Leaf(5) sits on top of a vsplit
    // whose first arm is half-height — wait, easier: HSplit{Leaf(1), VSplit{Leaf(3), Leaf(2)}}
    // with the inner vsplit ratio 12:11. Same shape as nested_splits_overlap_scoring but
    // the inner ids are swapped so id=2 (lowest) is bottom; p1 -> Right has p3(top, 12 rows
    // overlap) and p2(bottom, 11 rows overlap). p3 wins by overlap; no tie here.
    //
    // True tie: HSplit{Leaf(1), VSplit{Leaf(5), Leaf(2), ratio 11:11 with divider}}.
    // Outer total rows 23. Need both inner sides equal overlap.
    let p1 = PaneId(1);
    let p_low = PaneId(2);
    let p_hi = PaneId(5);
    let root = PaneNode::HSplit {
        left: Box::new(PaneNode::Leaf(p1)),
        right: Box::new(PaneNode::VSplit {
            top: Box::new(PaneNode::Leaf(p_hi)),
            bottom: Box::new(PaneNode::Leaf(p_low)),
            ratio: SplitRatio {
                first: 11,
                second: 11,
            },
        }),
        ratio: SplitRatio {
            first: 40,
            second: 39,
        },
    };
    // last_rows=23 so inner first+second+1 == 23.
    let tab = Tab {
        id: TabId(1),
        root,
        active_pane_id: p1,
        last_rows: 23,
        last_cols: 80,
    };
    // Tie: p_hi has 11 rows; p_low has 11 rows. Lowest id wins.
    assert_eq!(get_pane_direction(&tab, p1, Direction::Right), Some(p_low));
}
