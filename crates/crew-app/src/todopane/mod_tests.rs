//! End-to-end pane flows against the (test-guarded) shared store: create,
//! edit, toggle, delete, filter — everything the composer's Enter can mean.
use super::*;

#[test]
fn submit_creates_an_item_with_due_and_project_stripped_from_the_title() {
    let _g = store::test_guard(vec![]);
    let mut p = TodoPane::new();
    for c in "pay rent tomorrow 5pm @home".chars() {
        p.type_char(c);
    }
    p.submit();
    assert_eq!(p.input, "", "the composer clears on submit");
    let items = store::snapshot();
    assert_eq!(items.len(), 1);
    let it = &items[0];
    assert_eq!(it.title, "pay rent");
    assert_eq!(it.project.as_deref(), Some("home"));
    assert!(!it.done);
    assert!(it.due_has_time);
    let due = duedate::from_epoch_ms(it.due_ms.expect("a due was set")).unwrap();
    let tomorrow = duedate::now_local().date().succ_opt().unwrap();
    assert_eq!(due.date(), tomorrow);
    assert_eq!(
        (
            chrono::Timelike::hour(&due.time()),
            chrono::Timelike::minute(&due.time())
        ),
        (17, 0)
    );
    assert!(!it.notified, "a future due keeps its toast");
}

#[test]
fn submit_without_a_date_or_tag_is_a_plain_item() {
    let _g = store::test_guard(vec![]);
    let mut p = TodoPane::new();
    p.paste("just a plain thing");
    p.submit();
    let items = store::snapshot();
    assert_eq!(items[0].title, "just a plain thing");
    assert_eq!(items[0].project, None);
    assert_eq!(items[0].due_ms, None);
}

#[test]
fn a_date_or_tag_alone_never_creates_an_empty_titled_item() {
    let _g = store::test_guard(vec![]);
    let mut p = TodoPane::new();
    p.paste("tomorrow");
    p.submit();
    assert_eq!(store::snapshot().len(), 0);
    assert_eq!(
        p.input, "tomorrow",
        "the draft stays for the user to finish"
    );
}

#[test]
fn toggle_delete_and_ids_go_through_the_display_order() {
    let _g = store::test_guard(vec![]);
    let mut p = TodoPane::new();
    p.paste("bbb");
    p.submit();
    p.paste("aaa tomorrow"); // dated → sorts above the undated bbb
    p.submit();
    let order = p.order();
    assert_eq!(p.items[order[0]].title, "aaa");
    assert_eq!(p.items[order[1]].title, "bbb");

    p.toggle_done_at(0); // aaa done → hides, but stays in the store
    let order = p.order();
    assert_eq!(order.len(), 1);
    assert_eq!(p.items[order[0]].title, "bbb");
    assert!(store::snapshot()
        .iter()
        .any(|it| it.title == "aaa" && it.done));

    p.delete_at(0); // display index 0 is bbb now that aaa is hidden
    let titles: Vec<String> = store::snapshot()
        .iter()
        .map(|it| it.title.clone())
        .collect();
    assert_eq!(titles, vec!["aaa"]);
}

#[test]
fn edit_round_trips_title_tag_and_due() {
    let _g = store::test_guard(vec![]);
    let mut p = TodoPane::new();
    p.paste("ship build 2027-03-05 17:30 @crew");
    p.submit();
    let id = store::snapshot()[0].id;
    let due = store::snapshot()[0].due_ms;

    p.edit_at(0);
    assert_eq!(p.input, "ship build @crew 2027-03-05 17:30");
    assert_eq!(p.editing, Some(id));
    // Resubmitting unchanged keeps everything, including the due instant.
    p.submit();
    let items = store::snapshot();
    assert_eq!(items.len(), 1, "edit replaces, never duplicates");
    assert_eq!(items[0].id, id);
    assert_eq!(items[0].title, "ship build");
    assert_eq!(items[0].project.as_deref(), Some("crew"));
    assert_eq!(items[0].due_ms, due);

    // And an actual change lands.
    p.edit_at(0);
    p.input = "ship build @crew".into();
    p.submit();
    let items = store::snapshot();
    assert_eq!(items[0].due_ms, None, "removing the date clears the due");
}

#[test]
fn a_lone_tag_filters_and_a_lone_at_clears() {
    let _g = store::test_guard(vec![]);
    let mut p = TodoPane::new();
    p.paste("one @crew");
    p.submit();
    p.paste("two @home");
    p.submit();
    p.paste("@crew");
    p.submit();
    assert_eq!(p.filter.as_deref(), Some("crew"));
    assert_eq!(p.visible_len(), 1);
    assert_eq!(store::snapshot().len(), 2, "filtering created no item");
    p.paste("@");
    p.submit();
    assert_eq!(p.filter, None);
    assert_eq!(p.visible_len(), 2);
}

#[test]
fn extract_tag_takes_the_first_token_only() {
    assert_eq!(
        extract_tag("pay rent @home @extra"),
        ("pay rent @extra".to_string(), Some("home".to_string()))
    );
    assert_eq!(
        extract_tag("no tags here"),
        ("no tags here".to_string(), None)
    );
    // A bare `@` is not a tag.
    assert_eq!(
        extract_tag("weird @ thing"),
        ("weird @ thing".to_string(), None)
    );
}

#[test]
fn poll_resyncs_when_another_pane_writes() {
    let _g = store::test_guard(vec![]);
    let mut a = TodoPane::new();
    let mut b = TodoPane::new();
    assert!(!a.poll(), "no writes yet — no redraw");
    b.paste("shared thing");
    b.submit();
    assert!(a.poll(), "the other pane's write is seen");
    assert_eq!(a.items.len(), 1);
    assert_eq!(a.items[0].title, "shared thing");
    assert!(!a.poll(), "and only once");
}

/// App-level: the slash command opens (and focuses) a todo pane, Escape on
/// its empty composer closes it — the whole route, headless.
#[test]
fn slash_todo_opens_a_pane_and_escape_closes_it() {
    let _g = store::test_guard(vec![]);
    let mut app = crate::app::CrewApp::default();
    let exit = app.run_slash_command("todo");
    assert!(!exit);
    assert_eq!(app.panes.len(), 1);
    assert_eq!(app.panes[0].title_text(), "todo");
    // The session snapshot carries it, and the saved kind restores.
    let saved = app.session_panes();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].kind, "todo");
    assert!(saved[0].restorable_with(|_| false), "no dir needed");
    // Escape with an empty composer asks the app to close the pane.
    let crate::pane::PaneContent::Todo(t) = &mut app.panes[0].content else {
        panic!("a todo pane was spawned");
    };
    assert!(matches!(
        crate::todopane::keys::apply(t, crate::todopane::keys::TodoInput::Close, 40, 20),
        Some(TodoAction::Close)
    ));
}

#[test]
fn typing_at_opens_the_known_tag_popup() {
    let _g = store::test_guard(vec![]);
    let mut p = TodoPane::new();
    p.paste("one @crew");
    p.submit();
    for c in "two @c".chars() {
        p.type_char(c);
    }
    let m = p.tagmenu.as_ref().expect("popup open while typing @c");
    assert_eq!(m.matches, vec!["crew"]);
}

// --- done history (v0.17: `/todo done`) -----------------------------------

#[test]
fn ticking_stamps_done_ms_and_unticking_clears_it() {
    let _g = store::test_guard(vec![]);
    let mut p = TodoPane::new();
    for c in "ship it".chars() {
        p.type_char(c);
    }
    p.submit();
    let before = crate::chattime::unix_now_ms();
    p.toggle_done_at(0);
    let it = store::snapshot().remove(0);
    assert!(it.done);
    let stamp = it.done_ms.expect("a tick stamps done_ms");
    assert!(stamp >= before, "the stamp is the tick instant");

    // Un-tick (via the sunk show_done row) clears the stamp: the item is
    // open again, not "done at some stale instant".
    p.show_done = true;
    p.toggle_done_at(0);
    let it = store::snapshot().remove(0);
    assert!(!it.done);
    assert_eq!(it.done_ms, None, "un-tick clears the stamp");
}

#[test]
fn the_done_view_composer_filters_but_never_creates() {
    let _g = store::test_guard(vec![]);
    let mut p = TodoPane::new();
    for c in "real work @crew".chars() {
        p.type_char(c);
    }
    p.submit();
    p.toggle_done_at(0);
    p.done_view = true;
    assert_eq!(p.visible_len(), 1, "the history lists the done item");

    for c in "sneaky new todo".chars() {
        p.type_char(c);
    }
    p.submit();
    assert_eq!(
        store::snapshot().len(),
        1,
        "no item is born from inside the history"
    );

    p.cancel_edit();
    for c in "@crew".chars() {
        p.type_char(c);
    }
    p.tagmenu = None; // submit directly, popup path is keys::apply's
    p.submit();
    assert_eq!(p.filter.as_deref(), Some("crew"), "filtering still works");
    assert_eq!(p.visible_len(), 1);
}
