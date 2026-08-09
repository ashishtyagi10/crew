use super::*;

fn item(id: u64, title: &str) -> TodoItem {
    TodoItem {
        id,
        title: title.to_string(),
        done: false,
        project: Some("crew".into()),
        due_ms: Some(1_700_000),
        due_has_time: true,
        created_ms: id,
        notified: false,
    }
}

#[test]
fn save_and_load_round_trip_every_field() {
    let dir = std::env::temp_dir().join(format!("crew-todo-store-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join("todos.toml");
    let items = vec![
        item(1, "pay rent"),
        TodoItem {
            id: 2,
            title: "unicode títle ✓".into(),
            done: true,
            project: None,
            due_ms: None,
            due_has_time: false,
            created_ms: 2,
            notified: true,
        },
    ];
    save_at(Some(&p), &items);
    assert_eq!(load_at(Some(&p)), items);
    let _ = std::fs::remove_file(&p);
}

#[test]
fn unknown_fields_and_kinds_do_not_break_a_load() {
    let dir = std::env::temp_dir().join(format!("crew-todo-fwd-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join("todos.toml");
    // A file from a NEWER build: extra per-item field. Serde must shrug.
    std::fs::write(
        &p,
        "[[items]]\nid = 7\ntitle = \"from the future\"\nfancy_new_field = \"x\"\n",
    )
    .unwrap();
    let items = load_at(Some(&p));
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, 7);
    assert_eq!(items[0].title, "from the future");
    assert!(!items[0].done, "missing fields take their defaults");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn a_missing_file_is_an_empty_list() {
    let p = std::env::temp_dir().join("crew-todo-definitely-not-here.toml");
    assert_eq!(load_at(Some(&p)), Vec::new());
    assert_eq!(load_at(None), Vec::new());
}

#[test]
fn take_due_fires_once_per_item_and_persists_the_flag() {
    let mut due = item(1, "call mom");
    due.due_ms = Some(500);
    let mut later = item(2, "future thing");
    later.due_ms = Some(9_999_999);
    let mut done = item(3, "already done");
    done.due_ms = Some(400);
    done.done = true;
    let _g = test_guard(vec![due, later, done]);

    // First pass: exactly the one live overdue item, by title.
    assert_eq!(take_due(1_000), vec!["call mom".to_string()]);
    // The flag stuck — a second pass (a restart, in production) is silent.
    assert_eq!(take_due(1_000), Vec::<String>::new());
    assert!(snapshot().iter().any(|it| it.id == 1 && it.notified));
    // The future item still fires when its time comes.
    assert_eq!(take_due(10_000_000), vec!["future thing".to_string()]);
}

#[test]
fn mutate_bumps_the_revision_so_panes_resync() {
    let _g = test_guard(vec![]);
    let before = revision();
    mutate(|items| items.push(item(9, "new")));
    assert!(revision() > before);
    assert_eq!(snapshot().len(), 1);
}
