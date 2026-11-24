//! A Far row's `×` forgets its folder (a click, or Delete on the keyboard's cursor), a folder
//! gone from disk is drawn struck, and the open Far pane's folder wears an accent `▸`.
use std::path::PathBuf;

use winit::keyboard::{Key, NamedKey};

use super::*;
use crate::navfar::tests::{drawn_at, find_drawn, nav_app};

/// `names` made as real folders under a fresh temp base and noted, the last newest.
pub(super) fn folders(tag: &str, names: &[&str]) -> (PathBuf, Vec<PathBuf>) {
    crate::farrecent::clear();
    let base = std::env::temp_dir().join(format!("navfarrow-{tag}-{}", std::process::id()));
    let dirs: Vec<PathBuf> = names.iter().map(|n| base.join(n)).collect();
    for d in &dirs {
        std::fs::create_dir_all(d).unwrap();
        crate::farrecent::note(d);
    }
    (base, dirs)
}

pub(super) fn done(base: &PathBuf) {
    crate::farrecent::clear();
    let _ = std::fs::remove_dir_all(base);
}

/// The press the event loop makes, so the `×` is proved ahead of the row's open.
fn click(a: &mut CrewApp, at: (f32, f32)) {
    a.cursor = at;
    assert!(matches!(a.docked_press(), crate::btnpress::Press::Done));
}

#[test]
fn hovering_a_row_shows_its_x_and_a_click_on_it_forgets_without_opening() {
    let _g = crate::app::theme_test_guard();
    let (base, dirs) = folders("x", &["alphadir", "betadir"]);
    let mut a = nav_app("far");
    assert_eq!(
        find_drawn(&mut a, "\u{d7}"),
        None,
        "no × until a row is hovered"
    );
    let name = drawn_at(&mut a, "alphadir");
    a.cursor = name;
    let x = drawn_at(&mut a, "\u{d7}");
    assert_eq!(x.1, name.1, "the × is on the hovered row");
    assert_eq!(a.far_x_at_cursor(), None, "on the name, not the ×");
    a.cursor = x;
    assert_eq!(
        a.far_x_at_cursor(),
        Some(1),
        "the hit reads the column the × is drawn in"
    );
    click(&mut a, x);
    assert_eq!(
        crate::farrecent::list(),
        [dirs[1].clone()],
        "alphadir forgotten"
    );
    assert!(a.panes.is_empty(), "and not opened");
    done(&base);
}

#[test]
fn a_click_one_column_left_of_the_x_still_opens_the_folder() {
    let _g = crate::app::theme_test_guard();
    let (base, dirs) = folders("left", &["gammadir"]);
    let mut a = nav_app("far");
    a.cursor = drawn_at(&mut a, "gammadir");
    let x = drawn_at(&mut a, "\u{d7}");
    click(&mut a, (x.0 - 8.0, x.1));
    assert_eq!(a.panes.len(), 1, "a Far pane opened");
    assert_eq!(
        crate::farrecent::list(),
        [dirs[0].clone()],
        "nothing forgotten"
    );
    done(&base);
}

#[test]
fn delete_with_the_keys_held_forgets_the_cursors_folder() {
    let _g = crate::app::theme_test_guard();
    let (base, dirs) = folders("keys", &["kone", "ktwo", "kthree"]);
    let mut a = nav_app("panes");
    a.input.focused = true;
    a.nav_command("far");
    assert!(a.list_key(&Key::Named(NamedKey::ArrowDown), true));
    assert!(a.list_key(&Key::Named(NamedKey::Delete), true));
    assert_eq!(crate::farrecent::list(), [dirs[2].clone(), dirs[0].clone()]);
    assert!(a.list_has_keys(), "the list keeps the keys");
    assert!(a.list_key(&Key::Named(NamedKey::Backspace), true));
    assert_eq!(
        crate::farrecent::list(),
        [dirs[2].clone()],
        "the cursor stayed on the last"
    );
    assert!(a.panes.is_empty());
    done(&base);
}

#[test]
fn a_folder_gone_from_disk_is_drawn_muted_and_struck_after_the_look() {
    let _g = crate::app::theme_test_guard();
    let (base, dirs) = folders("gone", &["livedir", "deaddir"]);
    let mut a = nav_app("far");
    std::fs::remove_dir_all(&dirs[1]).unwrap();
    let struck = |a: &CrewApp| {
        let cells = a.far_cells(30, 10);
        // Newest first: deaddir is row 1, livedir row 2.
        let ind = crate::navtext::INDENT;
        let name = |row| {
            let c = cells.iter().find(|c| c.row == row && c.col == ind).unwrap();
            (c.deco.strike, c.fg)
        };
        (name(1), name(2))
    };
    assert!(!struck(&a).0 .0, "not looked at yet");
    assert!(crate::fargone::refresh(0, true), "the look finds it gone");
    assert!(!crate::fargone::refresh(1, true), "throttled");
    let (dead, live) = struck(&a);
    assert!(dead.0 && !live.0);
    assert_eq!(dead.1, crew_theme::theme().text_muted);
    assert_eq!(live.1, crew_theme::theme().ink);
    // The tick looks too, while Far shows.
    crate::fargone::reset();
    a.tick_far_recent();
    assert!(crate::fargone::is_gone(&dirs[1]));
    done(&base);
}

#[test]
fn the_open_far_panes_folder_wears_an_accent_gutter() {
    let _g = crate::app::theme_test_guard();
    let (base, _) = folders("here", &["heredir", "yonderdir"]);
    let mut a = nav_app("far");
    a.cursor = drawn_at(&mut a, "heredir");
    assert!(a.far_press());
    a.cursor = (0.0, 0.0);
    let cells = a.far_cells(30, 10);
    let gutter = |row| {
        cells
            .iter()
            .find(|c| c.row == row && c.col == 1)
            .unwrap()
            .fg
    };
    // yonderdir (newest) is row 1, heredir row 2.
    assert_eq!(gutter(2), crate::palette::accent(), "here");
    assert_eq!(gutter(1), crew_theme::theme().text_muted);
    done(&base);
}

#[test]
fn a_hovered_x_lights_when_the_pointer_is_on_it() {
    let _g = crate::app::theme_test_guard();
    let look = |on_x| RowLook {
        lit: true,
        on_x,
        ..Default::default()
    };
    let x_fg = |on_x| {
        let mut out = Vec::new();
        draw(&mut out, std::path::Path::new("/srv/a"), 1, 30, look(on_x));
        out.iter()
            .find(|c| c.c == '\u{d7}' && c.col == x_col(30))
            .unwrap()
            .fg
    };
    assert_eq!(x_fg(true), crate::palette::accent());
    assert_eq!(x_fg(false), crew_theme::theme().text_muted);
}
