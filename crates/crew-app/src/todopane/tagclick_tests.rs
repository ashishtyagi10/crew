//! The todo pane's `@project` pop-up answers the mouse and says when it
//! scrolls. Its band fell into the "past the list" branch of the hit-test
//! and returned nothing, and its `k/N` mark was gated on the palette's ten
//! rows while this card is capped at eight — so seven tags scrolled under
//! a card that never said so.
use crate::todopane::item::TodoItem;
use crate::todopane::measure::tag_at;
use crate::todopane::render::{click_at, TodoClick};
use crate::todopane::tagmenu::{self, TagMenu};
use crate::todopane::test_pane;

fn menu(n: usize, sel: usize) -> TagMenu {
    TagMenu {
        matches: (1..=n).map(|i| format!("p{i}")).collect(),
        sel,
    }
}

#[test]
fn the_interior_maps_to_a_tag_through_the_scroll_and_the_border_is_inert() {
    let m = menu(9, 0);
    let card = (5, 8, 20);
    assert_eq!(tag_at(&m, (6, 3), card), Some(0));
    assert_eq!(tag_at(&m, (11, 3), card), Some(5));
    assert_eq!(tag_at(&m, (5, 3), card), None, "top border");
    assert_eq!(tag_at(&m, (12, 3), card), None, "bottom border");
    assert_eq!(tag_at(&m, (6, 0), card), None, "left border");
    assert_eq!(tag_at(&m, (6, 19), card), None, "right border");
    // Selecting the last of nine scrolls three off the top.
    assert_eq!(tag_at(&menu(9, 8), (6, 3), card), Some(3));
}

#[test]
fn a_card_shorter_than_its_list_wears_the_scroll_mark() {
    let _g = crate::app::theme_test_guard();
    let items = crate::todopane::measure::tag_items(&menu(9, 0));
    let text = |rows: u16| -> String {
        let cells = crate::cmdmenu::menu_card("projects", &items, 0, 30, rows);
        // The last cell written to a column wins, as on screen.
        let mut by_col = std::collections::BTreeMap::new();
        for c in cells.iter().filter(|c| c.row == 0) {
            by_col.insert(c.col, c.c);
        }
        by_col.values().collect()
    };
    assert!(text(8).contains("1/9"), "{:?}", text(8));
    assert!(!text(12).contains('/'), "{:?}", text(12));
}

#[test]
fn clicking_a_tag_row_accepts_that_tag() {
    let _g = crate::app::theme_test_guard();
    let items = (1..=9)
        .map(|i| TodoItem {
            id: i,
            title: format!("t{i}"),
            done: false,
            done_ms: None,
            project: Some(format!("p{i}")),
            due_ms: None,
            due_has_time: false,
            created_ms: i,
            notified: false,
        })
        .collect();
    let mut p = test_pane(items);
    p.input = "@".into();
    p.cursor = 1;
    let tags = tagmenu::known_tags(&p.items);
    tagmenu::after_edit(&mut p.tagmenu, &p.input, || tags);
    assert_eq!(p.tagmenu.as_ref().map(|m| m.matches.len()), Some(9));
    let (cols, rows) = (40u16, 20u16);
    let ph = crate::todopane::measure::popup_h(&p, rows);
    assert_eq!(ph, 8, "capped at POPUP_MAX");
    let top = rows - crate::todopane::composer::height(&p, cols, rows) - ph;
    assert_eq!(
        click_at(&p, top + 2, 2, cols, rows),
        Some(TodoClick::PickTag(1))
    );
    assert_eq!(
        click_at(&p, top, 2, cols, rows),
        None,
        "the border is inert"
    );
    p.pick_tag(1);
    assert_eq!(p.input, "@p2 ");
    assert!(p.tagmenu.is_none());
}
