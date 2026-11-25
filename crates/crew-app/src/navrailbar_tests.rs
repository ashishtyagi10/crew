//! The collapsed card's body.
use super::*;

fn foot() -> crate::navrailfoot::Foot {
    crate::navrailfoot::Foot {
        time: "09:41".into(),
        sky: None,
        stats: Default::default(),
        git: None,
    }
}

/// Icons, a rule, the pane numbers under it, and the gear on the last row — no two on one row.
#[test]
fn the_list_starts_under_the_icons_and_the_gear_holds_the_last_row() {
    let _g = crate::app::theme_test_guard();
    let rows: Vec<crate::panelist::PaneRow> = (1..=3)
        .map(|index| crate::panelist::PaneRow {
            index,
            title: "p".into(),
            focused: index == 1,
            activity: false,
            minimized: false,
            attention: None,
            busy: false,
            unread: 0,
            hovered: false,
            card: None,
        })
        .collect();
    let list = crate::tabnav::NavList::from(rows);
    let f = foot();
    let b = RailBody {
        list: &list,
        foot: &f,
        hover: None,
        badges: Badges::default(),
        spin: '*',
    };
    let (cells, _) = body(&b, crate::navbar::BAR_COLS, 40, 2.0);
    let at = |row: u16| -> String {
        let mut v: Vec<_> = cells.iter().filter(|c| c.row == row).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    assert!(
        at(HEAD_ROWS).chars().all(|c| c == '\u{2500}'),
        "{:?}",
        at(HEAD_ROWS)
    );
    assert!(at(LIST_TOP).contains('1'), "{:?}", at(LIST_TOP));
    assert!(at(LIST_TOP + 2).contains('3'));
    // The gear is drawn on the last row, not written: paint around its centre.
    let (_, paint) = body(&b, crate::navbar::BAR_COLS, 40, 2.0);
    assert!(
        paint.iter().any(|p| (p.y - 39.5).abs() < 0.7),
        "the gear on row 39"
    );
}
