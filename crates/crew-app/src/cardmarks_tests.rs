use super::{marks, notches};
use crate::panecard::Bar;

/// An 8-row content grid (a 10-row card) in a 8×16 px cell.
const ROWS: u16 = 10;
const CW: f32 = 8.0;
const CH: f32 = 16.0;

fn bar<'a>(cmd: &'a [u16], fail: &'a [u16], err: &'a [u16]) -> Bar<'a> {
    Bar {
        cmd_rows: cmd,
        fail_rows: fail,
        err_rows: err,
        ..Default::default()
    }
}

/// The px rows a paint covers on the card: `[top, bottom)`.
fn px(p: &crew_render::Paint) -> (f32, f32) {
    (p.y * CH, (p.y + p.h) * CH)
}

/// The content sits 6 px lower than the card's rows (half its remainder):
/// the bar on content row 3 covers that line's own 16 px — not the card's
/// row 4, which started 6 px higher and took the bottom of the line above
/// (glass survey E3: the error bar straddled the `warning:` line).
#[test]
fn an_error_bar_covers_its_line_where_the_line_is_drawn() {
    let _g = crate::app::theme_test_guard();
    let dy = 6.0 / CH;
    let drawn = marks(ROWS, &bar(&[], &[], &[3]), dy, CW, CH);
    assert_eq!(drawn.len(), 1);
    // Content row 3: one row of frame, three rows of content, the slack.
    assert_eq!(
        px(&drawn[0]),
        (16.0 + 3.0 * 16.0 + 6.0, 16.0 + 4.0 * 16.0 + 6.0)
    );
    assert_eq!(drawn[0].color, crew_theme::theme().bell);
    assert_eq!((drawn[0].x, drawn[0].w), (0.0, 0.5), "the cell's left half");
}

/// A command's tick runs across the middle of its line, from the stroke in.
#[test]
fn a_command_tick_crosses_the_middle_of_its_line() {
    let _g = crate::app::theme_test_guard();
    let dy = 6.0 / CH;
    let drawn = marks(ROWS, &bar(&[2], &[], &[]), dy, CW, CH);
    assert_eq!(drawn.len(), 1);
    let (top, bottom) = px(&drawn[0]);
    let line = 16.0 + 2.0 * 16.0 + 6.0;
    assert!(top > line + 6.0 && bottom < line + 10.0, "{top}..{bottom}");
    assert_eq!(drawn[0].color, crew_theme::theme().legend_off);
    assert!(
        (drawn[0].x + drawn[0].w - 1.0).abs() < 1e-4,
        "reaches into the card"
    );
    // A failed block's tick is the alarm's, and heavier.
    let failed = marks(ROWS, &bar(&[], &[2], &[]), dy, CW, CH);
    assert_eq!(failed[0].color, crew_theme::theme().bell);
    assert!(failed[0].h > drawn[0].h);
}

/// "This failed" outranks "this began": a row that is both wears the bar.
#[test]
fn an_error_outranks_a_command_start_on_the_same_row() {
    let _g = crate::app::theme_test_guard();
    let drawn = marks(ROWS, &bar(&[2], &[2], &[2]), 0.0, CW, CH);
    assert_eq!(drawn.len(), 1, "{drawn:?}");
    assert_eq!(drawn[0].w, 0.5);
}

/// No mark lands on the bottom border's row, or runs into it — past it the
/// stretched frame would pull the mark's far edge down with the corner.
#[test]
fn no_mark_reaches_the_bottom_border() {
    let _g = crate::app::theme_test_guard();
    let dy = 6.0 / CH;
    let drawn = marks(ROWS, &bar(&[7, 8, 99], &[], &[7, 8, 99]), dy, CW, CH);
    assert_eq!(drawn.len(), 1, "only content row 7 is on the card");
    assert!(drawn[0].y + drawn[0].h <= f32::from(ROWS - 1) + 1e-4);
    assert!(marks(ROWS, &bar(&[], &[], &[]), dy, CW, CH).is_empty());
}

/// The glyph marks are gone from the card's cells: drawn once, not twice.
#[test]
fn the_card_no_longer_spells_the_marks() {
    let _g = crate::app::theme_test_guard();
    let cells = crate::panecard::pane_card(40, 8, &bar(&[1], &[2], &[3]));
    assert!(!cells
        .iter()
        .any(|c| c.col == 0 && (c.c == '\u{258c}' || c.c == '\u{2576}')));
}

/// A 400-line file scrolled `scroll` lines back from its end.
fn doc<'a>(ticks: &'a [usize], hits: &'a [usize], scroll: usize) -> Bar<'a> {
    Bar {
        total: 400,
        scroll,
        doc: crate::panehome::Doc::File,
        ticks,
        hits,
        ..Default::default()
    }
}

/// The card rows the notches of a 40×20 card land on, in drawing order.
fn notch_rows(b: &Bar) -> Vec<u16> {
    notches(40, 20, b, CW, CH)
        .iter()
        .map(|p| p.y.floor() as u16)
        .collect()
}

/// Landmarks sit down the gutter proportionally, one per row (three in a row
/// are one mark), between the corners — and INSIDE the card: from the
/// stroke at the border cell's middle inward, never past its far side into
/// the gap beside the card (glass survey E14).
#[test]
fn landmarks_notch_the_stroke_from_inside_one_per_row() {
    let _g = crate::app::theme_test_guard();
    let b = doc(&[0, 100, 101, 102, 399], &[], 382);
    let drawn = notches(40, 20, &b, CW, CH);
    let rows = notch_rows(&b);
    let mut uniq = rows.clone();
    uniq.dedup();
    assert_eq!(uniq, rows, "two notches in one row");
    assert_eq!(rows.len(), 2, "the top one is under the thumb: {rows:?}");
    assert!(rows.iter().all(|r| (1..19).contains(r)), "{rows:?}");
    assert!(*rows.last().unwrap() >= 17, "{rows:?}");
    for p in &drawn {
        assert_eq!(p.x, 39.0, "a notch left the border cell");
        // An 8 px cell's 1 px stroke at px 3: the notch is px 0..4.
        assert_eq!(p.w * CW, 4.0);
        assert_eq!(p.color, crew_theme::theme().legend_off);
    }
}

/// The thumb wins the row it shares with a landmark: where you ARE is the
/// answer, and a notch beside it would read as a second position.
#[test]
fn the_thumb_hides_the_landmark_it_sits_on() {
    let _g = crate::app::theme_test_guard();
    let (top, mid) = (doc(&[0, 200], &[], 382), doc(&[0, 200], &[], 0));
    assert!(!notch_rows(&top).contains(&1), "{:?}", notch_rows(&top));
    assert!(notch_rows(&mid).contains(&1), "{:?}", notch_rows(&mid));
}

/// While you are searching, the gutter shows where the matches are, in their
/// own colour and heavier; a hit and a landmark in one row are one mark,
/// the hit's.
#[test]
fn search_hits_outrank_landmarks_in_their_own_colour() {
    let _g = crate::app::theme_test_guard();
    let b = doc(&[10], &[10, 200, 300], 0);
    let drawn = notches(40, 20, &b, CW, CH);
    assert_eq!(drawn.len(), 3, "{drawn:?}");
    assert!(drawn.iter().all(|p| p.color == crate::findhl::hit_mark()));
    assert!(
        drawn.iter().all(|p| p.h * CH == 2.0),
        "a hit is the heavy mark"
    );
    let rows = notch_rows(&b);
    assert!(rows[0] < rows[1] && rows[1] < rows[2], "{rows:?}");
}

/// Nothing to mark, or no room for a gutter: nothing drawn.
#[test]
fn no_landmarks_and_no_room_both_draw_nothing() {
    let _g = crate::app::theme_test_guard();
    assert!(notches(40, 20, &doc(&[], &[], 0), CW, CH).is_empty());
    assert!(notches(40, 4, &doc(&[1, 2], &[3], 0), CW, CH).is_empty());
}
