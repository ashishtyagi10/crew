//! The Inbox and Far lists from the keyboard: `/nav` hands them the keys, the arrows walk,
//! Enter opens what a click would, Esc and any focus change give the keys back.
use std::path::PathBuf;

use winit::keyboard::{Key, NamedKey};

use crate::app::CrewApp;
use crate::inbox::wire::Thread;
use crate::navfar::tests::{find_drawn, nav_app};
use crate::navsection::NavSection;
use crate::pane::PaneContent;

fn press(a: &mut CrewApp, k: NamedKey) -> bool {
    a.list_key(&Key::Named(k), true, false)
}

fn item(a: &CrewApp) -> usize {
    a.sidebar.lists.keys.expect("the list has the keys").item
}

/// `n` real folders noted in Far's recent list, newest (the last made) first.
fn folders(tag: &str, n: usize) -> (PathBuf, Vec<String>) {
    let base = std::env::temp_dir().join(format!("navlistkeys-{tag}-{}", std::process::id()));
    let names: Vec<String> = (0..n).map(|i| format!("kdir{i:02}")).collect();
    for name in &names {
        std::fs::create_dir_all(base.join(name)).unwrap();
        crate::farrecent::note(&base.join(name));
    }
    (base, names)
}

/// The app as the user finds it: the bar has the keys, the nav shows Panes.
fn app() -> CrewApp {
    let mut a = nav_app("panes");
    a.input.focused = true;
    a
}

#[test]
fn nav_far_down_down_enter_opens_far_at_the_third_folder() {
    let _g = crate::app::theme_test_guard();
    crate::farrecent::clear();
    let (base, _) = folders("far", 4);
    let mut a = app();
    a.nav_command("far");
    assert!(
        a.list_has_keys() && !a.input.focused,
        "the list took the keys"
    );
    assert!(press(&mut a, NamedKey::ArrowDown));
    assert!(a.list_key(&Key::Character("j".into()), true, false));
    assert_eq!(item(&a), 2);
    assert!(a.list_key(&Key::Character("k".into()), true, false));
    press(&mut a, NamedKey::ArrowDown);
    assert!(press(&mut a, NamedKey::Enter));
    let want = crate::farrecent::list()[2].clone();
    let PaneContent::Far(f) = &a.panes[a.focused].content else {
        panic!("a Far pane opened")
    };
    assert_eq!(f.active_local_dir(), Some(want));
    assert!(!a.list_has_keys(), "the pane it opened has the keys");
    crate::farrecent::clear();
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn nav_inbox_down_enter_asks_for_the_second_thread() {
    let _g = crate::app::theme_test_guard();
    let th = |id: &str, from: &str| Thread {
        id: id.into(),
        from: from.into(),
        subject: "Hi".into(),
        unread: true,
        ..Default::default()
    };
    let list = vec![th("t-one", "Ann <a@x.org>"), th("t-two", "Bo <b@x.org>")];
    crate::inboxglance::seed(crate::inboxglance::count(2, list.first()), list);
    let mut a = app();
    a.nav_command("inbox");
    assert!(a.list_has_keys());
    press(&mut a, NamedKey::ArrowDown);
    press(&mut a, NamedKey::End);
    assert_eq!(item(&a), 1, "End stops on the last thread");
    press(&mut a, NamedKey::Home);
    press(&mut a, NamedKey::ArrowDown);
    assert!(press(&mut a, NamedKey::Enter));
    assert!(matches!(a.panes[a.focused].content, PaneContent::Inbox(_)));
    assert_eq!(crate::navinboxhit::take_want().as_deref(), Some("t-two"));
    crate::inboxglance::seed(crate::inboxglance::Card::Off, Vec::new());
}

#[test]
fn esc_or_a_focus_change_gives_the_keys_back() {
    let _g = crate::app::theme_test_guard();
    crate::farrecent::clear();
    let (base, _) = folders("esc", 2);
    let mut a = app();
    a.nav_command("far");
    assert!(press(&mut a, NamedKey::Escape));
    assert!(!a.list_has_keys());
    a.nav_command("far");
    a.input.focused = true;
    let _ = a.build_frame();
    a.input.focused = false;
    assert!(
        !a.list_has_keys(),
        "Cmd+I then Esc: the hold is gone for good"
    );
    a.nav_command("far");
    a.cursor = (900.0, 400.0);
    let _ = a.docked_press();
    assert!(a.sidebar.lists.keys.is_none(), "a click elsewhere");
    a.nav_command("far");
    assert!(
        !a.list_key(&Key::Character("x".into()), true, false),
        "x goes on to the pane"
    );
    assert!(!a.list_has_keys());
    crate::farrecent::clear();
    let _ = std::fs::remove_dir_all(&base);
}

/// Walking past the panel's bottom scrolls the list so the cursor's folder is still drawn —
/// and barred.
#[test]
fn the_cursor_scrolls_the_window_past_the_bottom() {
    let _g = crate::app::theme_test_guard();
    crate::farrecent::clear();
    let (base, names) = folders("scroll", 20);
    let mut a = app();
    // Short enough that the 20 folders do not fit.
    a.geo_override = Some((8.0, 17.0, 1280.0, 283.0, 1.0));
    a.nav_command("far");
    let last = names[0].clone(); // the oldest, listed last
    assert!(
        find_drawn(&mut a, &last).is_none(),
        "past the bottom at first"
    );
    press(&mut a, NamedKey::End);
    assert_eq!(item(&a), 19);
    assert!(find_drawn(&mut a, &last).is_some(), "the window followed");
    let rows = a.list_rows().expect("the panel");
    let (first, shown) = a.list_window(NavSection::Far, rows);
    assert_eq!(first + shown, 20, "the cursor's folder is the last drawn");
    let cells = a.far_cells(30, rows);
    let (_, fill) = crate::segment::inked(crate::palette::accent());
    let barred: Vec<u16> = (cells.iter().filter(|c| c.bg == fill))
        .map(|c| c.row)
        .collect();
    assert_eq!(barred.len(), 30, "one row, every column");
    assert!(barred
        .iter()
        .all(|&r| r == crate::navfar::TOP + shown as u16 - 1));
    press(&mut a, NamedKey::Home);
    assert_eq!(a.list_window(NavSection::Far, rows).0, 0, "and back up");
    crate::farrecent::clear();
    let _ = std::fs::remove_dir_all(&base);
}

/// The Far list holding the keys, the cursor bar two folders down.
/// `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew navlist_keys_shot -- --ignored`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn navlist_keys_shot() {
    let _g = crate::app::theme_test_guard();
    crate::farrecent::clear();
    let (base, _) = folders("shot", 6);
    for id in [
        crew_theme::ThemeId::TokyoNight,
        crew_theme::ThemeId::Porcelain,
    ] {
        let name = format!("navlist-farkeys-{id:?}").to_lowercase();
        let prep = |app: &mut CrewApp| {
            app.nav_command("far");
            press(app, NamedKey::ArrowDown);
            press(app, NamedKey::ArrowDown);
        };
        let opacity = crate::tubesheer::sheer(id.theme());
        crate::crtglassshot_tests::window_with(&name, id, opacity, &["/far"], &prep);
    }
    crate::farrecent::clear();
    let _ = std::fs::remove_dir_all(&base);
}
