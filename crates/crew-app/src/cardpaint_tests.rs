//! The frame's drawn readings, against the strokes they ride.
use super::{card_paint, thumb_span, tint};
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

/// The stroke under the thumb wears the thumb's colour, so the bar is one
/// colour across: the glyph is drawn over the paint, and centred on it the
/// thumb was pin-striped by the frame (glass survey G1). Rows it barely
/// touches, and the rest of the border, keep the frame's colour.
#[test]
fn the_stroke_under_the_thumb_takes_its_colour() {
    let _g = crate::app::theme_test_guard();
    let b = Bar {
        total: 400,
        scroll: 200,
        ..Default::default()
    };
    let (cols, rows) = (40u16, 20u16);
    let mut cells = crate::panecard::pane_card(cols - 2, rows - 2, &b);
    let before = cells.clone();
    tint(&mut cells, cols, rows, &b);
    let (top, h) = thumb_span(rows, &b).unwrap();
    let thumb = crate::panescroll::position_fg(crate::panescroll::position(400, 18, 200));
    let side = |v: &[crew_render::CellView], r: u16| {
        v.iter()
            .find(|c| c.col == cols - 1 && c.row == r)
            .map(|c| c.fg)
    };
    for r in 1..rows - 1 {
        let (lo, hi) = (f32::from(r), f32::from(r) + 1.0);
        if lo >= top && hi <= top + h {
            assert_eq!(side(&cells, r), Some(thumb), "row {r} is under the thumb");
        } else if hi <= top || lo >= top + h {
            assert_eq!(side(&cells, r), side(&before, r), "row {r} is not");
        }
    }
}

/// On a soft theme the thumb is a slot, not a bar: thinner ink than the
/// stroked themes draw, and a lit lip beside it.
#[test]
fn a_relief_card_cuts_the_thumb_as_a_slot() {
    let _g = crate::app::theme_test_guard();
    let b = Bar {
        total: 400,
        scroll: 200,
        ..Default::default()
    };
    let strongest = |ps: &[Paint]| ps.iter().map(|p| p.alpha).fold(0.0_f32, f32::max);
    let stroked = card_paint(40, 20, &b, (CW, CH), 0);
    crew_theme::set_theme(crew_theme::ThemeId::Porcelain);
    let soft = card_paint(40, 20, &b, (CW, CH), 0);
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    assert!(strongest(&stroked) > 0.9);
    let lit = soft.iter().filter(|p| p.color.0 > 240 && p.color.1 > 240);
    assert!(lit.count() > 0, "the far lip catches the light");
    let dark: Vec<Paint> = soft.into_iter().filter(|p| p.color.0 < 200).collect();
    assert!(!dark.is_empty() && strongest(&dark) < 0.6, "thinned ink");
}
