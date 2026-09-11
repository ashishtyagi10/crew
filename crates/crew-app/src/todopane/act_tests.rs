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
