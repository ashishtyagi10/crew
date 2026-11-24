//! A click on an unread thread opens the inbox on it, once the pane's list holds it.
use crate::inbox::wire::Thread;
use crate::navfar::tests::{drawn_at, nav_app};
use crate::pane::PaneContent;

#[test]
fn a_click_on_a_thread_opens_it_when_the_list_lands() {
    let _g = crate::app::theme_test_guard();
    let t = Thread {
        id: "t-zed".into(),
        from: "Zed Quill <z@q.org>".into(),
        subject: "Budget".into(),
        unread: true,
        ..Default::default()
    };
    let card = crate::inboxglance::count(1, Some(&t));
    crate::inboxglance::seed(card, vec![t.clone()]);
    let mut a = nav_app("inbox");
    a.cursor = drawn_at(&mut a, "Zed Quill");
    assert!(a.inbox_press());
    assert!(
        matches!(a.panes[a.focused].content, PaneContent::Inbox(_)),
        "the inbox pane opened"
    );
    // The list lands with the thread second: the tick opens that one.
    if let PaneContent::Inbox(p) = &mut a.panes[a.focused].content {
        let other = Thread {
            id: "t-other".into(),
            ..Default::default()
        };
        p.rows = vec![other, t];
    }
    assert!(a.tick_inbox_want());
    let PaneContent::Inbox(p) = &a.panes[a.focused].content else {
        unreachable!()
    };
    assert_eq!(p.sel, 1);
    assert_eq!(
        p.reading.as_ref().map(|r| r.thread.id.as_str()),
        Some("t-zed")
    );
    assert!(!a.tick_inbox_want(), "asked once");
    crate::inboxglance::seed(crate::inboxglance::Card::Off, Vec::new());
}
