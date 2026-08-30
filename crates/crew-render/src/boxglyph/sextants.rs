//! Sextants — U+1FB00–U+1FB3B, the 2×3 cell grid.
//!
//! The newest of the cell-graphics families and the reason a modern TUI can
//! plot at six times a cell's resolution without braille's dot gaps: every
//! combination of six half-cells, so a filled shape made of them is actually
//! filled. Charting libraries reach for them, and a pane running one was
//! drawing every cell of its plot from a font — sixty characters whose whole
//! definition is "these sixths of the cell are on".
//!
//! The encoding is a six-bit pattern read left-to-right, top-to-bottom, laid
//! out over the code points in order with four holes in it: the empty and
//! full patterns are `space` and `█`, and the two single-column ones are `▌`
//! and `▐`, so those four are not given sextant code points and everything
//! after them shifts down.
use super::Mask;

/// The patterns that are NOT sextants because another character already is
/// them: 0 is a space, 21 (`0b010101`) is `▌`, 42 (`0b101010`) is `▐`, and 63
/// is `█`.
const TAKEN: [u32; 2] = [21, 42];

/// The six-bit pattern behind a sextant code point, or `None` when it is not
/// one. Counting rather than a table: the sequence is patterns 1..=62 with
/// two holes, and sixty table rows would say the same thing at ten times the
/// size.
fn pattern_of(c: char) -> Option<u32> {
    let code = c as u32;
    if !(0x1FB00..=0x1FB3B).contains(&code) {
        return None;
    }
    let want = code - 0x1FB00;
    let mut seen = 0;
    for p in 1..63 {
        if TAKEN.contains(&p) {
            continue;
        }
        if seen == want {
            return Some(p);
        }
        seen += 1;
    }
    None
}

/// Draw `c` if it is a sextant.
pub(super) fn draw(m: &mut Mask, c: char) -> bool {
    let Some(bits) = pattern_of(c) else {
        return false;
    };
    // Thirds rounded so the three rows tile the cell exactly — a remainder
    // stripe at the bottom is what makes a plot made of these look striped.
    let (mw, mh) = (m.w, m.h);
    let row = |i: u32| (mh * i).div_ceil(3) as f32;
    let col = |i: u32| (mw * i).div_ceil(2) as f32;
    for i in 0..6u32 {
        if bits & (1 << i) == 0 {
            continue;
        }
        let (cx, cy) = (i % 2, i / 2);
        m.rect(col(cx), row(cy), col(cx + 1), row(cy + 1));
    }
    true
}
