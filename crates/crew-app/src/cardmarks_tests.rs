use super::marks;
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
