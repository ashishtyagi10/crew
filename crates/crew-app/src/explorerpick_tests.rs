//! The rule's `+`: always drawn, lit under the pointer, and a click on it opens the folder
//! chooser, whose folders are pinned as roots after the pane's.
use crate::app::CrewApp;
use crate::explorer::tests::tree;
use crate::explorerhit::tests::{app, at, CELL};

/// The `+` on the EXPLORER rule — the topmost `+` in the nav, above every folder's — as the
/// pixel at its middle and the fill behind it.
fn plus(a: &mut CrewApp) -> ((f32, f32), (u8, u8, u8)) {
    let rule = at(a, "EXPLORER");
    let scenes = a.build_frame();
    let s = scenes.iter().find(|s| s.x < 30.0 && s.h > 60.0).unwrap();
    let c = (s.cells.iter().filter(|c| c.c == '+'))
        .min_by_key(|c| c.row)
        .expect("a + drawn");
    let px = (
        s.x + (f32::from(c.col) + 0.5) * CELL.0,
        s.y + (f32::from(c.row) + 0.5) * CELL.1,
    );
    assert_eq!(px.1, rule.1, "the + is on the EXPLORER rule");
    (px, c.bg)
}

#[test]
fn the_plus_on_the_rule_adds_the_folders_chosen() {
    let _g = crate::app::theme_test_guard();
    let here = tree("pick-here", &["a.txt"]);
    let far = tree("pick-far", &["notes/b.md"]);
    let mut a = app(&here);
    let (px, idle) = plus(&mut a);
    a.cursor = px;
    assert!(a.explorer_add_at_cursor());
    let (_, lit) = plus(&mut a);
    assert_ne!(idle, lit, "the button fills under the pointer");
    // The word is the button too, not only the +.
    a.cursor = (px.0 + 3.0 * CELL.0, px.1);
    assert!(a.explorer_add_at_cursor(), "ADD is part of the button");
    crate::explorerpick::CHOSEN.with(|c| *c.borrow_mut() = vec![far.clone()]);
    assert!(a.explorer_press(), "the click was the +'s");
    assert!(
        a.sidebar.explorer.pinned().is_empty(),
        "not until it answers"
    );
    assert!(a.poll_explorer_chooser());
    assert_eq!(a.sidebar.explorer.pinned(), vec![far.clone()]);
    assert!(a.sidebar.explorer.choosing.is_none());
    let names: Vec<_> = a.explorer_items().iter().map(|i| i.name.clone()).collect();
    assert!(names.contains(&"notes".to_string()), "{names:?}");
}

#[test]
fn a_cancelled_chooser_pins_nothing_and_add_alone_opens_it() {
    let _g = crate::app::theme_test_guard();
    let here = tree("pick-none", &["a.txt"]);
    let mut a = app(&here);
    a.nav_command("explorer add");
    assert!(
        a.sidebar.explorer.choosing.is_some(),
        "add alone opens the chooser"
    );
    assert!(a.poll_explorer_chooser());
    assert!(a.sidebar.explorer.pinned().is_empty());
    let said = a.status.clone().map(|(m, _)| m).unwrap_or_default();
    assert!(said.contains("no folder added"), "{said}");
    assert!(!a.poll_explorer_chooser(), "nothing more to hear");
}
