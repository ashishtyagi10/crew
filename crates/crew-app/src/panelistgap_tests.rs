//! The air between a row's title and its markers — found by shooting every
//! marker a PANES row can wear at the nav's real width (`transientshot_tests`).
use super::{pane_cells, PaneRow};

fn row(title: &str) -> PaneRow {
    PaneRow {
        index: 2,
        title: title.into(),
        focused: false,
        activity: false,
        minimized: false,
        attention: None,
        busy: false,
        unread: 0,
        hovered: false,
    }
}

fn text(panes: &[PaneRow], cols: u16) -> String {
    let cells = pane_cells(panes, cols, 8, '\u{280b}');
    let mut line = vec![' '; cols as usize];
    for c in cells.iter().filter(|c| c.row == 1) {
        line[c.col as usize] = c.c;
    }
    line.into_iter().collect::<String>().trim_end().to_string()
}

/// `cargo wat…12 ●`: the cut title, the unread count and the dot with no
/// air between the first two. Every neighbour on the row gets a column.
#[test]
fn a_cut_title_keeps_a_column_of_air_before_its_count() {
    let mut p = row("cargo watch -x check");
    p.unread = 12;
    p.activity = true;
    let line = text(&[p], 21);
    assert!(line.contains("\u{2026} 12 \u{25cf}"), "{line:?}");
}

#[test]
fn a_cut_title_keeps_a_column_of_air_before_the_spinner_and_the_restore_button() {
    let mut busy = row("claude \u{2014} crew session");
    busy.busy = true;
    let line = text(&[busy], 21);
    assert!(line.ends_with("\u{2026} \u{280b}"), "{line:?}");
    let mut hidden = row("far ~/code/crew/crates");
    hidden.minimized = true;
    let line = text(&[hidden], 21);
    assert!(line.contains("\u{2026} [+]"), "{line:?}");
}

/// A title that fits with its column of air is not touched.
#[test]
fn a_short_title_is_left_whole() {
    let mut p = row("zsh");
    p.attention = Some(('!', true));
    assert_eq!(text(&[p], 21), "    2 zsh          !");
}

/// A row with nothing at its right edge keeps that column for its title —
/// the air is for a neighbour, and `smith` has none.
#[test]
fn a_row_with_no_marker_gives_the_column_back_to_the_title() {
    assert_eq!(text(&[row("smith")], 13), "    2 smith");
    // The same title beside a marker is cut, with air.
    let mut busy = row("smith");
    busy.busy = true;
    assert_eq!(text(&[busy], 13), "    2 smi\u{2026} \u{280b}");
    // A marker in its off phase still holds the column: no jitter.
    let mut blink = row("smith");
    blink.attention = Some(('!', false));
    assert_eq!(text(&[blink], 13), "    2 smi\u{2026}");
}
