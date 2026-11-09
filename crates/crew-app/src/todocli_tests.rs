use super::*;
use crate::todopane::item::TodoIssue;

fn item(id: u64, title: &str) -> TodoItem {
    TodoItem {
        id,
        title: title.into(),
        ..TodoItem::default()
    }
}

#[test]
fn an_id_is_found_by_any_unique_tail() {
    let items = [item(1731000000123, "a"), item(1731000009123, "b")];
    assert_eq!(find(&items, "9123"), Ok(1));
    assert_eq!(find(&items, "1731000000123"), Ok(0));
    assert!(find(&items, "123").unwrap_err().contains("2 todos"));
    assert!(find(&items, "999").unwrap_err().contains("no todo"));
    assert!(find(&items, "abc").is_err());
}

#[test]
fn a_line_carries_tags_and_the_issue() {
    let mut it = item(42, "fix the build");
    it.project = Some("crew".into());
    it.assignee = Some("ana".into());
    it.issue = Some(TodoIssue {
        repo: "me/crew".into(),
        number: 7,
        ..TodoIssue::default()
    });
    assert_eq!(
        line(&it, 0),
        "[ ] 42  fix the build  @crew  #ana  me/crew#7"
    );
    it.done = true;
    assert!(line(&it, 0).starts_with("[x] 42"));
}

#[test]
fn bad_words_print_usage() {
    assert_eq!(run(&["frobnicate".to_string()]), 64);
    assert_eq!(run(&["done".to_string()]), 64);
}
