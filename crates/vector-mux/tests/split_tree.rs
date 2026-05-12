//! WIN-03: Cmd-D / Cmd-Shift-D tree mutation. Plan 04-02.

use vector_mux::{
    compute_layout, split_at_leaf, PaneId, PaneNode, Rect, SplitDirection, SplitError,
};

#[test]
fn split_horizontal_at_leaf_returns_hsplit() {
    let p1 = PaneId(1);
    let p2 = PaneId(2);
    let viewport = Rect {
        x: 0,
        y: 0,
        w: 80,
        h: 24,
    };
    let result = split_at_leaf(
        PaneNode::Leaf(p1),
        p1,
        p2,
        SplitDirection::Horizontal,
        viewport,
    )
    .expect("80x24 viewport accommodates 2x20-col split");
    match result {
        PaneNode::HSplit { left, right, ratio } => {
            assert!(matches!(*left, PaneNode::Leaf(id) if id == p1));
            assert!(matches!(*right, PaneNode::Leaf(id) if id == p2));
            // 80 / 2 = 40 first; 80 - 40 - 1 = 39 second.
            assert_eq!(ratio.first, 40);
            assert_eq!(ratio.second, 39);
        }
        other => panic!("expected HSplit, got {other:?}"),
    }
}

#[test]
fn split_vertical_inside_hsplit_nests_correctly() {
    let p1 = PaneId(1);
    let p2 = PaneId(2);
    let p3 = PaneId(3);
    let viewport = Rect {
        x: 0,
        y: 0,
        w: 80,
        h: 24,
    };
    // First: split p1 horizontally with p2.
    let step1 = split_at_leaf(
        PaneNode::Leaf(p1),
        p1,
        p2,
        SplitDirection::Horizontal,
        viewport,
    )
    .expect("step1");
    // Then: split p2 vertically with p3.
    let step2 = split_at_leaf(step1, p2, p3, SplitDirection::Vertical, viewport).expect("step2");
    match step2 {
        PaneNode::HSplit { left, right, .. } => {
            assert!(matches!(*left, PaneNode::Leaf(id) if id == p1));
            match *right {
                PaneNode::VSplit { top, bottom, .. } => {
                    assert!(matches!(*top, PaneNode::Leaf(id) if id == p2));
                    assert!(matches!(*bottom, PaneNode::Leaf(id) if id == p3));
                }
                other => panic!("expected nested VSplit, got {other:?}"),
            }
        }
        other => panic!("expected outer HSplit, got {other:?}"),
    }
}

#[test]
fn split_below_minimum_size_is_rejected() {
    let p1 = PaneId(1);
    let p2 = PaneId(2);
    // Viewport width 30 — too small for two 20-col halves (need ≥ 2*20+1 = 41).
    let viewport = Rect {
        x: 0,
        y: 0,
        w: 30,
        h: 24,
    };
    let result = split_at_leaf(
        PaneNode::Leaf(p1),
        p1,
        p2,
        SplitDirection::Horizontal,
        viewport,
    );
    assert_eq!(result.unwrap_err(), SplitError::BelowMinimum);
}

#[test]
fn compute_layout_three_panes_horizontal_sums_correctly() {
    let p1 = PaneId(1);
    let p2 = PaneId(2);
    let p3 = PaneId(3);
    let viewport = Rect {
        x: 0,
        y: 0,
        w: 120,
        h: 24,
    };
    // Step 1: split p1 horizontally -> p2 (first=60 second=59).
    let n1 = split_at_leaf(
        PaneNode::Leaf(p1),
        p1,
        p2,
        SplitDirection::Horizontal,
        viewport,
    )
    .expect("first split");
    // Step 2: split p2 horizontally -> p3 (p2's pane has 59 cols >= 41 floor).
    let n2 = split_at_leaf(n1, p2, p3, SplitDirection::Horizontal, viewport).expect("second split");
    let layout = compute_layout(&n2, viewport);
    let r1 = layout[&p1];
    let r2 = layout[&p2];
    let r3 = layout[&p3];
    assert_eq!(r1.h, 24);
    assert_eq!(r2.h, 24);
    assert_eq!(r3.h, 24);
    // Total = 120; 2 dividers consume 2 cells; visible cells sum to 118.
    assert_eq!(u32::from(r1.w) + u32::from(r2.w) + u32::from(r3.w) + 2, 120);
}
