//! WIN-03: Cmd-Shift-Arrow 1-cell ratio shift (D-60). Plan 04-02.

use vector_mux::{nudge_ratio, Direction, NudgeError, PaneId, PaneNode, SplitRatio, MIN_PANE_COLS};

fn hsplit(left: PaneId, right: PaneId, first: u16, second: u16) -> PaneNode {
    PaneNode::HSplit {
        left: Box::new(PaneNode::Leaf(left)),
        right: Box::new(PaneNode::Leaf(right)),
        ratio: SplitRatio { first, second },
    }
}

fn ratio_of(node: &PaneNode) -> SplitRatio {
    match node {
        PaneNode::HSplit { ratio, .. } | PaneNode::VSplit { ratio, .. } => *ratio,
        PaneNode::Leaf(_) => panic!("Leaf has no ratio"),
    }
}

#[test]
fn nudge_right_shifts_hsplit_ratio_one() {
    let p1 = PaneId(1);
    let p2 = PaneId(2);
    let mut node = hsplit(p1, p2, 40, 39);
    nudge_ratio(&mut node, p1, Direction::Right, MIN_PANE_COLS).expect("nudge ok");
    let r = ratio_of(&node);
    assert_eq!(r.first, 41);
    assert_eq!(r.second, 38);
}

#[test]
fn nudge_left_from_same_pane_shrinks_first() {
    let p1 = PaneId(1);
    let p2 = PaneId(2);
    let mut node = hsplit(p1, p2, 41, 38);
    nudge_ratio(&mut node, p1, Direction::Left, MIN_PANE_COLS).expect("nudge ok");
    let r = ratio_of(&node);
    assert_eq!(r.first, 40);
    assert_eq!(r.second, 39);
}

#[test]
fn nudge_below_minimum_returns_error() {
    let p1 = PaneId(1);
    let p2 = PaneId(2);
    // first=20 is at the floor; nudging Left from p1 would drop first to 19.
    let mut node = hsplit(p1, p2, 20, 59);
    let err = nudge_ratio(&mut node, p1, Direction::Left, MIN_PANE_COLS).unwrap_err();
    assert_eq!(err, NudgeError::BelowMinimumSize);
    let r = ratio_of(&node);
    assert_eq!(r.first, 20);
    assert_eq!(r.second, 59);
}

#[test]
fn nudge_with_no_matching_split_returns_error() {
    let p1 = PaneId(1);
    let mut node = PaneNode::Leaf(p1);
    let err = nudge_ratio(&mut node, p1, Direction::Right, MIN_PANE_COLS).unwrap_err();
    assert_eq!(err, NudgeError::NoSplitInDirection);
}

#[test]
fn nudge_finds_nearest_ancestor_split() {
    // VSplit{ HSplit{Leaf(1), Leaf(2)}, Leaf(3), ratio 12:11 }
    // From p1, Direction::Right -> must find inner HSplit (not outer VSplit which is U/D axis).
    let p1 = PaneId(1);
    let p2 = PaneId(2);
    let p3 = PaneId(3);
    let mut root = PaneNode::VSplit {
        top: Box::new(PaneNode::HSplit {
            left: Box::new(PaneNode::Leaf(p1)),
            right: Box::new(PaneNode::Leaf(p2)),
            ratio: SplitRatio {
                first: 40,
                second: 39,
            },
        }),
        bottom: Box::new(PaneNode::Leaf(p3)),
        ratio: SplitRatio {
            first: 12,
            second: 11,
        },
    };
    nudge_ratio(&mut root, p1, Direction::Right, MIN_PANE_COLS).expect("found inner HSplit");
    if let PaneNode::VSplit { top, ratio, .. } = &root {
        // Outer VSplit ratio is unchanged.
        assert_eq!(ratio.first, 12);
        assert_eq!(ratio.second, 11);
        // Inner HSplit's first/second shifted right by 1.
        let inner_r = ratio_of(top);
        assert_eq!(inner_r.first, 41);
        assert_eq!(inner_r.second, 38);
    } else {
        panic!("root should still be VSplit");
    }
}
