//! The cards are clicked where they are drawn: the pixel under a card's
//! name or its second line is that pane; the gap between two cards is
//! nobody's; and a panel one row too short goes back to one-line rows,
//! whose clicks still land.
#![cfg(unix)]
use crate::app::CrewApp;
use crate::explorerhit::tests::at;

/// An app with the PANES section open and three named shells, `h` pixels tall.
fn app(h: f32) -> CrewApp {
    let dir = std::env::temp_dir();
    let mut a = CrewApp {
        geo_override: Some((8.0, 17.0, 1280.0, h, 1.0)),
        cwd: dir.clone(),
        ..Default::default()
    };
    a.config.show_nav = true;
    a.config.nav_collapsed = false;
    a.config.nav_section = "panes".into();
    a.input.focused = false;
    for name in ["alpha", "bravo", "charlie"] {
        a.spawn_new_pane();
        let p = a.panes.last_mut().unwrap();
        p.name = Some(name.into());
        // A short folder, so the second line shows it whole.
        p.dir = Some(std::path::PathBuf::from(format!("/zz{name}")));
    }
    a
}

#[test]
fn a_click_on_either_line_of_a_card_is_its_pane() {
    let _g = crate::app::theme_test_guard();
    let mut a = app(720.0);
    assert!(a.pane_cards(), "a tall panel draws cards");
    a.cursor = at(&mut a, "bravo");
    assert_eq!(a.pane_at_panes_list(), Some(1), "the name line");
    a.cursor = at(&mut a, "/zzcharlie");
    assert_eq!(a.pane_at_panes_list(), Some(2), "the second line");
    // One row under alpha's second line: the gap before bravo.
    let (x, y) = at(&mut a, "/zzalpha");
    a.cursor = (x, y + 17.0);
    assert_eq!(a.pane_at_panes_list(), None, "the gap is nobody's");
}

/// Exactly the rows the cards need, then one fewer (one app, its window
/// shrunk a row at a time).
#[test]
fn one_row_short_the_list_goes_back_to_one_line_rows() {
    let _g = crate::app::theme_test_guard();
    let mut a = app(720.0);
    assert_eq!(a.nav_lines().len(), 3 * 3 - 1);
    a.cursor = at(&mut a, "alpha");
    assert_eq!(a.pane_at_panes_list(), Some(0));
    let tall = |a: &mut CrewApp, h: f32| a.geo_override = Some((8.0, 17.0, 1280.0, h, 1.0));
    let mut h = 720.0;
    while a.pane_cards() {
        h -= 17.0;
        tall(&mut a, h);
    }
    assert_eq!(a.nav_lines().len(), 3, "one line a pane");
    a.cursor = at(&mut a, "charlie");
    assert_eq!(a.pane_at_panes_list(), Some(2));
    tall(&mut a, h + 17.0);
    assert!(a.pane_cards(), "a row more and the cards are back");
    a.cursor = at(&mut a, "charlie");
    assert_eq!(a.pane_at_panes_list(), Some(2));
}

/// The hover card reads what the card draws: both of its rows, its name and
/// what it is doing — so a cut second line, not only a cut name, can be read whole.
#[test]
fn the_hover_card_covers_both_lines_of_a_card() {
    let _g = crate::app::theme_test_guard();
    let mut a = app(720.0);
    a.cursor_in = true;
    a.cursor = at(&mut a, "/zzbravo");
    let t = a.nav_tip_target().expect("a target");
    assert_eq!(t.rows.len(), 2, "the name's row and the one under it");
    assert_eq!(t.rows[1], t.rows[0] + 1);
    assert_eq!(t.drawn[0], "bravo");
    assert_eq!(t.whole, vec!["bravo".to_string(), "/zzbravo".to_string()]);
    a.cursor = at(&mut a, "bravo");
    assert_eq!(
        a.nav_tip_target().unwrap().rows,
        t.rows,
        "same card from its name"
    );
}
