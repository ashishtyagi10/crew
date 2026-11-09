use super::*;

#[test]
fn tags_and_title_come_apart() {
    let e = parse("fix the build @crew #ana").unwrap();
    assert_eq!(e.title, "fix the build");
    assert_eq!(e.project.as_deref(), Some("crew"));
    assert_eq!(e.assignee.as_deref(), Some("ana"));
    assert_eq!(e.due_ms, None);
}

#[test]
fn a_due_is_cut_out_of_the_title() {
    let e = parse("ship it tomorrow 5pm @crew").unwrap();
    assert_eq!(e.title, "ship it");
    assert!(e.due_ms.is_some() && e.has_time);
}

#[test]
fn a_tag_or_date_alone_is_not_an_item() {
    assert_eq!(parse("@crew"), None);
    assert_eq!(parse("  tomorrow "), None);
}
