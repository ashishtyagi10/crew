//! The frame's drawn readings, against the strokes they ride.
use super::card_paint;
use crate::panecard::Bar;
use crew_render::Paint;

const CW: f32 = 8.0;
const CH: f32 = 16.0;

/// The paint-weighted centre of `ps` in px on a CW×CH cell.
fn centre(ps: &[Paint]) -> (f32, f32) {
    let wt = |p: &Paint| p.w * p.h * p.alpha;
    let sum: f32 = ps.iter().map(wt).sum();
    let at = |f: &dyn Fn(&Paint) -> f32| ps.iter().map(|p| f(p) * wt(p)).sum::<f32>() / sum;
    (
        at(&|p| (p.x + p.w / 2.0) * CW),
        at(&|p| (p.y + p.h / 2.0) * CH),
    )
}

/// The thumb and the progress bar ride their strokes, which run at the
/// cell's middle rounded down (px 3 of 8 across, 7 of 16 down) — centred
/// on the cell's middle they hung a pixel outside the card (glass survey
/// F11).
#[test]
fn the_border_readings_are_centred_on_their_strokes() {
    let _g = crate::app::theme_test_guard();
    let b = Bar {
        total: 400,
        scroll: 200,
        progress: Some(crew_term::Progress {
            percent: Some(50),
            alarm: false,
        }),
        ..Default::default()
    };
    let ps = card_paint(40, 20, &b, (CW, CH), 0);
    let (side, bottom): (Vec<_>, Vec<_>) = ps.into_iter().partition(|p| p.x >= 39.0);
    assert!(
        (centre(&side).0 - 39.0 * CW - 3.5).abs() < 0.25,
        "{:?}",
        centre(&side)
    );
    assert!(
        (centre(&bottom).1 - 19.0 * CH - 7.5).abs() < 0.25,
        "{:?}",
        centre(&bottom)
    );
}
