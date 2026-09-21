use super::{at, strip};
use crate::todopane::keys::{apply, TodoAction, TodoInput};
use crate::todopane::{store, TodoPane};

#[test]
fn only_a_trailing_ampersand_is_the_mark() {
    assert_eq!(strip("fix it @crew &"), ("fix it @crew", true));
    assert_eq!(strip("fix it @crew&  "), ("fix it @crew", true));
    assert_eq!(strip("fix it @crew"), ("fix it @crew", false));
    assert_eq!(strip("a && b then c"), ("a && b then c", false));
    assert_eq!(strip("R&D review"), ("R&D review", false));
    assert_eq!(strip("&"), ("", true), "a bare mark is no item");
}

#[test]
fn the_mark_is_found_by_char_for_the_tint() {
    let chars: Vec<char> = "fix it @crew &".chars().collect();
    assert_eq!(at(&chars), Some(13));
    let chars: Vec<char> = "fix it @crew & ".chars().collect();
    assert_eq!(at(&chars), Some(13), "trailing blanks do not hide it");
    let chars: Vec<char> = "a && b".chars().collect();
    assert_eq!(at(&chars), None);
    assert_eq!(at(&[]), None);
}

#[test]
fn enter_on_a_marked_draft_adds_the_item_and_asks_to_run_it() {
    let _g = store::test_guard(vec![]);
    let mut p = TodoPane::new();
    p.paste("fix the flaky test @crew &");
    let action = apply(&mut p, TodoInput::Enter, 60, 20);
    let items = store::snapshot();
    assert_eq!(items.len(), 1, "added first");
    assert_eq!(
        items[0].title, "fix the flaky test",
        "the mark left the title"
    );
    assert_eq!(items[0].project.as_deref(), Some("crew"));
    assert!(
        matches!(action, Some(TodoAction::Run(id)) if id == items[0].id),
        "and handed to the app to run"
    );
    assert!(p.input.is_empty(), "the composer is clear for the next one");
}

#[test]
fn enter_on_a_plain_draft_only_adds() {
    let _g = store::test_guard(vec![]);
    let mut p = TodoPane::new();
    p.paste("buy milk @home");
    assert!(apply(&mut p, TodoInput::Enter, 60, 20).is_none());
    assert_eq!(store::snapshot().len(), 1);
}

#[test]
fn a_mark_on_an_edit_runs_the_edited_item() {
    let _g = store::test_guard(vec![]);
    let mut p = TodoPane::new();
    p.paste("fix it @crew");
    p.submit();
    let id = store::snapshot()[0].id;
    p.edit_at(0);
    p.paste(" &");
    assert!(matches!(apply(&mut p, TodoInput::Enter, 60, 20), Some(TodoAction::Run(i)) if i == id));
    assert_eq!(store::snapshot()[0].title, "fix it");
}

#[test]
fn the_legend_says_a_marked_draft_will_run() {
    let _g = store::test_guard(vec![]);
    let mut p = TodoPane::new();
    p.paste("fix it @crew tomorrow &");
    let (legend, _) = crate::todopane::legend::text(&p, None);
    assert_eq!(legend, "\u{25b6} runs on enter");
}
