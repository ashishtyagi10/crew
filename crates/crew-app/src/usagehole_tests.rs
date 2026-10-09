//! The ring centres on the total in its hole, whatever its length. Split
//! from `usagepane_tests` (at its line cap).
use super::*;
use crate::usagelayout::{layout, RING_ROW};

/// A cell-placed word centres to half a column, so a one- or three-character
/// total sat half a column right of a ring centred on `RING_CX`; the ring
/// now centres on the word. Measured off what `paint` emitted, against the
/// columns the text took.
#[test]
fn the_ring_centres_on_the_total_in_its_hole() {
    let _g = crate::app::theme_test_guard();
    for (tok_in, tok_out) in [(3, 0), (345, 0), (1_840_000, 410_000)] {
        let b = Buckets {
            hourly: vec![0; DAYS * HOURS],
            daily_cost: vec![0; DAYS],
            tok_in,
            tok_out,
            cost_microusd: 10,
        };
        let l = layout(40);
        let cols: Vec<u16> = cells(&b, 60, 40)
            .into_iter()
            .filter(|c| c.row == l.split_top + RING_ROW && c.col < 13 && c.c != ' ')
            .map(|c| c.col)
            .collect();
        let text_mid = f32::from(cols[0] + *cols.last().unwrap() + 1) / 2.0;
        let band: Vec<_> = paint(&b, 60, 40, 2.0)
            .into_iter()
            .filter(|p| p.y >= f32::from(l.split_top) && p.y < f32::from(l.cost_top))
            .collect();
        let left = band.iter().fold(f32::MAX, |a, p| a.min(p.x));
        let right = band.iter().fold(0.0f32, |a, p| a.max(p.x + p.w));
        let ring_mid = (left + right) / 2.0;
        assert!(
            (ring_mid - text_mid).abs() < 0.15,
            "{tok_in}: ring {ring_mid} text {text_mid}"
        );
    }
}
