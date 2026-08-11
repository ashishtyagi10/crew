use super::*;
use winit::keyboard::{Key, NamedKey};

use crate::todopane::{store, TodoPane};

#[test]
fn classification_covers_the_pane_keys() {
    assert_eq!(
        todo_key(&Key::Named(NamedKey::Escape), true),
        TodoInput::Close
    );
    assert_eq!(
        todo_key(&Key::Named(NamedKey::Enter), true),
        TodoInput::Enter
    );
    assert_eq!(
        todo_key(&Key::Named(NamedKey::Backspace), true),
        TodoInput::Backspace
    );
    assert_eq!(
        todo_key(&Key::Named(NamedKey::Delete), true),
        TodoInput::DeleteKey
    );
    assert_eq!(todo_key(&Key::Named(NamedKey::Tab), true), TodoInput::Tab);
    assert_eq!(
        todo_key(&Key::Named(NamedKey::ArrowUp), true),
        TodoInput::Up
    );
    assert_eq!(
        todo_key(&Key::Named(NamedKey::ArrowDown), true),
        TodoInput::Down
    );
    assert_eq!(
        todo_key(&Key::Named(NamedKey::Space), true),
        TodoInput::Char(' ')
    );
    assert_eq!(
        todo_key(&Key::Character("d".into()), true),
        TodoInput::Char('d')
    );
    // Releases never act.
    assert_eq!(
        todo_key(&Key::Named(NamedKey::Escape), false),
        TodoInput::Ignore
    );
}

const COLS: u16 = 40;
const ROWS: u16 = 20;

fn pane_with(titles: &[&str]) -> TodoPane {
    let mut p = TodoPane::new();
    for t in titles {
        p.paste(t);
        p.submit();
    }
    p
}

#[test]
fn escape_walks_back_one_layer_at_a_time() {
    let _g = store::test_guard(vec![]);
    let mut p = pane_with(&["one @crew"]);
    // Layer 3: an open tag popup.
    for c in "two @c".chars() {
        p.type_char(c);
    }
    assert!(p.tagmenu.is_some());
    assert!(apply(&mut p, TodoInput::Close, COLS, ROWS).is_none());
    assert!(p.tagmenu.is_none());
    assert_eq!(p.input, "two @c", "closing the popup keeps the draft");
    // Layer 2: draft text.
    assert!(apply(&mut p, TodoInput::Close, COLS, ROWS).is_none());
    assert_eq!(p.input, "");
    // Layer 1: an empty composer → close the pane.
    assert!(matches!(
        apply(&mut p, TodoInput::Close, COLS, ROWS),
        Some(TodoAction::Close)
    ));
}

#[test]
fn arrows_move_between_composer_and_list() {
    let _g = store::test_guard(vec![]);
    let mut p = pane_with(&["one", "two", "three"]);
    assert_eq!(p.sel, None);
    apply(&mut p, TodoInput::Down, COLS, ROWS);
    assert_eq!(p.sel, Some(0), "Down from the composer enters at the top");
    apply(&mut p, TodoInput::Down, COLS, ROWS);
    assert_eq!(p.sel, Some(1));
    apply(&mut p, TodoInput::Up, COLS, ROWS);
    apply(&mut p, TodoInput::Up, COLS, ROWS);
    assert_eq!(p.sel, None, "Up past the top returns to the composer");
    apply(&mut p, TodoInput::Up, COLS, ROWS);
    assert_eq!(p.sel, Some(2), "Up from the composer enters at the bottom");
    apply(&mut p, TodoInput::Down, COLS, ROWS);
    assert_eq!(p.sel, Some(2), "Down at the bottom stays put");
}

#[test]
fn space_toggles_and_d_deletes_the_selected_row() {
    let _g = store::test_guard(vec![]);
    let mut p = pane_with(&["one", "two"]);
    apply(&mut p, TodoInput::Down, COLS, ROWS); // select "one"
    apply(&mut p, TodoInput::Char(' '), COLS, ROWS);
    let done: Vec<(String, bool)> = store::snapshot()
        .iter()
        .map(|it| (it.title.clone(), it.done))
        .collect();
    assert_eq!(
        done,
        vec![("one".to_string(), true), ("two".to_string(), false)]
    );

    // "one" hid; selection index 0 is now "two". `d` deletes it.
    apply(&mut p, TodoInput::Char('d'), COLS, ROWS);
    let titles: Vec<String> = store::snapshot()
        .iter()
        .map(|it| it.title.clone())
        .collect();
    assert_eq!(titles, vec!["one"], "the done item stays in the store");
    assert_eq!(p.sel, None, "nothing visible left to select");
}

#[test]
fn e_reloads_the_selected_item_into_the_composer() {
    let _g = store::test_guard(vec![]);
    let mut p = pane_with(&["fix scroll @crew"]);
    apply(&mut p, TodoInput::Down, COLS, ROWS);
    apply(&mut p, TodoInput::Char('e'), COLS, ROWS);
    assert_eq!(p.sel, None, "editing happens in the composer");
    assert_eq!(p.input, "fix scroll @crew");
    assert!(p.editing.is_some());
}

#[test]
fn other_letters_jump_back_to_the_composer_and_type() {
    let _g = store::test_guard(vec![]);
    let mut p = pane_with(&["one"]);
    apply(&mut p, TodoInput::Down, COLS, ROWS);
    apply(&mut p, TodoInput::Char('x'), COLS, ROWS);
    assert_eq!(p.sel, None);
    assert_eq!(p.input, "x");
}

#[test]
fn popup_navigation_and_tab_accept() {
    let _g = store::test_guard(vec![]);
    let mut p = pane_with(&["a @crew", "b @home"]);
    for c in "new @".chars() {
        p.type_char(c);
    }
    let m = p.tagmenu.as_ref().expect("popup open");
    assert_eq!(m.matches.len(), 2);
    apply(&mut p, TodoInput::Down, COLS, ROWS);
    assert_eq!(p.tagmenu.as_ref().unwrap().sel, 1);
    apply(&mut p, TodoInput::Tab, COLS, ROWS);
    assert!(p.tagmenu.is_none());
    // Both tags are used once; the tie breaks alphabetically → crew, home.
    assert_eq!(p.input, "new @home ");
}

#[test]
fn enter_submits_from_the_composer() {
    let _g = store::test_guard(vec![]);
    let mut p = TodoPane::new();
    for c in "ship it".chars() {
        p.type_char(c);
    }
    apply(&mut p, TodoInput::Enter, COLS, ROWS);
    let titles: Vec<String> = store::snapshot()
        .iter()
        .map(|it| it.title.clone())
        .collect();
    assert_eq!(titles, vec!["ship it"]);
}

#[test]
fn classification_covers_the_paging_keys() {
    assert_eq!(
        todo_key(&Key::Named(NamedKey::PageUp), true),
        TodoInput::PageUp
    );
    assert_eq!(
        todo_key(&Key::Named(NamedKey::PageDown), true),
        TodoInput::PageDown
    );
    assert_eq!(todo_key(&Key::Named(NamedKey::Home), true), TodoInput::Home);
    assert_eq!(todo_key(&Key::Named(NamedKey::End), true), TodoInput::End);
}

/// Concrete page math: 30 one-row items in a 20-row pane (empty composer =
/// 3 rows, list window = 17 rows) page exactly 17 items at a time.
#[test]
fn page_keys_hop_one_visible_page_of_single_line_items() {
    let _g = store::test_guard(vec![]);
    let titles: Vec<String> = (0..30).map(|i| format!("item {i}")).collect();
    let refs: Vec<&str> = titles.iter().map(|s| s.as_str()).collect();
    let mut p = pane_with(&refs);
    apply(&mut p, TodoInput::Down, COLS, ROWS); // enter the list at 0
    assert_eq!(p.sel, Some(0));
    apply(&mut p, TodoInput::PageDown, COLS, ROWS);
    assert_eq!(p.sel, Some(17), "one page = the 17 rows the window shows");
    apply(&mut p, TodoInput::PageDown, COLS, ROWS);
    assert_eq!(p.sel, Some(29), "second page clamps to the last item");
    apply(&mut p, TodoInput::PageUp, COLS, ROWS);
    assert_eq!(p.sel, Some(12), "a page back is 17 items again");
    apply(&mut p, TodoInput::Home, COLS, ROWS);
    assert_eq!(p.sel, Some(0));
    assert_eq!(p.scroll, 0, "Home scrolls the window back to the top");
    apply(&mut p, TodoInput::End, COLS, ROWS);
    assert_eq!(p.sel, Some(29));
}

/// Variable heights: 60-char titles wrap to 2 rows at 40 cols, so a 7-row
/// window (10-row pane) pages 3 items, not 7.
#[test]
fn a_page_is_a_row_sum_not_an_item_count() {
    let _g = store::test_guard(vec![]);
    let titles: Vec<String> = (0..8)
        .map(|i| format!("{i} pad pad pad pad pad pad pad pad pad pad pad pad"))
        .collect();
    let refs: Vec<&str> = titles.iter().map(|s| s.as_str()).collect();
    let mut p = pane_with(&refs);
    let rows = 10; // composer 3 → 7 list rows
    assert_eq!(
        crate::todopane::render::item_h(&p.items[0], COLS, 0),
        2,
        "premise: titles wrap to two rows"
    );
    apply(&mut p, TodoInput::Down, COLS, rows);
    apply(&mut p, TodoInput::PageDown, COLS, rows);
    assert_eq!(p.sel, Some(3), "3 two-row items fit 7 rows");
    apply(&mut p, TodoInput::PageUp, COLS, rows);
    assert_eq!(p.sel, Some(0));
}

/// The paging keys are list-only: on an empty list (or from the composer)
/// they change nothing, and End respects an active @filter.
#[test]
fn paging_edges_empty_list_composer_zone_and_filter() {
    let _g = store::test_guard(vec![]);
    let mut p = pane_with(&[]);
    apply(&mut p, TodoInput::PageDown, COLS, ROWS);
    apply(&mut p, TodoInput::End, COLS, ROWS);
    assert_eq!(p.sel, None, "composer zone ignores paging keys");

    let mut p = pane_with(&["a @crew", "b @home", "c @crew"]);
    p.filter = Some("crew".into());
    apply(&mut p, TodoInput::Down, COLS, ROWS);
    apply(&mut p, TodoInput::End, COLS, ROWS);
    assert_eq!(p.sel, Some(1), "End lands on the last FILTERED item");
    apply(&mut p, TodoInput::Home, COLS, ROWS);
    assert_eq!(p.sel, Some(0));
}
