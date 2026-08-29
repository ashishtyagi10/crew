use super::*;

use crate::attention::{Attention, BLINK_MS};
use crate::notify::NotifyKind;

/// `+3` says how many panes are behind the tile and nothing about which,
/// which is the one thing you would look at it to find out.
#[test]
fn the_overflow_tile_names_the_panes_behind_it() {
    let _g = crate::app::theme_test_guard();
    let names = vec!["7 build".to_string(), "8 crew \u{b7} claude".to_string()];
    let cells = overflow_cells(&names, 20, 4);
    let row = |r: u16| -> String {
        let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == r).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    assert_eq!(row(0), "7 build");
    assert!(row(1).starts_with("8 crew"), "{:?}", row(1));
}

/// A tile with room for two names shows two, and clips a long one rather
/// than drawing past its own card.
#[test]
fn the_overflow_tile_shows_what_fits_and_no_more() {
    let _g = crate::app::theme_test_guard();
    let names: Vec<String> = (0..9)
        .map(|i| format!("{i} a-very-long-pane-name"))
        .collect();
    let cells = overflow_cells(&names, 8, 2);
    assert!(
        cells.iter().all(|c| c.col < 8 && c.row < 2),
        "drew past the card"
    );
    assert_eq!(cells.iter().filter(|c| c.row == 0).count(), 8);
}

/// A thumbnail is the narrowest card crew draws — the strip fits as many
/// as it can — so it is where a marker and a count are likeliest to land
/// on each other.
#[test]
fn a_thumbnail_never_draws_two_glyphs_in_one_cell() {
    let _g = crate::app::theme_test_guard();
    let marker = Some(('\u{25cf}', crew_theme::theme().activity));
    for cols in 0..=40u16 {
        for unread in [0usize, 7, 128] {
            let cells = strip_row(cols, marker, unread);
            assert!(
                cells.iter().all(|c| c.col < cols.max(1)),
                "{cols}: a cell escaped a thumbnail"
            );
            let mut used: Vec<u16> = cells.iter().map(|c| c.col).collect();
            used.sort_unstable();
            let before = used.len();
            used.dedup();
            assert_eq!(
                used.len(),
                before,
                "{cols}/{unread}: two glyphs in one cell"
            );
        }
    }
}

#[test]
fn attention_supersedes_the_activity_dot() {
    let _g = crate::app::theme_test_guard();
    let a = Attention {
        kind: NotifyKind::AgentDone,
        at_ms: 0,
    };
    let t = crew_theme::theme();
    assert_eq!(strip_marker(true, Some(a), false, 0), Some(('✓', t.bell)));
    assert_eq!(strip_marker(true, None, false, 0), Some(('●', t.activity)));
    assert_eq!(strip_marker(false, None, false, 0), None);
}

#[test]
fn marker_blinks_off_mid_pulse() {
    let a = Attention {
        kind: NotifyKind::Bell,
        at_ms: 0,
    };
    assert_eq!(strip_marker(false, Some(a), false, BLINK_MS), None);
    assert!(strip_marker(false, Some(a), false, 2 * BLINK_MS).is_some());
}

/// The strip is where a pane goes when you have not looked at it, so it
/// is where "how much did I miss" matters most. The count is right-
/// aligned; the marker keeps the left.
#[test]
fn a_thumbnail_shows_the_count_of_what_arrived_while_it_was_away() {
    let _g = crate::app::theme_test_guard();
    let marker = Some(('\u{25cf}', crew_theme::theme().activity));
    let row = strip_row(12, marker, 7);
    let at = |col: u16| row.iter().find(|c| c.col == col).map(|c| c.c);
    assert_eq!(at(0), Some('\u{25cf}'), "the marker lost its column");
    assert_eq!(at(11), Some('7'), "the count is not at the right edge");
    let many = strip_row(12, marker, 4000);
    let text: String = {
        let mut v: Vec<&CellView> = many.iter().filter(|c| c.col > 0).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    assert_eq!(text, "99+", "{text:?}");
}

/// Nothing new, nothing drawn — and a card with no room for both keeps
/// the marker, which is the one that says a pane is alive.
#[test]
fn a_quiet_or_tiny_thumbnail_draws_no_count() {
    let _g = crate::app::theme_test_guard();
    let marker = Some(('\u{25cf}', crew_theme::theme().activity));
    assert_eq!(strip_row(12, marker, 0).len(), 1);
    assert_eq!(
        strip_row(2, marker, 7).len(),
        1,
        "the count crowded out the marker"
    );
    assert!(strip_row(0, marker, 7).is_empty());
}

#[test]
fn busy_pane_pulses_a_dot_that_never_blinks_out() {
    // A busy pane always shows the dot (never None), and its colour changes
    // over the pulse — the trough (dim) differs from the peak (full).
    let trough = strip_marker(false, None, true, 0).expect("busy always shows a dot");
    let peak = strip_marker(false, None, true, PULSE_MS / 2).expect("busy always shows a dot");
    assert_eq!(trough.0, '●');
    assert_ne!(trough.1, peak.1, "the dot pulses between dim and bright");
    // Busy beats a plain activity dot; both are '●' but busy pulses.
    assert!(strip_marker(true, None, true, 0).is_some());
}

/// Read the tile back as text, one string per row.
fn tile(names: &[String], cols: u16, rows: u16) -> Vec<String> {
    let cells = overflow_cells(names, cols, rows);
    (0..rows)
        .map(|r| {
            let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == r).collect();
            v.sort_by_key(|c| c.col);
            v.iter().map(|c| c.c).collect()
        })
        .collect()
}

/// The tile's whole job is to answer "which panes are behind this?" — and it
/// was dropping the tail of that answer twice over. A name wider than the
/// tile read `8 crew · claude-opus-5 revi`, as if the pane were called
/// "revi".
#[test]
fn a_name_too_wide_for_the_tile_ellipsizes() {
    let _g = crate::app::theme_test_guard();
    let names = vec!["8 crew \u{b7} claude-opus-5 reviewing the diff".to_string()];
    let row = tile(&names, 20, 4).remove(0);
    assert!(row.ends_with('\u{2026}'), "no ellipsis: {row:?}");
    assert!(row.chars().count() <= 20, "{row:?} overran the tile");
}

/// A sixth pane behind a four-row tile simply was not there, under a legend
/// that said `+6`. The last row says how many it could not name.
#[test]
fn panes_the_tile_has_no_room_for_are_counted_not_dropped() {
    let _g = crate::app::theme_test_guard();
    let names: Vec<String> = (1..=6).map(|i| format!("{i} pane")).collect();
    let rows = tile(&names, 20, 4);
    assert_eq!(rows[3], "+3 more", "{rows:?}");
    // Three names plus the count — everything the tile can say, said.
    assert_eq!(&rows[..3], &["1 pane", "2 pane", "3 pane"]);
}

/// A list that fits spends every row on a name.
#[test]
fn a_list_that_fits_needs_no_count() {
    let _g = crate::app::theme_test_guard();
    let names: Vec<String> = (1..=3).map(|i| format!("{i} pane")).collect();
    let rows = tile(&names, 20, 4);
    assert!(!rows.iter().any(|r| r.contains("more")), "{rows:?}");
    assert_eq!(rows[3], "");
}

/// The leading number is how `Cmd+N` reaches the pane, so it wears the accent
/// the way every other actionable token in crew does.
#[test]
fn the_pane_number_is_the_actionable_half_of_the_row() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let cells = overflow_cells(&["12 htop".to_string()], 20, 2);
    let accent = crate::palette::accent();
    let numbered: String = cells
        .iter()
        .filter(|c| c.fg == accent)
        .map(|c| c.c)
        .collect();
    assert_eq!(numbered, "12", "only the number is accented: {numbered:?}");
}
