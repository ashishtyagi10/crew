use super::*;

fn tagged(project: Option<&str>) -> TodoItem {
    TodoItem {
        id: 1,
        title: "t".into(),
        done: false,
        done_ms: None,
        project: project.map(str::to_string),
        due_ms: None,
        due_has_time: false,
        created_ms: 0,
        notified: false,
    }
}

#[test]
fn pending_tag_reads_the_trailing_token_leading_included() {
    // Unlike the chat composer, the LEADING token is a tag too — a todo
    // pane has no @agent routing to reserve it for.
    assert_eq!(pending_tag("@cr"), Some("cr"));
    assert_eq!(pending_tag("pay rent @ho"), Some("ho"));
    assert_eq!(pending_tag("@"), Some(""));
    assert_eq!(pending_tag("pay rent"), None);
    // A completed tag (trailing space) is no longer pending.
    assert_eq!(pending_tag("pay rent @home "), None);
}

#[test]
fn known_tags_dedupe_case_insensitively_most_used_first() {
    let items = vec![
        tagged(Some("home")),
        tagged(Some("Crew")),
        tagged(Some("crew")),
        tagged(None),
        tagged(Some("alpha")),
    ];
    // crew used twice (first-seen spelling kept); home/alpha tie → alphabetical.
    assert_eq!(known_tags(&items), vec!["Crew", "alpha", "home"]);
}

#[test]
fn filter_ranks_prefix_over_substring_over_subsequence() {
    let tags = vec!["crew".to_string(), "screwdriver".into(), "carew-x".into()];
    assert_eq!(
        filter_tags(&tags, "cr"),
        vec!["crew", "screwdriver", "carew-x"]
    );
    assert_eq!(filter_tags(&tags, "zzz"), Vec::<String>::new());
    // Empty query keeps the incoming (usage) order.
    assert_eq!(filter_tags(&tags, ""), tags);
}

#[test]
fn accept_splices_the_tag_and_closes_the_token() {
    assert_eq!(accept("pay rent @ho", "home"), "pay rent @home ");
    assert_eq!(accept("@cr", "crew"), "@crew ");
}

#[test]
fn after_edit_opens_narrows_and_closes() {
    let tags = || vec!["crew".to_string(), "home".to_string()];
    let mut menu = None;
    after_edit(&mut menu, "pay @", tags);
    assert_eq!(menu.as_ref().unwrap().matches, vec!["crew", "home"]);
    after_edit(&mut menu, "pay @h", tags);
    assert_eq!(menu.as_ref().unwrap().matches, vec!["home"]);
    // No match (a brand-new tag) → no popup; it's accepted free-form later.
    after_edit(&mut menu, "pay @zzz", tags);
    assert!(menu.is_none());
    after_edit(&mut menu, "pay @home ", tags);
    assert!(menu.is_none(), "a finished token closes the popup");
}
