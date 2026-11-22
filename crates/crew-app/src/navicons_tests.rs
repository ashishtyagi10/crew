//! The drawn icons.
use super::*;

fn all() -> Vec<BarHit> {
    let mut v: Vec<BarHit> = NavSection::ALL.into_iter().map(BarHit::Section).collect();
    v.push(BarHit::Settings);
    v
}

/// Every icon is a shape — some of its box inside it — and stays inside its box: an icon that
/// spilled past its half-size would draw over the badge or the next icon.
#[test]
fn every_icon_draws_inside_its_own_box() {
    let (at, u) = ((0.0, 0.0), 1.3);
    for hit in all() {
        let mut inside = 0;
        for iy in -30..=30 {
            for ix in -30..=30 {
                let p = (ix as f32 * 0.05, iy as f32 * 0.05);
                let d = shape(hit, p, at, u);
                if d < 0.0 {
                    inside += 1;
                    assert!(
                        p.0.abs() <= u + 0.1 && p.1.abs() <= u + 0.1,
                        "{hit:?} at {p:?}"
                    );
                }
            }
        }
        assert!(inside > 30, "{hit:?} draws almost nothing: {inside}");
    }
}

/// No two icons are the same picture.
#[test]
fn no_two_icons_are_alike() {
    let (at, u) = ((0.0, 0.0), 1.3);
    let mask = |hit| -> Vec<bool> {
        (-20..=20)
            .flat_map(|iy| (-20..=20).map(move |ix| (ix as f32 * 0.07, iy as f32 * 0.07)))
            .map(|p| shape(hit, p, at, u) < 0.0)
            .collect()
    };
    let masks: Vec<Vec<bool>> = all().into_iter().map(mask).collect();
    for i in 0..masks.len() {
        for j in i + 1..masks.len() {
            let same = masks[i]
                .iter()
                .zip(&masks[j])
                .filter(|(a, b)| a == b)
                .count();
            assert!(
                same < masks[i].len() * 95 / 100,
                "{:?} ~ {:?}",
                all()[i],
                all()[j]
            );
        }
    }
}

/// The paint lands on each icon's row, in the bar's columns.
#[test]
fn the_paint_is_where_the_bar_puts_each_icon() {
    let _g = crate::app::theme_test_guard();
    let rows = 30;
    let paint = bar_paint(Some(NavSection::System), None, rows, 2.0);
    for hit in all() {
        let (cx, cy) = centre(hit, rows).unwrap();
        assert!(
            paint
                .iter()
                .any(|p| (p.y + p.h * 0.5 - cy).abs() < 0.7 && (p.x - cx).abs() < 1.5),
            "{hit:?}"
        );
    }
    assert!(paint
        .iter()
        .all(|p| p.x >= 0.0 && p.x + p.w <= f32::from(crate::navbar::BAR_COLS)));
}

/// No icon is cut by the card's edge: the first one used to sit on row 0, half of it above the
/// card, and the gear on the last row, half below it.
#[test]
fn no_icon_leaves_the_bar_at_any_height() {
    let _g = crate::app::theme_test_guard();
    for rows in [10u16, 14, 24, 40] {
        for aspect in [1.8f32, 2.1, 2.4] {
            for p in bar_paint(None, None, rows, aspect) {
                assert!(
                    p.y >= 0.0 && p.y + p.h <= f32::from(rows) + 0.01,
                    "rows={rows}: {p:?}"
                );
            }
        }
    }
}
