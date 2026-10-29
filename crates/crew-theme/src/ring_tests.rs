//! No two palettes draw one focus ring. The ring is the most-looked-at
//! colour on the canvas (it marks where you are), and `glass-night` shipped
//! with `glass-clear`'s exact blue→pink pair: switching between the two
//! glasses changed the page and the accent and left the ring as it was.
use crate::oklch::distance;
use crate::{ThemeId, ALL_THEMES};

/// How far apart two rings must be: the near-duplicate gate's own number
/// (`no_two_palettes_are_near_duplicates`), a visible step between pairs.
const FLOOR: f32 = 0.05;

type Ring = ((u8, u8, u8), (u8, u8, u8));

/// How far apart two rings are: the mean OKLab distance of their poles,
/// in whichever pairing is closer (a ring drawn b→a is the same ring).
fn gap(a: Ring, b: Ring) -> f32 {
    let same = (distance(a.0, b.0) + distance(a.1, b.1)) / 2.0;
    let flip = (distance(a.0, b.1) + distance(a.1, b.0)) / 2.0;
    same.min(flip)
}

#[test]
fn no_two_palettes_draw_one_focus_ring() {
    let rings: Vec<(ThemeId, Ring)> = ALL_THEMES
        .into_iter()
        .filter_map(|id| id.theme().modern.map(|m| (id, (m.pole_a, m.pole_b))))
        .collect();
    assert!(rings.len() >= 13, "only {} rings", rings.len());
    for (i, a) in rings.iter().enumerate() {
        for b in &rings[i + 1..] {
            let d = gap(a.1, b.1);
            assert!(
                d >= FLOOR,
                "{} and {} draw one ring (Δ {d:.3})",
                a.0.as_str(),
                b.0.as_str()
            );
        }
    }
}
