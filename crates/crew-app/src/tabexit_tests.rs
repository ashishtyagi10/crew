//! The tab being left folds away under the one arriving.
use crate::app::CrewApp;
use crate::ghost::Exit;

use super::tests::tile;

/// Switching down the list leaves one departing ghost per visible card of the old tab, drifting
/// up; switching to the tab already on screen leaves none.
#[test]
fn switching_tabs_leaves_the_old_cards_departing() {
    let _g = crate::app::motion_test_guard();
    let mut app = CrewApp {
        panes: vec![tile(), tile()],
        ..Default::default()
    };
    app.reconcile_grid();
    let first = app.tabs.active;
    let second = app.tabs.add();
    app.activate_tab(second);
    let exits: Vec<Exit> = app.ghosts.iter().map(|g| g.exit).collect();
    assert_eq!(exits, [Exit::Departed(1), Exit::Departed(1)]);
    app.ghosts.clear();
    app.activate_tab(second);
    assert!(app.ghosts.is_empty(), "no switch, nothing leaves");
    app.activate_tab(first);
    assert!(
        app.ghosts.is_empty(),
        "the second tab had no cards on screen"
    );
}
