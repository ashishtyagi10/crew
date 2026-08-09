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
    assert!(apply(&mut p, TodoInput::Close, ROWS).is_none());
    assert!(p.tagmenu.is_none());
    assert_eq!(p.input, "two @c", "closing the popup keeps the draft");
    // Layer 2: draft text.
    assert!(apply(&mut p, TodoInput::Close, ROWS).is_none());
    assert_eq!(p.input, "");
    // Layer 1: an empty composer → close the pane.
    assert!(matches!(
        apply(&mut p, TodoInput::Close, ROWS),
        Some(TodoAction::Close)
    ));
}

#[test]
fn arrows_move_between_composer_and_list() {
    let _g = store::test_guard(vec![]);
    let mut p = pane_with(&["one", "two", "three"]);
    assert_eq!(p.sel, None);
    apply(&mut p, TodoInput::Down, ROWS);
    assert_eq!(p.sel, Some(0), "Down from the composer enters at the top");
    apply(&mut p, TodoInput::Down, ROWS);
    assert_eq!(p.sel, Some(1));
    apply(&mut p, TodoInput::Up, ROWS);
    apply(&mut p, TodoInput::Up, ROWS);
    assert_eq!(p.sel, None, "Up past the top returns to the composer");
    apply(&mut p, TodoInput::Up, ROWS);
    assert_eq!(p.sel, Some(2), "Up from the composer enters at the bottom");
    apply(&mut p, TodoInput::Down, ROWS);
    assert_eq!(p.sel, Some(2), "Down at the bottom stays put");
}

#[test]
fn space_toggles_and_d_deletes_the_selected_row() {
    let _g = store::test_guard(vec![]);
    let mut p = pane_with(&["one", "two"]);
    apply(&mut p, TodoInput::Down, ROWS); // select "one"
    apply(&mut p, TodoInput::Char(' '), ROWS);
    let done: Vec<(String, bool)> = store::snapshot()
        .iter()
        .map(|it| (it.title.clone(), it.done))
        .collect();
    assert_eq!(
        done,
        vec![("one".to_string(), true), ("two".to_string(), false)]
    );

    // "one" sank; selection index 0 is now "two". `d` deletes it.
    apply(&mut p, TodoInput::Char('d'), ROWS);
    let titles: Vec<String> = store::snapshot()
        .iter()
        .map(|it| it.title.clone())
        .collect();
    assert_eq!(titles, vec!["one"]);
    assert_eq!(p.sel, Some(0), "selection clamps to the remaining row");
}

#[test]
fn e_reloads_the_selected_item_into_the_composer() {
    let _g = store::test_guard(vec![]);
    let mut p = pane_with(&["fix scroll @crew"]);
    apply(&mut p, TodoInput::Down, ROWS);
    apply(&mut p, TodoInput::Char('e'), ROWS);
    assert_eq!(p.sel, None, "editing happens in the composer");
    assert_eq!(p.input, "fix scroll @crew");
    assert!(p.editing.is_some());
}

#[test]
fn other_letters_jump_back_to_the_composer_and_type() {
    let _g = store::test_guard(vec![]);
    let mut p = pane_with(&["one"]);
    apply(&mut p, TodoInput::Down, ROWS);
    apply(&mut p, TodoInput::Char('x'), ROWS);
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
    apply(&mut p, TodoInput::Down, ROWS);
    assert_eq!(p.tagmenu.as_ref().unwrap().sel, 1);
    apply(&mut p, TodoInput::Tab, ROWS);
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
    apply(&mut p, TodoInput::Enter, ROWS);
    let titles: Vec<String> = store::snapshot()
        .iter()
        .map(|it| it.title.clone())
        .collect();
    assert_eq!(titles, vec!["ship it"]);
}
