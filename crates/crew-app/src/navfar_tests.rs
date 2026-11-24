//! The Far section's rows: one folder each, its name then where it is.
use super::*;

fn text(cells: &[CellView], row: u16) -> String {
    let mut r: Vec<_> = cells.iter().filter(|c| c.row == row).collect();
    r.sort_by_key(|c| c.col);
    r.iter().map(|c| c.c).collect()
}

#[test]
fn a_folder_reads_as_its_name_then_its_place() {
    let (name, at) = name_and_parent(Path::new("/srv/data/logs"));
    assert_eq!((name.as_str(), at.as_str()), ("logs", "/srv/data"));
    let (root, _) = name_and_parent(Path::new("/"));
    assert_eq!(root, "/");
}

#[test]
fn each_row_is_one_folder_newest_first() {
    let _g = crate::app::theme_test_guard();
    let list = [PathBuf::from("/srv/one"), PathBuf::from("/srv/two")];
    let cells = cells(&list, (0, 2), None, (30, 10));
    assert!(text(&cells, 1).contains("one"));
    assert!(text(&cells, 1).contains("/srv"));
    assert!(text(&cells, 2).contains("two"));
    assert_eq!(item_at(0, 2), None, "the rule");
    assert_eq!(item_at(2, 2), Some(1));
    assert_eq!(item_at(3, 2), None);
}

#[test]
fn no_folders_yet_says_how_to_start() {
    let _g = crate::app::theme_test_guard();
    let cells = cells(&[], (0, 0), None, (30, 10));
    assert!(text(&cells, 1).contains("no folders yet"));
    assert!(text(&cells, 2).contains("open far"));
}

/// The pixel at the middle of where `word` is drawn in the nav, in a real frame.
pub(crate) fn drawn_at(a: &mut CrewApp, word: &str) -> (f32, f32) {
    find_drawn(a, word).unwrap_or_else(|| panic!("{word} drawn"))
}

/// Where `word` is drawn in the nav, if it is.
pub(crate) fn find_drawn(a: &mut CrewApp, word: &str) -> Option<(f32, f32)> {
    let scenes = a.build_frame();
    let s = scenes
        .iter()
        .find(|s| s.x < 30.0 && s.h > 60.0)
        .expect("the nav");
    let w: Vec<char> = word.chars().collect();
    let c = s.cells.iter().find(|c| {
        w.iter().enumerate().all(|(i, ch)| {
            s.cells
                .iter()
                .any(|d| d.row == c.row && d.col == c.col + i as u16 && d.c == *ch)
        })
    })?;
    Some((
        s.x + (f32::from(c.col) + 0.5) * 8.0,
        s.y + (f32::from(c.row) + 0.5) * 17.0,
    ))
}

pub(crate) fn nav_app(section: &str) -> CrewApp {
    let mut a = CrewApp {
        geo_override: Some((8.0, 17.0, 1280.0, 720.0, 1.0)),
        ..Default::default()
    };
    a.config.show_nav = true;
    a.config.nav_collapsed = false;
    a.config.nav_section = section.into();
    a.input.focused = false;
    a
}

/// A click on a drawn folder opens Far there; a second click, on another folder, takes the
/// same Far pane there rather than opening another.
#[test]
fn a_click_on_a_folder_opens_far_there() {
    let _g = crate::app::theme_test_guard();
    let base = std::env::temp_dir().join(format!("navfar-click-{}", std::process::id()));
    let (one, two) = (base.join("alphadir"), base.join("betadir"));
    std::fs::create_dir_all(&one).unwrap();
    std::fs::create_dir_all(&two).unwrap();
    crate::farrecent::note(&one);
    crate::farrecent::note(&two);
    let mut a = nav_app("far");
    a.cursor = drawn_at(&mut a, "alphadir");
    assert!(a.far_press());
    let dir = |a: &CrewApp| match &a.panes[a.focused].content {
        PaneContent::Far(f) => f.active_local_dir(),
        _ => None,
    };
    assert_eq!(a.panes.len(), 1);
    assert_eq!(dir(&a), Some(one.clone()));
    a.cursor = drawn_at(&mut a, "betadir");
    assert!(a.far_press());
    assert_eq!(a.panes.len(), 1, "the open Far pane went there");
    assert_eq!(dir(&a), Some(two.clone()));
    // …and the move is what the list learns next tick.
    a.tick_far_recent();
    assert_eq!(crate::farrecent::list().first(), Some(&two));
    let _ = std::fs::remove_dir_all(&base);
}
