//! The bar's rows: what is drawn where, and what a row means.
use super::*;

fn all() -> Vec<BarHit> {
    let mut v: Vec<BarHit> = NavSection::ALL.into_iter().map(BarHit::Section).collect();
    v.push(BarHit::Settings);
    v
}

/// One mapping, both ways, at every height: a row means what is drawn on it.
#[test]
fn every_row_means_what_is_drawn_on_it() {
    for rows in 0..40u16 {
        for h in all() {
            if let Some(r) = row_of(h, rows) {
                assert!(r < rows);
                assert_eq!(at_row(r, rows), Some(h), "rows={rows} {h:?}");
            }
        }
        // A row answers for an icon only inside that icon's slot: its row or the air under it.
        for r in 0..rows {
            if let Some(h) = at_row(r, rows) {
                let drawn = row_of(h, rows).unwrap();
                assert!(
                    r == drawn || r == drawn + 1,
                    "rows={rows} row {r} -> {h:?} on {drawn}"
                );
            }
        }
    }
}

/// The gear never lands on an icon's row: on a bar too short for both, it is the one left out.
#[test]
fn a_short_bar_drops_the_gear_before_an_icon() {
    let rows = HEAD_ROWS - 1;
    assert_eq!(row_of(BarHit::Settings, rows), None);
    assert!(NavSection::ALL
        .into_iter()
        .all(|s| row_of(BarHit::Section(s), rows).is_some()));
}

#[test]
fn the_open_section_wears_the_stripe_and_no_other_does() {
    let _g = crate::app::theme_test_guard();
    let cells = bar_cells(Some(NavSection::System), Badges::default(), 30);
    let stripes: Vec<u16> = cells
        .iter()
        .filter(|c| c.c == '\u{258e}')
        .map(|c| c.row)
        .collect();
    assert_eq!(
        stripes,
        vec![row_of(BarHit::Section(NavSection::System), 30).unwrap()]
    );
    let none = bar_cells(None, Badges::default(), 30);
    assert!(
        !none.iter().any(|c| c.c == '\u{258e}'),
        "collapsed: nothing is open"
    );
}

/// The Panes icon carries the loudest mark a pane has, in the rail's own order.
#[test]
fn the_panes_badge_is_the_loudest_pane_mark() {
    let _g = crate::app::theme_test_guard();
    let row =
        |attention: Option<(char, bool)>, busy: bool, unread: usize| crate::panelist::PaneRow {
            index: 1,
            title: "p".into(),
            focused: false,
            activity: false,
            minimized: false,
            attention,
            busy,
            unread,
            hovered: false,
        };
    assert_eq!(panes_badge(&[row(None, false, 0)], '*'), None);
    let unread = panes_badge(&[row(None, false, 3)], '*').unwrap().0;
    assert_eq!(unread, '\u{25cf}');
    let busy = panes_badge(&[row(None, false, 3), row(None, true, 0)], '*')
        .unwrap()
        .0;
    assert_eq!(busy, '*');
    let all = [row(None, true, 0), row(Some(('!', true)), false, 0)];
    assert_eq!(panes_badge(&all, '*').unwrap().0, '!');
}

#[test]
fn the_activity_badge_counts_what_waits() {
    let _g = crate::app::theme_test_guard();
    let b = |n| {
        let cells = bar_cells(
            None,
            Badges {
                panes: None,
                waiting: n,
            },
            30,
        );
        let row = row_of(BarHit::Section(NavSection::Activity), 30).unwrap();
        cells
            .iter()
            .find(|c| c.row == row && c.col == BADGE_COL)
            .map(|c| c.c)
    };
    assert_eq!(b(0), None);
    assert_eq!(b(3), Some('3'));
    assert_eq!(b(12), Some('+'));
}
