use crate::todopane::{store, TodoPane};

#[test]
fn the_two_filters_are_independent_and_compose() {
    let _g = store::test_guard(vec![]);
    let mut p = TodoPane::new();
    for draft in ["one @crew #priya", "two @crew #sam", "three @home #priya"] {
        p.paste(draft);
        p.submit();
    }
    assert_eq!(store::snapshot().len(), 3);
    p.paste("#priya");
    p.submit();
    assert_eq!(p.who.as_deref(), Some("priya"));
    assert_eq!(p.visible_len(), 2, "priya's two, across both projects");
    p.paste("@crew");
    p.submit();
    assert_eq!(p.visible_len(), 1, "AND-ed: priya's crew item only");
    assert_eq!(p.filter.as_deref(), Some("crew"), "the project axis held");
    p.paste("#");
    p.submit();
    assert_eq!(p.who, None, "a bare # clears only its own axis");
    assert_eq!(p.filter.as_deref(), Some("crew"));
    assert_eq!(p.visible_len(), 2);
}

#[test]
fn an_assignee_round_trips_through_an_edit() {
    let _g = store::test_guard(vec![]);
    let mut p = TodoPane::new();
    p.paste("ship notes @crew #priya tomorrow");
    p.submit();
    p.edit_at(0);
    assert!(
        p.input.contains("#priya"),
        "the edit draft has to carry the owner back: {:?}",
        p.input
    );
    p.submit();
    let items = store::snapshot();
    assert_eq!(items.len(), 1, "an edit replaces, it does not add");
    assert_eq!(items[0].assignee.as_deref(), Some("priya"));
    assert_eq!(items[0].project.as_deref(), Some("crew"));
    assert_eq!(items[0].title, "ship notes");
}

/// `e` then Enter keeps a todo exactly as it was: a title ending in "on"
/// lost the word to the due's lead-in, and a title's own `@types/node`
/// became the project.
#[test]
fn editing_unchanged_keeps_the_todo_whole() {
    let day = crate::chattime::unix_now_ms() + 3 * 86_400_000;
    let it = |title: &str, project: Option<&str>| super::super::item::TodoItem {
        id: 7,
        title: title.into(),
        project: project.map(str::to_string),
        due_ms: Some(day),
        ..Default::default()
    };
    for want in [
        it("turn the heater on", None),
        it("Bump @types/node to 22", Some("crew")),
    ] {
        let _g = super::super::store::test_guard(vec![want.clone()]);
        let mut p = super::super::TodoPane::new();
        p.edit_at(0);
        p.submit();
        let got = &super::super::store::snapshot()[0];
        assert_eq!(
            (&got.title, &got.project),
            (&want.title, &want.project),
            "{}",
            p.input
        );
    }
}

/// A todo deleted elsewhere while it was open for editing keeps what was
/// typed, as a todo of its own: Enter dropped the edit without a word.
#[test]
fn an_edit_of_a_todo_deleted_meanwhile_is_kept() {
    let it = super::super::item::TodoItem {
        id: 7,
        title: "call mom".into(),
        ..Default::default()
    };
    let _g = super::super::store::test_guard(vec![it]);
    let mut p = super::super::TodoPane::new();
    p.edit_at(0);
    super::super::store::mutate(|items| items.clear());
    p.paste(" back");
    p.submit();
    let now = super::super::store::snapshot();
    assert_eq!(now.len(), 1, "kept");
    assert_eq!(now[0].title, "call mom back", "as typed");
}
