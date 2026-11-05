use super::*;
use crate::welcome::{rain_box, welcome_cells_animated};

fn full() -> String {
    let (lead, head) = headline().expect("a changelog headline");
    format!("{lead}{head}")
}

fn row_text(cells: &[crew_render::CellView], row: u16) -> String {
    let mut v: Vec<_> = cells.iter().filter(|c| c.row == row).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect::<String>().trim().to_string()
}

/// The queue's case: a tall narrow tile with rows to spare. The headline is
/// folded onto them and read whole, not cut at its first clause — and every
/// row of it is on screen, under the hint, inside the card's margins.
#[test]
fn a_tall_narrow_tile_reads_the_headline_whole() {
    let _g = crate::app::theme_test_guard();
    let (cols, rows) = (40, 50);
    let (_, _, _, h) = rain_box(cols, rows, false).expect("rain fits 40×50");
    let lines = news_lines(cols, rows, h, false);
    // Headlines change every release; one too short to need folding is
    // still read whole, on one row.
    let folds = full().chars().count() > 40 - 8;
    assert_eq!(lines.len() > 1, folds, "{lines:?}");
    assert_eq!(lines.join(" "), full());
    assert!(
        lines.iter().all(|l| l.chars().count() <= 40 - 8),
        "{lines:?}"
    );
    let cells = welcome_cells_animated(cols, rows, 0, None);
    let (top, _, _, h) = rain_box(cols, rows, false).unwrap();
    for (i, line) in lines.iter().enumerate() {
        assert_eq!(&row_text(&cells, top + h + 3 + i as u16), line);
    }
}

/// A restore offer still lands under the news, a blank row clear of it.
#[test]
fn the_restore_offer_sits_under_the_folded_headline() {
    let _g = crate::app::theme_test_guard();
    let (cols, rows) = (60, 50);
    let (top, _, _, h) = rain_box(cols, rows, true).unwrap();
    let n = news_lines(cols, rows, h, true).len() as u16;
    let cells = welcome_cells_animated(cols, rows, 0, Some(2));
    assert_eq!(row_text(&cells, top + h + 3 + n), "");
    assert!(row_text(&cells, top + h + 4 + n).ends_with("/restore"));
}

/// No spare rows, no fold: the one clipped line it always was.
#[test]
fn without_spare_rows_the_headline_stays_one_line() {
    let (_, _, _, h) = rain_box(120, 30, false).unwrap();
    let lines = news_lines(40, h + 5, h, false);
    assert_eq!(lines, vec![whats_new(40).unwrap()]);
    assert!(news_lines(10, 50, h, false).is_empty());
}

/// More words than rows: the last row is cut between words, with `…`.
#[test]
fn what_the_rows_cannot_hold_is_cut_on_the_last_one() {
    let lines = wrap("one two three four five six seven", 9, 2);
    assert_eq!(
        lines,
        vec!["one two".to_string(), "three\u{2026}".to_string()]
    );
    assert_eq!(wrap("a b c", 9, 3), vec!["a b c".to_string()]);
}

/// Folded rows are near one length: no lone word under a full row.
#[test]
fn folded_rows_are_balanced() {
    let text = "new in 0.29.93 · mouse drags and moves reach programs that asked for them";
    let lines = balanced(text, 52, wrap(text, 52, 4));
    assert_eq!(lines.len(), 2);
    assert_eq!(lines.join(" "), text);
    let (a, b) = (lines[0].chars().count(), lines[1].chars().count());
    assert!(a.abs_diff(b) <= 12, "{lines:?}");
    let cut = wrap("one two three four five six seven", 9, 2);
    assert_eq!(
        balanced("one two three four five six seven", 9, cut.clone()),
        cut
    );
}
