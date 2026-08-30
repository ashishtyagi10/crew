//! Braille, U+2800–U+28FF — a 2×4 grid of dots in one cell.
//!
//! Crew spins its own busy indicator out of these, but the reason to draw
//! them rather than read them is what runs INSIDE a pane: btop, gotop,
//! bandwhich and every other monitor of that generation plot their graphs in
//! braille, because eight dots per cell is four times the vertical resolution
//! a block ramp gives. A whole chart made of font glyphs is a whole chart
//! taking the letterform pipeline — rasterized at whatever position the
//! typeface put its dots, dilated sideways, and lifted at the rim.
//!
//! Drawn instead, the eight dots are on a grid the cell owns, so a rising
//! line reads as a rising line and two adjacent cells' dots sit in the same
//! columns. Each dot is a square rather than a disc: at the four-pixel
//! sub-cell a terminal actually gives it, a disc is a square with its corners
//! antialiased into a smudge, and the smudge is what a plotted line is made
//! of here.
use super::Mask;

/// Bit `i` of the code point's low byte, as `(column, row)` in the 2×4 grid.
/// The first six run down the left column then the right — the historical
/// six-dot layout — and the last two are the eight-dot row underneath.
const DOTS: [(u32, u32); 8] = [
    (0, 0),
    (0, 1),
    (0, 2),
    (1, 0),
    (1, 1),
    (1, 2),
    (0, 3),
    (1, 3),
];

/// Draw `c` if it is a braille pattern. U+2800 itself is blank — a real
/// character with no dots, and one crew's spinner passes through — so it is
/// claimed and drawn as nothing rather than falling back to the font, which
/// would draw a blank of a different width.
pub(super) fn draw(m: &mut Mask, c: char) -> bool {
    let code = c as u32;
    if !(0x2800..=0x28FF).contains(&code) {
        return false;
    }
    let bits = code - 0x2800;
    // Sub-cell size, rounded so the four rows tile the cell exactly rather
    // than leaving a stripe of remainder at the bottom.
    let step = |extent: u32, i: u32, n: u32| (extent * i).div_ceil(n);
    // The dot fills three quarters of its sub-cell: large enough that a
    // plotted line reads as a line, small enough that the grid still reads
    // as dots rather than a solid block.
    for (i, (cx, cy)) in DOTS.iter().enumerate() {
        if bits & (1 << i) == 0 {
            continue;
        }
        let (x0, x1) = (step(m.w, *cx, 2), step(m.w, cx + 1, 2));
        let (y0, y1) = (step(m.h, *cy, 4), step(m.h, cy + 1, 4));
        let d = (x1 - x0).min(y1 - y0);
        let side = (d * 3).div_ceil(4).max(1);
        let (ox, oy) = ((x1 - x0 - side) / 2, (y1 - y0 - side) / 2);
        let (px, py) = (x0 + ox, y0 + oy);
        m.rect(px as f32, py as f32, (px + side) as f32, (py + side) as f32);
    }
    true
}
