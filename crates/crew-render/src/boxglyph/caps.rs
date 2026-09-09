//! The powerline round caps `` / `` (U+E0B6 / U+E0B4) — the two ends of
//! every badge crew draws: a card's sender, a fence's language, the footer's
//! working agents, the plan row's buttons.
//!
//! A cap is a Nerd Font private-use glyph, and the fonts are the reason it
//! is drawn here. Each patched face scales its powerline extras to ITS OWN
//! cell, and crew's cells are not that cell: the arc came back taller than
//! the block it capped on one side and shorter on the other, so a pill read
//! as a block with two mismatched bites out of it — neither round nor
//! square. Drawn, the cap is exactly the cell's height, meets the block's
//! edge at full ink along every row, and is the same shape in every font.
//!
//! The shape is a half-disc bulging away from the block: a true semicircle
//! whenever the cell is at least half as wide as it is tall (every
//! monospace face crew ships or allows), and a semi-ellipse squeezed to the
//! cell's width otherwise — a circle clipped by the cell edge would put a
//! flat cut on the very end a pill exists to round off.
use super::Mask;

/// Left half circle, thick — the LEFT end of a badge, centred on the cell's
/// right edge (nf-ple-left_half_circle_thick).
const LEFT: char = '\u{e0b6}';
/// Right half circle, thick — the RIGHT end of a badge, centred on the
/// cell's left edge (nf-ple-right_half_circle_thick).
const RIGHT: char = '\u{e0b4}';

/// Draw `c` if it is a powerline round cap.
pub(super) fn draw(m: &mut Mask, c: char) -> bool {
    let bulge_left = match c {
        LEFT => true,
        RIGHT => false,
        _ => return false,
    };
    let (w, h) = (m.w as f32, m.h as f32);
    let ry = h / 2.0;
    let rx = w.min(ry);
    // The centre sits on the edge the block is on; the disc bulges the other
    // way. Only that half is drawn — the block's own cell continues the fill.
    let cx = if bulge_left { w } else { 0.0 };
    let cy = ry;
    m.sample(move |x, y| {
        let on_bulge_side = if bulge_left { x <= cx } else { x >= cx };
        let (dx, dy) = ((x - cx) / rx, (y - cy) / ry);
        on_bulge_side && dx * dx + dy * dy <= 1.0
    });
    true
}
