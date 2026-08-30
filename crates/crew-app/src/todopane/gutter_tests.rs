//! The list's scroll thumb: it only appears when there is something to say,
//! and it reaches both ends of the track.
use super::cells;

/// Rows the thumb (`┃`) covers, and the track rows around it (`│`).
fn thumb(above: u16, total: u16, lh: u16) -> (Vec<u16>, usize) {
    let v = cells(above, total, 0, lh, 9);
    let rows: Vec<u16> = v
        .iter()
        .filter(|c| c.c == '\u{2503}')
        .map(|c| c.row)
        .collect();
    (rows, v.len())
}

#[test]
fn a_list_that_fits_draws_no_gutter() {
    let _g = crate::app::theme_test_guard();
    assert!(cells(0, 6, 0, 6, 9).is_empty());
    assert!(cells(0, 3, 0, 6, 9).is_empty());
    assert!(cells(0, 9, 0, 0, 9).is_empty());
}

#[test]
fn the_thumb_covers_the_whole_track_and_nothing_else() {
    let _g = crate::app::theme_test_guard();
    let (_, n) = thumb(0, 20, 8);
    assert_eq!(n, 8, "one cell per list row, no more");
}

/// The two ends are the two facts a reading has to get right: at the top
/// there is nothing above, at the bottom nothing below.
#[test]
fn the_thumb_reaches_both_ends() {
    let _g = crate::app::theme_test_guard();
    let (top, _) = thumb(0, 20, 8);
    assert_eq!(
        *top.first().unwrap(),
        0,
        "top of the list, top of the track"
    );
    let (bot, _) = thumb(12, 20, 8);
    assert_eq!(
        *bot.last().unwrap(),
        7,
        "a thumb stopping short of the bottom says there is still more, forever"
    );
}

#[test]
fn the_thumb_is_proportional_and_never_vanishes() {
    let _g = crate::app::theme_test_guard();
    let (half, _) = thumb(0, 16, 8);
    assert_eq!(half.len(), 4, "half the list visible, half the track");
    let (tiny, _) = thumb(0, 2000, 8);
    assert_eq!(tiny.len(), 1, "a huge list still has to mark its place");
}
