//! Cell width per UAX #11. Source of truth — never font advance (Pitfall 2).

use unicode_width::UnicodeWidthChar;

/// 1 default, 0 combining/zero-width, 2 wide.
#[must_use]
pub fn cell_width(c: char) -> u8 {
    u8::try_from(c.width().unwrap_or(1)).unwrap_or(1)
}
