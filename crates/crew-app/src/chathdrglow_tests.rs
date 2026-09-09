//! The header's liveness motion: the `thinking` shimmer and the idle dot's
//! breath — both pure functions of the clock `header_cells_at` is handed.
use super::*;
use crate::motion::{set_level, MotionLevel};

/// Row 0 sorted by column.
fn row(cells: &[CellView]) -> Vec<&CellView> {
    let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == 0).collect();
    v.sort_by_key(|c| c.col);
    v
}

/// The colours of `word`'s cells, in order.
fn word_fgs(cells: &[CellView], word: &str) -> Vec<Color> {
    let row = row(cells);
    let text: String = row.iter().map(|c| c.c).collect();
    let at = text
        .find(word)
        .unwrap_or_else(|| panic!("{word:?} not in {text:?}"));
    let start = text[..at].chars().count();
    row[start..start + word.chars().count()]
        .iter()
        .map(|c| c.fg)
        .collect()
}

fn dot_fg(cells: &[CellView]) -> Color {
    row(cells)
        .iter()
        .find(|c| c.c == '\u{25cf}')
        .expect("connected dot")
        .fg
}

#[test]
fn the_thinking_word_shimmers_at_full_motion() {
    let _g = crate::app::theme_test_guard();
    set_level(MotionLevel::Full);
    let fgs = word_fgs(
        &header_cells_at(60, true, true, None, false, 0, 700),
        "thinking",
    );
    assert_eq!(fgs.len(), 8);
    assert!(
        fgs.iter().any(|c| *c != fgs[0]),
        "the word must not be one flat colour mid-sweep: {fgs:?}"
    );
    // The window sits on the muted word: the rest of it IS text_muted.
    let muted = crew_theme::theme().text_muted;
    assert!(
        fgs.contains(&muted),
        "base cells are the muted ink: {fgs:?}"
    );
}

#[test]
fn the_thinking_word_is_flat_muted_at_off() {
    let _g = crate::app::theme_test_guard();
    set_level(MotionLevel::Off);
    let muted = crew_theme::theme().text_muted;
    for now in [0, 700, 1_400] {
        let fgs = word_fgs(
            &header_cells_at(60, true, true, None, false, 0, now),
            "thinking",
        );
        assert!(fgs.iter().all(|c| *c == muted), "now={now}: {fgs:?}");
    }
}

#[test]
fn the_idle_dot_breathes_and_holds_still_at_off_or_busy() {
    let _g = crate::app::theme_test_guard();
    set_level(MotionLevel::Full);
    let t = crew_theme::theme();
    let idle = |now| dot_fg(&header_cells_at(60, true, false, None, false, 0, now));
    assert_eq!(idle(0), t.activity, "the breath starts lit");
    assert_ne!(
        idle(crate::shimmer::BREATH_MS / 2),
        t.activity,
        "half a breath in, dimmer"
    );
    // Busy: the spinner is the liveness; the dot holds the activity colour.
    let busy = dot_fg(&header_cells_at(
        60,
        true,
        true,
        None,
        false,
        0,
        crate::shimmer::BREATH_MS / 2,
    ));
    assert_eq!(busy, t.activity);
    set_level(MotionLevel::Off);
    assert_eq!(
        idle(crate::shimmer::BREATH_MS / 2),
        t.activity,
        "Off is static"
    );
}
