use super::*;

use crate::attention::BLINK_MS;

fn text(cells: &[CellView], row: u16) -> String {
    let mut v: Vec<_> = cells.iter().filter(|c| c.row == row).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect()
}

/// Both halves of the card: the thing to press, and what pressing it buys.
#[test]
fn the_card_says_what_to_press_and_what_it_lands_on() {
    let _g = crate::app::theme_test_guard();
    let _p = crate::palette::test_guard();
    let cells = restart_cells("9.9.9", 0, 24, 2);
    let first = text(&cells, 0);
    assert!(
        first.contains('\u{27f3}'),
        "the restart glyph leads: {first}"
    );
    assert!(first.contains("restart"), "{first}");
    let second = text(&cells, 1);
    assert!(
        second.contains("v9.9.9"),
        "the version it restarts into: {second}"
    );
}

/// A one-row card keeps the button, not the detail — the press is the point.
#[test]
fn a_single_row_card_keeps_the_button() {
    let _g = crate::app::theme_test_guard();
    let _p = crate::palette::test_guard();
    let cells = restart_cells("9.9.9", 0, 24, 1);
    assert!(text(&cells, 0).contains("restart"));
    assert!(cells.iter().all(|c| c.row == 0), "nothing below row 0");
}

#[test]
fn a_card_with_no_room_draws_nothing() {
    let _g = crate::app::theme_test_guard();
    let _p = crate::palette::test_guard();
    assert!(restart_cells("9.9.9", 0, 3, 2).is_empty());
    assert!(restart_cells("9.9.9", 0, 24, 0).is_empty());
}

/// The blink is what makes it a nag rather than a label, so it has to actually
/// alternate on the shared half-period — and the button's glyph and word must
/// change together, or the card flickers in two colours at once.
#[test]
fn the_button_alternates_on_the_blink_clock() {
    // `motion_test_guard` IS the appearance guard — it pins the theme too.
    // Taking `theme_test_guard` as well deadlocks: one lock, not reentrant.
    let _g = crate::app::motion_test_guard();
    let _p = crate::palette::test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let accent = crate::palette::accent();
    assert_eq!(button_fg(0), accent, "phase 0 is the accent");
    assert_ne!(
        button_fg(BLINK_MS),
        accent,
        "the next half-period must look different"
    );
    assert_eq!(button_fg(2 * BLINK_MS), accent, "and back again");
    let on = restart_cells("9.9.9", 0, 24, 2);
    let off = restart_cells("9.9.9", BLINK_MS, 24, 2);
    let fg_of =
        |c: &[CellView], col: u16| c.iter().find(|x| x.row == 0 && x.col == col).unwrap().fg;
    assert_ne!(fg_of(&on, 0), fg_of(&off, 0), "the glyph blinks");
    assert_ne!(fg_of(&on, 2), fg_of(&off, 2), "and the word blinks with it");
    assert_eq!(fg_of(&on, 0), fg_of(&on, 2), "one colour per phase");
}

/// Motion off is a real setting, not a suggestion: the button holds the accent
/// so it is still visibly the thing to press, and drives no frames.
#[test]
fn motion_off_holds_the_button_steady_and_visible() {
    // `motion_test_guard` IS the appearance guard — it pins the theme too.
    // Taking `theme_test_guard` as well deadlocks: one lock, not reentrant.
    let _g = crate::app::motion_test_guard();
    let _p = crate::palette::test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Off);
    let accent = crate::palette::accent();
    for t in [0, BLINK_MS, 5 * BLINK_MS] {
        assert_eq!(button_fg(t), accent, "at {t} ms");
    }
}
