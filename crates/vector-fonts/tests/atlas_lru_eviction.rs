//! Pure-Rust LRU bookkeeping smoke. The wgpu-backed integration lives in
//! vector-render/tests/atlas_lru.rs.

use std::collections::VecDeque;

fn touch(lru: &mut VecDeque<char>, c: char) {
    if let Some(pos) = lru.iter().position(|&k| k == c) {
        lru.remove(pos);
    }
    lru.push_back(c);
}

#[test]
fn lru_moves_touched_key_to_back() {
    let mut lru: VecDeque<char> = VecDeque::from(['a', 'b', 'c', 'd']);
    touch(&mut lru, 'a');
    assert_eq!(lru.back(), Some(&'a'), "touched key must move to back");
    assert_eq!(lru.front(), Some(&'b'), "next-oldest moves to front");
}

#[test]
fn lru_pop_front_returns_oldest() {
    let mut lru: VecDeque<char> = VecDeque::from(['a', 'b', 'c']);
    assert_eq!(lru.pop_front(), Some('a'));
    assert_eq!(lru.pop_front(), Some('b'));
}
