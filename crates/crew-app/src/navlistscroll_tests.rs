//! The wheel over the Inbox and Far panels moves a window down their lists, and a click
//! lands on the item drawn under it.
use std::path::PathBuf;

use super::window;
use crate::app::CrewApp;
use crate::inbox::wire::Thread;
use crate::navfar::tests::{drawn_at, find_drawn, nav_app};
use crate::pane::PaneContent;

#[test]
fn the_window_stops_where_the_last_item_reaches_the_bottom() {
    assert_eq!(window(0, 20, 8), (0, 8));
    assert_eq!(window(5, 20, 8), (5, 8));
    assert_eq!(window(99, 20, 8), (12, 8), "no further than the last item");
    assert_eq!(window(7, 3, 8), (0, 3), "a list that fits shows whole");
    assert_eq!(window(4, 0, 8), (0, 0));
}

/// A nav short enough that neither list fits in it: 14 rows, the rule then six threads
/// and the note under them, or 13 folders.
fn short_nav(section: &str) -> CrewApp {
    let mut a = nav_app(section);
    a.geo_override = Some((8.0, 17.0, 1280.0, 283.0, 1.0));
    a
}

/// Wheel down over the panel, notch by notch, through the entry point the event loop calls.
fn wheel_down(a: &mut CrewApp, at: (f32, f32), notches: usize) {
    a.cursor = at;
    for _ in 0..notches {
        a.scroll_at_cursor(-3, true);
    }
}

/// The drawn word among `words` that stands highest in the panel.
fn topmost<'a>(a: &mut CrewApp, words: &'a [String]) -> &'a str {
    let mut best: Option<(f32, &str)> = None;
    for w in words {
        if let Some((_, y)) = find_drawn(a, w) {
            if best.is_none_or(|(by, _)| y < by) {
                best = Some((y, w));
            }
        }
    }
    best.expect("a word drawn").1
}

#[test]
fn the_wheel_scrolls_far_and_a_click_opens_the_folder_drawn_there() {
    let _g = crate::app::theme_test_guard();
    let base = std::env::temp_dir().join(format!("navlist-far-{}", std::process::id()));
    let names: Vec<String> = (0..20).map(|i| format!("fold{i:02}")).collect();
    for n in &names {
        std::fs::create_dir_all(base.join(n)).unwrap();
        crate::farrecent::note(&base.join(n));
    }
    let mut a = short_nav("far");
    // Newest first: fold19 tops the list, fold00 is past the panel's bottom.
    assert_eq!(topmost(&mut a, &names), "fold19");
    assert!(find_drawn(&mut a, "fold00").is_none(), "cut at the bottom");
    let at = drawn_at(&mut a, "fold19");
    wheel_down(&mut a, at, 10);
    assert!(
        find_drawn(&mut a, "fold19").is_none(),
        "scrolled off the top"
    );
    let top = topmost(&mut a, &names).to_string();
    assert_ne!(top, "fold19");
    // The click reads the same window the draw did: the folder under it opens.
    a.cursor = drawn_at(&mut a, &top);
    assert!(a.far_press());
    let dir = match &a.panes[a.focused].content {
        PaneContent::Far(f) => f.active_local_dir(),
        _ => None,
    };
    assert_eq!(dir, Some(base.join(&top)));
    // …and the last folder can be reached at all.
    assert!(
        find_drawn(&mut a, "fold00").is_some(),
        "the end is in reach"
    );
    crate::farrecent::clear();
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn the_wheel_scrolls_the_inbox_and_a_click_opens_the_thread_drawn_there() {
    let _g = crate::app::theme_test_guard();
    let list: Vec<Thread> = (0..12)
        .map(|i| Thread {
            id: format!("t-{i:02}"),
            from: format!("Sender{i:02} <s{i}@x.org>"),
            subject: format!("Subject {i}"),
            unread: true,
            ..Default::default()
        })
        .collect();
    let names: Vec<String> = (0..12).map(|i| format!("Sender{i:02}")).collect();
    crate::inboxglance::seed(crate::inboxglance::count(40, list.first()), list.clone());
    let mut a = short_nav("inbox");
    assert_eq!(topmost(&mut a, &names), "Sender00");
    assert!(
        find_drawn(&mut a, "Sender11").is_none(),
        "cut at the bottom"
    );
    let at = drawn_at(&mut a, "Sender00");
    wheel_down(&mut a, at, 10);
    assert!(
        find_drawn(&mut a, "Sender00").is_none(),
        "scrolled off the top"
    );
    let top = topmost(&mut a, &names).to_string();
    // The last thread is in reach, and the note counts only what is below the window.
    assert!(find_drawn(&mut a, "Sender11").is_some());
    assert!(find_drawn(&mut a, "28 more in the inbox").is_some());
    a.cursor = drawn_at(&mut a, &top);
    assert!(a.inbox_press());
    let k: usize = top["Sender".len()..].parse().unwrap();
    if let PaneContent::Inbox(p) = &mut a.panes[a.focused].content {
        p.rows = list.clone();
    }
    assert!(a.tick_inbox_want());
    let PaneContent::Inbox(p) = &a.panes[a.focused].content else {
        unreachable!()
    };
    let want = format!("t-{k:02}");
    assert_eq!(
        p.reading.as_ref().map(|r| r.thread.id.as_str()),
        Some(want.as_str())
    );
    crate::inboxglance::seed(crate::inboxglance::Card::Off, Vec::new());
}

/// The scrolled Far panel in a short window, for a look: `project-19`…`-15` scrolled off the
/// top, `project-14` the first row drawn.
///
/// `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew nav_far_scrolled_shot -- --ignored`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn nav_far_scrolled_shot() {
    let _g = crate::app::theme_test_guard();
    let base = std::env::temp_dir().join(format!("navlist-shot-{}", std::process::id()));
    for i in 0..20 {
        let d: PathBuf = base.join(format!("project-{i:02}"));
        std::fs::create_dir_all(&d).unwrap();
        crate::farrecent::note(&d);
    }
    let prep = |app: &mut CrewApp| {
        app.config.show_nav = true;
        app.config.nav_collapsed = false;
        app.config.nav_section = "far".into();
        app.sidebar.lists.far = 5;
    };
    let id = crew_theme::ThemeId::TokyoNight;
    crate::crtglassshot_tests::window_sized("nav-far-scrolled", id, 1.0, &[], &prep, (900, 280));
    crate::farrecent::clear();
    let _ = std::fs::remove_dir_all(&base);
}
