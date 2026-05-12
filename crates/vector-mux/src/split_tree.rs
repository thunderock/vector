//! Split-tree pure algorithms: layout, mutation, directional focus, resize-nudge.
//!
//! All functions are pure over `&PaneNode` / `&mut PaneNode` plus a viewport
//! `Rect`. No Mux dependency — Mux delegates to these.

use std::collections::HashMap;

use crate::ids::{
    Direction, NudgeError, PaneId, SplitDirection, SplitError, MIN_PANE_COLS, MIN_PANE_ROWS,
};
use crate::pane::{PaneNode, SplitRatio};
use crate::tab::Tab;

/// Pixel-free cell rectangle. x/y are cell offsets from viewport origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

/// Walk the tree and assign each Leaf a rectangle inside `viewport`.
/// HSplit divider takes 1 cell of width; VSplit divider takes 1 cell of height.
#[must_use]
pub fn compute_layout(root: &PaneNode, viewport: Rect) -> HashMap<PaneId, Rect> {
    let mut out = HashMap::new();
    walk_layout(root, viewport, &mut out);
    out
}

fn walk_layout(node: &PaneNode, rect: Rect, out: &mut HashMap<PaneId, Rect>) {
    match node {
        PaneNode::Leaf(id) => {
            out.insert(*id, rect);
        }
        PaneNode::HSplit { left, right, ratio } => {
            let left_rect = Rect {
                x: rect.x,
                y: rect.y,
                w: ratio.first,
                h: rect.h,
            };
            // Divider sits at x + first; right starts at x + first + 1.
            let right_rect = Rect {
                x: rect.x.saturating_add(ratio.first).saturating_add(1),
                y: rect.y,
                w: ratio.second,
                h: rect.h,
            };
            walk_layout(left, left_rect, out);
            walk_layout(right, right_rect, out);
        }
        PaneNode::VSplit { top, bottom, ratio } => {
            let top_rect = Rect {
                x: rect.x,
                y: rect.y,
                w: rect.w,
                h: ratio.first,
            };
            let bot_rect = Rect {
                x: rect.x,
                y: rect.y.saturating_add(ratio.first).saturating_add(1),
                w: rect.w,
                h: ratio.second,
            };
            walk_layout(top, top_rect, out);
            walk_layout(bottom, bot_rect, out);
        }
    }
}

/// Bisect the leaf carrying `target` into a `dir`-split with `new_pane` on the
/// far side. Returns Err(BelowMinimum) if the resulting halves would violate
/// MIN_PANE_COLS/ROWS. Returns Err(PaneNotFound) if `target` is not in the tree.
pub fn split_at_leaf(
    node: PaneNode,
    target: PaneId,
    new_pane: PaneId,
    dir: SplitDirection,
    viewport: Rect,
) -> Result<PaneNode, SplitError> {
    // First compute the leaf's current rect so we know the size we're bisecting.
    let layout = compute_layout(&node, viewport);
    let target_rect = layout
        .get(&target)
        .copied()
        .ok_or(SplitError::PaneNotFound)?;

    // Bisect with the divider taking 1 cell.
    let (first, second) = match dir {
        SplitDirection::Horizontal => {
            if target_rect.w < 2 * MIN_PANE_COLS + 1 {
                return Err(SplitError::BelowMinimum);
            }
            let first = target_rect.w / 2;
            let second = target_rect.w - first - 1;
            (first, second)
        }
        SplitDirection::Vertical => {
            if target_rect.h < 2 * MIN_PANE_ROWS + 1 {
                return Err(SplitError::BelowMinimum);
            }
            let first = target_rect.h / 2;
            let second = target_rect.h - first - 1;
            (first, second)
        }
    };

    let ratio = SplitRatio { first, second };
    Ok(replace_leaf(node, target, new_pane, dir, ratio))
}

fn replace_leaf(
    node: PaneNode,
    target: PaneId,
    new_pane: PaneId,
    dir: SplitDirection,
    ratio: SplitRatio,
) -> PaneNode {
    match node {
        PaneNode::Leaf(id) if id == target => match dir {
            SplitDirection::Horizontal => PaneNode::HSplit {
                left: Box::new(PaneNode::Leaf(id)),
                right: Box::new(PaneNode::Leaf(new_pane)),
                ratio,
            },
            SplitDirection::Vertical => PaneNode::VSplit {
                top: Box::new(PaneNode::Leaf(id)),
                bottom: Box::new(PaneNode::Leaf(new_pane)),
                ratio,
            },
        },
        PaneNode::Leaf(_) => node,
        PaneNode::HSplit {
            left,
            right,
            ratio: r,
        } => PaneNode::HSplit {
            left: Box::new(replace_leaf(*left, target, new_pane, dir, ratio)),
            right: Box::new(replace_leaf(*right, target, new_pane, dir, ratio)),
            ratio: r,
        },
        PaneNode::VSplit {
            top,
            bottom,
            ratio: r,
        } => PaneNode::VSplit {
            top: Box::new(replace_leaf(*top, target, new_pane, dir, ratio)),
            bottom: Box::new(replace_leaf(*bottom, target, new_pane, dir, ratio)),
            ratio: r,
        },
    }
}

/// Drop `target` from the tree by collapsing its parent split into the sibling.
/// Returns the new root; on the last-leaf case (root was the target itself)
/// returns None to signal "tab is empty, cascade up".
#[must_use]
pub fn remove_leaf(node: PaneNode, target: PaneId) -> Option<PaneNode> {
    match node {
        PaneNode::Leaf(id) => {
            if id == target {
                None
            } else {
                Some(PaneNode::Leaf(id))
            }
        }
        PaneNode::HSplit { left, right, ratio } => {
            collapse_split(*left, *right, target, |l, r| PaneNode::HSplit {
                left: Box::new(l),
                right: Box::new(r),
                ratio,
            })
        }
        PaneNode::VSplit { top, bottom, ratio } => {
            collapse_split(*top, *bottom, target, |t, b| PaneNode::VSplit {
                top: Box::new(t),
                bottom: Box::new(b),
                ratio,
            })
        }
    }
}

fn collapse_split<F>(a: PaneNode, b: PaneNode, target: PaneId, rebuild: F) -> Option<PaneNode>
where
    F: Fn(PaneNode, PaneNode) -> PaneNode,
{
    let a_has = a.contains(target);
    let b_has = b.contains(target);
    match (a_has, b_has) {
        (true, _) => match remove_leaf(a, target) {
            Some(new_a) => Some(rebuild(new_a, b)),
            None => Some(b),
        },
        (_, true) => match remove_leaf(b, target) {
            Some(new_b) => Some(rebuild(a, new_b)),
            None => Some(a),
        },
        (false, false) => Some(rebuild(a, b)),
    }
}

/// WezTerm `get_pane_direction` simplification: edge-overlap scoring, lowest-PaneId tie-break.
#[must_use]
pub fn get_pane_direction(tab: &Tab, from: PaneId, dir: Direction) -> Option<PaneId> {
    let viewport = Rect {
        x: 0,
        y: 0,
        w: tab.last_cols,
        h: tab.last_rows,
    };
    let layout = compute_layout(&tab.root, viewport);
    let from_rect = *layout.get(&from)?;

    let mut best: Option<(u32, PaneId)> = None; // (overlap_score, lowest-tiebreak id)
    for (id, rect) in &layout {
        if *id == from {
            continue;
        }
        if let Some(overlap) = edge_overlap(from_rect, *rect, dir) {
            let candidate = (overlap, *id);
            match best {
                None => best = Some(candidate),
                Some((cur_overlap, cur_id)) => {
                    if overlap > cur_overlap || (overlap == cur_overlap && *id < cur_id) {
                        best = Some(candidate);
                    }
                }
            }
        }
    }
    best.map(|(_, id)| id)
}

/// Return the edge-overlap length in cells if `candidate` is on the `dir` side
/// of `from`, sharing an adjacency edge. None if not adjacent in that direction.
fn edge_overlap(from: Rect, candidate: Rect, dir: Direction) -> Option<u32> {
    match dir {
        Direction::Right => {
            // candidate's left edge must be exactly at from's right edge + 1 (divider).
            let expected_x = u32::from(from.x) + u32::from(from.w) + 1;
            if u32::from(candidate.x) != expected_x {
                return None;
            }
            vertical_overlap(from, candidate)
        }
        Direction::Left => {
            // from's left edge must be at candidate's right edge + 1.
            let expected_x = u32::from(candidate.x) + u32::from(candidate.w) + 1;
            if u32::from(from.x) != expected_x {
                return None;
            }
            vertical_overlap(from, candidate)
        }
        Direction::Down => {
            let expected_y = u32::from(from.y) + u32::from(from.h) + 1;
            if u32::from(candidate.y) != expected_y {
                return None;
            }
            horizontal_overlap(from, candidate)
        }
        Direction::Up => {
            let expected_y = u32::from(candidate.y) + u32::from(candidate.h) + 1;
            if u32::from(from.y) != expected_y {
                return None;
            }
            horizontal_overlap(from, candidate)
        }
    }
}

fn vertical_overlap(a: Rect, b: Rect) -> Option<u32> {
    let a_top = u32::from(a.y);
    let a_bot = u32::from(a.y) + u32::from(a.h);
    let b_top = u32::from(b.y);
    let b_bot = u32::from(b.y) + u32::from(b.h);
    let lo = a_top.max(b_top);
    let hi = a_bot.min(b_bot);
    if hi > lo {
        Some(hi - lo)
    } else {
        None
    }
}

fn horizontal_overlap(a: Rect, b: Rect) -> Option<u32> {
    let a_left = u32::from(a.x);
    let a_right = u32::from(a.x) + u32::from(a.w);
    let b_left = u32::from(b.x);
    let b_right = u32::from(b.x) + u32::from(b.w);
    let lo = a_left.max(b_left);
    let hi = a_right.min(b_right);
    if hi > lo {
        Some(hi - lo)
    } else {
        None
    }
}

/// Walk down to `target`'s leaf; on the way up find the nearest ancestor split
/// whose orientation matches `dir`'s axis (HSplit for L/R, VSplit for U/D);
/// shift its ratio by 1 cell. `min_cells` enforces the per-side floor.
pub fn nudge_ratio(
    node: &mut PaneNode,
    target: PaneId,
    dir: Direction,
    min_cells: u16,
) -> Result<(), NudgeError> {
    match nudge_walk(node, target, dir, min_cells) {
        NudgeOutcome::Done => Ok(()),
        NudgeOutcome::Err(e) => Err(e),
        NudgeOutcome::NotFound => Err(NudgeError::NoSplitInDirection),
    }
}

enum NudgeOutcome {
    Done,
    Err(NudgeError),
    NotFound,
}

fn nudge_walk(node: &mut PaneNode, target: PaneId, dir: Direction, min_cells: u16) -> NudgeOutcome {
    let axis_h = matches!(dir, Direction::Left | Direction::Right);
    match node {
        PaneNode::Leaf(_) => NudgeOutcome::NotFound,
        PaneNode::HSplit { left, right, ratio } => {
            let in_left = left.contains(target);
            let in_right = right.contains(target);
            if !in_left && !in_right {
                return NudgeOutcome::NotFound;
            }
            let inner = if in_left {
                nudge_walk(left, target, dir, min_cells)
            } else {
                nudge_walk(right, target, dir, min_cells)
            };
            match inner {
                NudgeOutcome::Done | NudgeOutcome::Err(_) => return inner,
                NudgeOutcome::NotFound => {}
            }
            if axis_h {
                // HSplit + L/R: shift ratio.first by ±1. From a leaf inside `left`,
                // Direction::Right grows first (push the divider rightward).
                let delta: i32 = match (in_left, dir) {
                    (true, Direction::Right) | (false, Direction::Left) => 1,
                    (true, Direction::Left) | (false, Direction::Right) => -1,
                    _ => 0,
                };
                apply_ratio_delta(ratio, delta, min_cells)
            } else {
                NudgeOutcome::NotFound
            }
        }
        PaneNode::VSplit { top, bottom, ratio } => {
            let in_top = top.contains(target);
            let in_bot = bottom.contains(target);
            if !in_top && !in_bot {
                return NudgeOutcome::NotFound;
            }
            let inner = if in_top {
                nudge_walk(top, target, dir, min_cells)
            } else {
                nudge_walk(bottom, target, dir, min_cells)
            };
            match inner {
                NudgeOutcome::Done | NudgeOutcome::Err(_) => return inner,
                NudgeOutcome::NotFound => {}
            }
            if axis_h {
                NudgeOutcome::NotFound
            } else {
                let delta: i32 = match (in_top, dir) {
                    (true, Direction::Down) | (false, Direction::Up) => 1,
                    (true, Direction::Up) | (false, Direction::Down) => -1,
                    _ => 0,
                };
                apply_ratio_delta(ratio, delta, min_cells)
            }
        }
    }
}

fn apply_ratio_delta(ratio: &mut SplitRatio, delta: i32, min_cells: u16) -> NudgeOutcome {
    let new_first = i32::from(ratio.first) + delta;
    let new_second = i32::from(ratio.second) - delta;
    if new_first < i32::from(min_cells) || new_second < i32::from(min_cells) {
        return NudgeOutcome::Err(NudgeError::BelowMinimumSize);
    }
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    {
        ratio.first = new_first as u16;
        ratio.second = new_second as u16;
    }
    NudgeOutcome::Done
}

/// Proportionally redistribute split ratios to fit a new viewport. Preserves
/// the relative `first / (first + second)` proportion per split, ratchets to
/// integer cells, and re-asserts the `first + second + 1 == axis_size` invariant.
pub fn redistribute(node: &mut PaneNode, viewport: Rect) {
    match node {
        PaneNode::Leaf(_) => {}
        PaneNode::HSplit { left, right, ratio } => {
            let total = viewport.w.saturating_sub(1); // 1 cell for divider
            let prev = u32::from(ratio.first) + u32::from(ratio.second);
            let new_first = if prev == 0 {
                total / 2
            } else {
                #[allow(clippy::cast_possible_truncation)]
                let v = (u32::from(ratio.first) * u32::from(total) / prev) as u16;
                v
            };
            let new_second = total.saturating_sub(new_first);
            ratio.first = new_first;
            ratio.second = new_second;
            let left_rect = Rect {
                w: new_first,
                ..viewport
            };
            let right_rect = Rect {
                w: new_second,
                ..viewport
            };
            redistribute(left, left_rect);
            redistribute(right, right_rect);
        }
        PaneNode::VSplit { top, bottom, ratio } => {
            let total = viewport.h.saturating_sub(1);
            let prev = u32::from(ratio.first) + u32::from(ratio.second);
            let new_first = if prev == 0 {
                total / 2
            } else {
                #[allow(clippy::cast_possible_truncation)]
                let v = (u32::from(ratio.first) * u32::from(total) / prev) as u16;
                v
            };
            let new_second = total.saturating_sub(new_first);
            ratio.first = new_first;
            ratio.second = new_second;
            let top_rect = Rect {
                h: new_first,
                ..viewport
            };
            let bot_rect = Rect {
                h: new_second,
                ..viewport
            };
            redistribute(top, top_rect);
            redistribute(bottom, bot_rect);
        }
    }
}
