use super::{plan, Pair, Step};
use crate::reminders::fields::Fields;

fn f(title: &str, done: bool, due: Option<&str>) -> Fields {
    Fields {
        title: title.into(),
        done,
        due: due.map(Into::into),
    }
}

fn open(title: &str) -> Fields {
    f(title, false, None)
}

fn pair(todo: u64, rem: &str, last: Fields) -> Pair {
    Pair {
        todo,
        rem: rem.into(),
        last,
    }
}

#[test]
fn a_change_on_one_side_goes_to_the_other() {
    let last = open("milk");
    let pairs = [pair(1, "r1", last.clone()), pair(2, "r2", open("bread"))];
    let todos = [(1, open("oat milk")), (2, open("bread"))];
    let rems = [
        ("r1".to_string(), last),
        ("r2".into(), f("bread", true, None)),
    ];
    let p = plan(&todos, &rems, &pairs);
    assert_eq!(
        p.steps,
        [
            Step::SetRem {
                rem: "r1".into(),
                was: open("milk"),
                f: open("oat milk"),
            },
            Step::SetTodo {
                todo: 2,
                was: open("bread"),
                f: f("bread", true, None),
            },
        ]
    );
    assert_eq!(p.pairs[0].last, open("oat milk"), "agreed on the change");
}

#[test]
fn both_changed_crew_wins_the_field_and_each_keeps_the_other_field() {
    let pairs = [pair(1, "r1", open("milk"))];
    let todos = [(1, open("oat milk"))];
    let rems = [("r1".to_string(), f("whole milk", true, Some("2026-10-24")))];
    let p = plan(&todos, &rems, &pairs);
    let both = f("oat milk", true, Some("2026-10-24"));
    assert_eq!(p.pairs[0].last, both);
    assert_eq!(p.steps.len(), 2, "{:?}", p.steps);
}

#[test]
fn a_delete_goes_across_unless_the_other_side_changed_since() {
    let pairs = [
        pair(1, "r1", open("a")),
        pair(2, "r2", open("b")),
        pair(3, "r3", open("c")),
        pair(4, "r4", open("d")),
        pair(5, "r5", open("e")),
    ];
    // 1 deleted in crew; 2 deleted in Reminders; 3 deleted in crew but
    // edited in Reminders; 4 deleted in Reminders but edited in crew; 5 done
    // in crew and cleared from Reminders.
    let todos = [(2, open("b")), (4, open("d!")), (5, f("e", true, None))];
    let rems = [("r1".to_string(), open("a")), ("r3".into(), open("c!"))];
    let p = plan(&todos, &rems, &pairs);
    assert_eq!(
        p.steps,
        [
            Step::DropRem { rem: "r1".into() },
            Step::DropTodo {
                todo: 2,
                was: open("b")
            },
            Step::NewTodo {
                rem: "r3".into(),
                f: open("c!")
            },
            Step::NewRem {
                todo: 4,
                f: open("d!")
            },
        ]
    );
    assert!(p.pairs.is_empty(), "every old pair ended: {:?}", p.pairs);
}

#[test]
fn the_unpaired_pair_by_title_before_anything_is_made() {
    let todos = [
        (1, f("Milk", false, Some("2026-10-24"))),
        (2, open("call mum")),
        (3, f("old", true, None)),
    ];
    let rems = [
        ("r1".to_string(), f("milk", true, None)),
        ("r2".into(), open("dentist")),
        ("r3".into(), f("long done", true, None)),
    ];
    let p = plan(&todos, &rems, &[]);
    assert_eq!(p.pairs.len(), 1);
    assert_eq!(p.pairs[0].last, f("Milk", true, Some("2026-10-24")));
    let made: Vec<&Step> = (p.steps.iter())
        .filter(|s| matches!(s, Step::NewRem { .. } | Step::NewTodo { .. }))
        .collect();
    assert_eq!(
        made,
        [
            &Step::NewRem {
                todo: 2,
                f: open("call mum")
            },
            &Step::NewTodo {
                rem: "r2".into(),
                f: open("dentist")
            },
        ],
        "done ones are history, not news"
    );
}

#[test]
fn a_reminder_whose_id_changed_is_found_again_by_its_title() {
    let pairs = [pair(1, "old-id", open("milk")), pair(2, "r2", open("tea"))];
    let todos = [(1, open("milk")), (2, open("tea"))];
    let rems = [
        ("new-id".to_string(), open("milk")),
        ("r2".into(), open("tea")),
    ];
    let p = plan(&todos, &rems, &pairs);
    assert!(p.steps.is_empty(), "{:?}", p.steps);
    assert_eq!(p.pairs[0].rem, "new-id");
}

#[test]
fn reminders_answering_empty_never_deletes_the_todo_list() {
    let pairs = [pair(1, "r1", open("a")), pair(2, "r2", open("b"))];
    let todos = [(1, open("a")), (2, open("b"))];
    let p = plan(&todos, &[], &pairs);
    assert!(
        p.steps.iter().all(|s| matches!(s, Step::NewRem { .. })),
        "{:?}",
        p.steps
    );
    assert_eq!(p.steps.len(), 2, "sent again, not deleted");
}
