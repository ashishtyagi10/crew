use std::collections::HashMap;

use super::*;

fn todo(id: u64, title: &str, project: Option<&str>) -> TodoItem {
    TodoItem {
        id,
        title: title.into(),
        project: project.map(Into::into),
        ..TodoItem::default()
    }
}

fn filed(mut it: TodoItem, title: &str, open: bool) -> TodoItem {
    it.issue = Some(TodoIssue {
        repo: "me/crew".into(),
        number: 9,
        url: String::new(),
        title: title.into(),
        open,
    });
    it
}

fn crew_repo(p: &str) -> Option<String> {
    (p == "crew").then(|| "me/crew".to_string())
}

fn run(items: &[TodoItem], remote: &[(u64, &str, bool)]) -> Vec<Step> {
    let remote: HashMap<u64, Remote> = remote
        .iter()
        .map(|&(id, t, open)| {
            (
                id,
                Remote {
                    title: t.into(),
                    open,
                },
            )
        })
        .collect();
    plan(items, &mut crew_repo, &remote)
}

/// (title, open, push_title, push_open) of the one Sync step, if any.
fn sync_of(steps: &[Step]) -> Option<(String, bool, bool, bool)> {
    match steps {
        [Step::Sync {
            title,
            open,
            push_title,
            push_open,
            ..
        }] => Some((title.clone(), *open, *push_title, *push_open)),
        [] => None,
        s => panic!("unexpected {s:?}"),
    }
}

#[test]
fn an_open_project_todo_in_a_github_repo_is_filed() {
    let mut done = todo(3, "old", Some("crew"));
    done.done = true;
    let items = [
        todo(1, "fix it", Some("crew")),
        todo(2, "call", Some("jessi")),
        done,
        todo(4, "x", None),
    ];
    assert_eq!(
        run(&items, &[]),
        vec![Step::File {
            todo: 1,
            repo: "me/crew".into(),
            title: "fix it".into()
        }]
    );
}

#[test]
fn nothing_changed_is_nothing_to_do() {
    let it = filed(todo(1, "a", Some("crew")), "a", true);
    assert_eq!(sync_of(&run(&[it], &[(1, "a", true)])), None);
}

#[test]
fn a_tick_in_crew_closes_the_issue() {
    let mut it = filed(todo(1, "a", Some("crew")), "a", true);
    it.done = true;
    assert_eq!(
        sync_of(&run(&[it], &[(1, "a", true)])),
        Some(("a".into(), false, false, true))
    );
}

#[test]
fn closing_on_github_ticks_the_todo() {
    let it = filed(todo(1, "a", Some("crew")), "a", true);
    assert_eq!(
        sync_of(&run(&[it], &[(1, "a", false)])),
        Some(("a".into(), false, false, false))
    );
}

#[test]
fn a_rename_goes_both_ways_and_crew_wins_a_tie() {
    let it = filed(todo(1, "b", Some("crew")), "a", true);
    assert_eq!(
        sync_of(&run(&[it.clone()], &[(1, "a", true)])),
        Some(("b".into(), true, true, false))
    );
    let gh = filed(todo(1, "a", Some("crew")), "a", true);
    assert_eq!(
        sync_of(&run(&[gh], &[(1, "c", true)])),
        Some(("c".into(), true, false, false))
    );
    assert_eq!(
        sync_of(&run(&[it], &[(1, "c", true)])),
        Some(("b".into(), true, true, false))
    );
}

#[test]
fn a_settled_done_todo_is_not_asked_about() {
    let mut it = filed(todo(1, "a", Some("crew")), "a", false);
    it.done = true;
    assert!(!live(&it));
    it.done = false;
    assert!(live(&it));
}
