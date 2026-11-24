//! The hover card: a cut nav row, shown whole after the pointer rests on it.
use super::*;
use crate::navfar::tests::{drawn_at, nav_app};
use crew_render::PaneScene;

/// The card on a frame, if there is one: the overlay standing right of the nav, its rows
/// joined (a path wraps with no space to break at, so the rows join with nothing).
pub(super) fn card(scenes: &[PaneScene]) -> Option<(PaneScene, String)> {
    card_of(scenes, "path")
}

/// [`card`] for a section's legend.
fn card_of(scenes: &[PaneScene], name: &str) -> Option<(PaneScene, String)> {
    let legend = |s: &PaneScene| {
        let mut top: Vec<_> = s.cells.iter().filter(|c| c.row == 0).collect();
        top.sort_by_key(|c| c.col);
        top.iter()
            .map(|c| c.c)
            .collect::<String>()
            .contains(&format!(" {name} "))
    };
    let s = scenes
        .iter()
        .find(|s| s.overlay && s.x > 30.0 && legend(s))?;
    let rows = s.cells.iter().map(|c| c.row).max().unwrap_or(0);
    let cols = s.cells.iter().map(|c| c.col).max().unwrap_or(0);
    let text = (1..rows)
        .map(|r| {
            let mut v: Vec<_> = s
                .cells
                .iter()
                .filter(|c| c.row == r && c.col > 0 && c.col < cols)
                .collect();
            v.sort_by_key(|c| c.col);
            v.iter().map(|c| c.c).collect::<String>().trim().to_string()
        })
        .collect();
    Some((s.clone(), text))
}

/// A Far folder deep enough that the nav cuts where it is.
fn far_with(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let base = std::env::temp_dir().join(format!("navtip-{}-{name}", std::process::id()));
    let dir = base
        .join("a-rather-long-folder-name")
        .join("and-deeper")
        .join(name);
    std::fs::create_dir_all(&dir).unwrap();
    crate::farrecent::note(&dir);
    (base, dir)
}

/// Where the clock tests' pointer rests.
const P: (f32, f32) = (10.0, 10.0);

#[test]
fn the_clock_waits_then_asks_for_one_frame() {
    let key = Some((NavSection::Far, 0));
    let mut t = NavTip::default();
    assert!(!t.observe(key, true, (0, 1000), P), "not before the wait");
    assert!(!t.take_due(1200, 0, P), "not owed yet");
    assert!(t.take_due(1400, 0, P), "owed at the deadline");
    assert!(!t.take_due(1450, 0, P), "owed once");
    assert!(t.observe(key, true, (0, 1400), P), "up after the wait");
    assert!(
        t.take_due(1500, 1450, P),
        "a key puts it away, and that is a frame"
    );
    assert!(
        !t.observe(key, true, (1450, 1500), P),
        "and it stays away on that row"
    );
    assert!(!t.take_due(5000, 1450, P));
    // A row drawn whole: nothing, ever.
    let mut t = NavTip::default();
    assert!(!t.observe(Some((NavSection::Far, 1)), false, (0, 0), P));
    assert!(!t.observe(Some((NavSection::Far, 1)), false, (0, 5000), P));
    assert!(!t.take_due(5000, 0, P));
}

/// The user's path: rest on a cut Far folder, the card says its full path; off the row it
/// goes; a click on the row while it shows still opens the folder.
#[test]
fn a_cut_far_row_shows_its_full_path() {
    let _g = crate::app::theme_test_guard();
    crate::farrecent::clear();
    let (base, dir) = far_with("tipdir");
    let mut a = nav_app("far");
    a.cursor_in = true;
    a.cursor = drawn_at(&mut a, "tipdir");
    assert!(card(&a.build_frame()).is_none(), "not before the wait");
    let now = crate::anim::now_ms();
    assert!(!a.nav_tip_due(now), "the wait costs no frame");
    assert!(a.nav_tip_due(now + DELAY_MS), "one frame at the deadline");
    // The clock past the wait.
    a.nav_tip.show_at = 0;
    let (_, text) = card(&a.build_frame()).expect("the card");
    assert_eq!(text, crate::disktile::short_path(&dir));
    assert!(
        !a.nav_tip_due(crate::anim::now_ms()),
        "an open card costs no frames"
    );
    // A click on the row still opens the folder there.
    assert!(a.far_press());
    match &a.panes[a.focused].content {
        crate::pane::PaneContent::Far(f) => assert_eq!(f.active_local_dir(), Some(dir.clone())),
        _ => panic!("a Far pane"),
    }
    // Off the row, into the panes: gone.
    a.cursor = (900.0, 300.0);
    assert!(card(&a.build_frame()).is_none(), "off the row");
    crate::farrecent::clear();
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn a_row_drawn_whole_gets_no_card() {
    let _g = crate::app::theme_test_guard();
    crate::farrecent::clear();
    let (base, _) = far_with("fitdir");
    let mut a = nav_app("far");
    a.config.nav_width = 1000.0;
    a.cursor_in = true;
    a.cursor = drawn_at(&mut a, "fitdir");
    a.build_frame();
    a.nav_tip.show_at = 0;
    assert!(card(&a.build_frame()).is_none(), "nothing was cut");
    assert!(
        !a.nav_tip_due(crate::anim::now_ms() + 10 * DELAY_MS),
        "and nothing is owed"
    );
    crate::farrecent::clear();
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn a_key_puts_the_card_away() {
    let _g = crate::app::theme_test_guard();
    crate::farrecent::clear();
    let (base, _) = far_with("keydir");
    let mut a = nav_app("far");
    a.cursor_in = true;
    a.cursor = drawn_at(&mut a, "keydir");
    a.build_frame();
    a.nav_tip.show_at = 0;
    assert!(card(&a.build_frame()).is_some());
    a.last_input_ms = a.nav_tip.since + 1;
    assert!(a.nav_tip_due(crate::anim::now_ms()), "the key's repaint");
    assert!(card(&a.build_frame()).is_none());
    crate::farrecent::clear();
    let _ = std::fs::remove_dir_all(&base);
}

/// The Explorer: a file name longer than the panel shows whole on the card.
#[test]
fn a_cut_file_name_shows_whole() {
    let _g = crate::app::theme_test_guard();
    let root = std::env::temp_dir().join(format!("navtip-tree-{}", std::process::id()));
    let name = "quarterly-planning-notes-for-the-whole-team-final-v2.md";
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join(name), "x").unwrap();
    let mut a = crate::explorerhit::tests::app(&root);
    a.cursor_in = true;
    a.cursor = crate::explorerhit::tests::at(&mut a, "quarterly");
    a.build_frame();
    a.nav_tip.show_at = 0;
    let (_, text) = card_of(&a.build_frame(), "name").expect("the card");
    assert_eq!(text.replace(' ', ""), name.replace(' ', ""));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_path_wraps_at_its_folders() {
    let p = "/var/folders/wm/z5pvj/T/navtip-shot/clients/northwind/quarterly-reports".to_string();
    let rows = crate::navtipdraw::lines(std::slice::from_ref(&p), 30);
    assert!(
        rows.iter().all(|r| crate::chatwidth::str_w(r) <= 30),
        "{rows:?}"
    );
    assert!(
        rows[..rows.len() - 1].iter().all(|r| r.ends_with('/')),
        "{rows:?}"
    );
    assert_eq!(rows.concat(), p);
    let prose = crate::navtipdraw::lines(
        &["Ada Lovelace".into(), "the whole subject line".into()],
        12,
    );
    assert_eq!(prose, ["Ada Lovelace", "the whole", "subject line"]);
}
