use super::todos;
use crate::reminders::fields::Fields;
use crate::reminders::plan::Step;
use crate::todopane::item::TodoItem;

const NOW: u64 = 1_792_500_000_000;

fn item(id: u64, title: &str) -> TodoItem {
    TodoItem {
        id,
        title: title.into(),
        created_ms: id,
        project: Some("home".into()),
        ..TodoItem::default()
    }
}

fn f(title: &str, done: bool, due: Option<&str>) -> Fields {
    Fields {
        title: title.into(),
        done,
        due: due.map(Into::into),
    }
}

#[test]
fn reminders_changes_land_on_the_todo_and_crew_keeps_what_reminders_has_not() {
    let mut items = vec![item(1, "milk"), item(2, "bread")];
    let steps = [
        Step::SetTodo {
            todo: 1,
            was: f("milk", false, None),
            f: f("oat milk", true, Some("2026-10-24T08:30")),
        },
        Step::DropTodo {
            todo: 2,
            was: f("bread", false, None),
        },
    ];
    assert!(todos(&mut items, &steps, NOW).is_empty());
    assert_eq!(items.len(), 1);
    let it = &items[0];
    assert_eq!(
        (it.title.as_str(), it.done, it.done_ms),
        ("oat milk", true, Some(NOW))
    );
    assert!(it.due_has_time && it.due_ms.is_some());
    assert_eq!(
        it.project.as_deref(),
        Some("home"),
        "the tag is crew's alone"
    );
}

#[test]
fn a_todo_edited_since_the_plan_is_left_for_the_next_pass() {
    let mut items = vec![item(1, "milk (edited)"), item(2, "bread (edited)")];
    let steps = [
        Step::SetTodo {
            todo: 1,
            was: f("milk", false, None),
            f: f("oat milk", false, None),
        },
        Step::DropTodo {
            todo: 2,
            was: f("bread", false, None),
        },
    ];
    todos(&mut items, &steps, NOW);
    assert_eq!(
        items,
        vec![item(1, "milk (edited)"), item(2, "bread (edited)")]
    );
}

#[test]
fn a_reminder_new_to_crew_becomes_a_todo_paired_with_it() {
    let mut items = vec![item(NOW + 5, "x")];
    let steps = [Step::NewTodo {
        rem: "r9".into(),
        f: f("dentist", false, Some("2026-10-30")),
    }];
    let born = todos(&mut items, &steps, NOW);
    let it = items.last().unwrap();
    assert_eq!(it.id, NOW + 6, "after every id there is");
    assert_eq!((it.title.as_str(), it.due_has_time), ("dentist", false));
    assert_eq!(born.len(), 1);
    assert_eq!((born[0].todo, born[0].rem.as_str()), (NOW + 6, "r9"));
    assert_eq!(Fields::of(it), born[0].last, "agreed as written");
}
