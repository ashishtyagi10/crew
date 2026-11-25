//! The card list's lines are the line map's lines.
use super::*;
use crate::navpanecard::tests::line;
use crate::navpanecard::Card;
use crate::panestate::EntryState;

fn row(index: usize, name: &str) -> PaneRow {
    PaneRow {
        index,
        title: name.into(),
        focused: false,
        activity: false,
        minimized: false,
        attention: None,
        busy: false,
        unread: 0,
        hovered: false,
        card: Some(Card {
            name: name.into(),
            detail: Some(format!("{name} detail")),
            glyph: '❯',
            state: EntryState::Idle,
        }),
    }
}

/// Rule, then card, gap, card: the order `push_entries` gives the map.
#[test]
fn a_flat_list_is_cards_with_a_gap_between() {
    let _g = crate::app::theme_test_guard();
    let cells = flat(&[row(1, "a"), row(2, "b")], 26, 20);
    assert!(line(&cells, 0, 26).contains("PANES"));
    assert!(line(&cells, 1, 26).contains("1 a"));
    assert!(line(&cells, 2, 26).contains("a detail"));
    assert_eq!(line(&cells, 3, 26), "", "the gap");
    assert!(line(&cells, 4, 26).contains("2 b"));
    assert!(line(&cells, 5, 26).contains("b detail"));
}

#[test]
fn tabs_frame_their_cards_and_count_what_shows() {
    let _g = crate::app::theme_test_guard();
    let blocks = vec![
        TabBlock {
            num: 1,
            title: "one".into(),
            active: true,
            hovered: false,
            rows: vec![row(1, "a"), row(2, "b")],
        },
        TabBlock {
            num: 2,
            title: "two".into(),
            active: false,
            hovered: false,
            rows: vec![row(3, "c")],
        },
    ];
    let cells = tabs(&blocks, 26, 30);
    assert!(
        line(&cells, 0, 26).contains("2 tabs \u{b7} 3"),
        "{}",
        line(&cells, 0, 26)
    );
    assert!(line(&cells, 1, 26).contains("1 \u{b7} one"));
    assert!(line(&cells, 2, 26).contains("1 a"));
    assert!(
        line(&cells, 4, 26).starts_with(" \u{2502}"),
        "a gap keeps the card's sides"
    );
    assert!(line(&cells, 5, 26).contains("2 b"));
    assert!(line(&cells, 7, 26).contains('\u{2570}'), "the bottom edge");
    assert!(line(&cells, 8, 26).contains("2 \u{b7} two"));
    assert!(line(&cells, 9, 26).contains("3 c"));
    // A short panel shows what fits whole and says so.
    let short = tabs(&blocks, 26, 5);
    assert!(
        line(&short, 0, 26).contains("1/3"),
        "{}",
        line(&short, 0, 26)
    );
}
