use crate::app::CrewApp;
use crate::pane::{Pane, PaneContent};

fn tile() -> Pane {
    Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Todo(crate::todopane::TodoPane::new()),
        grid: crew_term::GridSize { cols: 80, rows: 24 },
        rect: crate::spawn::PLACEHOLDER_RECT,
        label: None,
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: 0,
        zoom: None,
        tab: 0,
    }
}

/// Focus pane `i` the way Cmd+N does, then run the frame's reconcile.
fn focus(app: &mut CrewApp, i: usize) {
    app.focused = i;
    app.input.focused = false;
    app.reconcile_grid();
}

/// Two panes in the first tab, then a second tab holding a third — what
/// Cmd+T does, without forking a shell.
fn two_tabs() -> (CrewApp, u32, u32) {
    let mut app = CrewApp::default();
    app.panes = vec![tile(), tile()];
    focus(&mut app, 1);
    let first = app.tabs.active;
    let second = app.tabs.add();
    app.activate_tab(second);
    app.panes.push(tile());
    focus(&mut app, 2);
    (app, first, second)
}

#[test]
fn a_new_pane_joins_the_tab_on_screen_and_only_its_tab_is_on_the_grid() {
    let (app, first, second) = two_tabs();
    assert_eq!(app.tabs.active, second);
    assert_eq!(app.panes[0].tab, first);
    assert_eq!(app.panes[2].tab, second);
    assert!(app.on_grid(2));
    assert!(
        !app.on_grid(0) && !app.on_grid(1),
        "the other tab's panes are off the grid"
    );
    assert_eq!(app.grid.full(), vec![2]);
}

#[test]
fn coming_back_to_a_tab_lands_on_the_pane_it_was_left_at() {
    let (mut app, first, _) = two_tabs();
    app.step_tab(-1);
    app.reconcile_grid();
    assert_eq!(app.tabs.active, first);
    assert_eq!(app.focused, 1, "pane 2 had the focus when the tab was left");
    let mut full = app.grid.full();
    full.sort();
    assert_eq!(full, vec![0, 1]);
}

#[test]
fn focusing_a_pane_in_another_tab_brings_its_tab_forward() {
    let (mut app, first, _) = two_tabs();
    focus(&mut app, 0); // Cmd+1, or a click on its nav row
    assert_eq!(app.tabs.active, first);
    assert!(app.on_grid(0));
}

#[test]
fn closing_a_tabs_last_pane_closes_the_tab_and_its_neighbour_comes_on() {
    let (mut app, first, _) = two_tabs();
    app.close_pane(2);
    app.reconcile_grid();
    assert_eq!(app.tabs.list.len(), 1);
    assert_eq!(app.tabs.active, first);
    assert!(app.focused < 2 && !app.input.focused);
}

#[test]
fn closing_a_pane_keeps_focus_in_its_own_tab() {
    let (mut app, _, second) = two_tabs();
    app.panes.push(tile());
    focus(&mut app, 3);
    app.close_pane(3);
    app.reconcile_grid();
    assert_eq!(app.tabs.active, second);
    assert_eq!(app.focused, 2);
}

#[test]
fn a_pane_sent_to_another_tab_takes_you_with_it() {
    let (mut app, first, second) = two_tabs();
    app.move_pane_to_tab(1);
    app.reconcile_grid();
    assert_eq!(app.panes[2].tab, first);
    assert_eq!(app.tabs.active, first);
    assert!(app.tabs.pos(second).is_none(), "the emptied tab went");
    // One past the last opens a tab for it.
    app.move_pane_to_tab(2);
    app.reconcile_grid();
    assert_eq!(app.tabs.list.len(), 2);
    assert_eq!(app.tabs.active, app.panes[2].tab);
}

#[test]
fn ctrl_tab_cycles_the_panes_of_this_tab_only() {
    let (mut app, _, _) = two_tabs();
    app.panes.push(tile());
    focus(&mut app, 3);
    app.cycle_tab_panes(1);
    assert_eq!(app.focused, 2, "wrapped within the tab, past panes 1 and 2");
}

#[test]
fn the_nav_draws_a_card_per_tab_with_its_panes_inside() {
    use crate::tabnav::Line;
    let (app, first, second) = two_tabs();
    assert_eq!(
        app.nav_lines(),
        vec![
            Line::Tab(first),
            Line::Pane(0),
            Line::Pane(1),
            Line::Edge(first),
            Line::Tab(second),
            Line::Pane(2),
            Line::Edge(second),
        ]
    );
    // With one tab the list is the flat list it always was.
    assert_eq!(CrewApp::default().nav_lines(), vec![]);
}

#[test]
fn a_tab_is_called_by_its_name_else_its_first_pane() {
    let (mut app, first, _) = two_tabs();
    assert_eq!(app.tab_title(first), app.panes[0].title_text());
    app.tab_command("rename build");
    assert_eq!(app.tab_title(app.tabs.active), "build");
    assert!(app.tab_summary().contains("build*"));
    app.tab_command("rename");
    assert_eq!(app.tabs.name(app.tabs.active), None);
}

#[test]
fn a_session_remembers_which_tab_each_pane_was_in() {
    let (mut app, first, second) = two_tabs();
    app.tabs.get_mut(second).expect("tab").name = Some("web".into());
    let mut sp = crate::sessionsave::SavedPane::crew();
    app.stamp_tab(&mut sp, first);
    assert_eq!((sp.tab, sp.tab_name.as_deref()), (0, None));
    app.stamp_tab(&mut sp, second);
    assert_eq!((sp.tab, sp.tab_name.as_deref()), (1, Some("web")));
    // Restored into a fresh crew: saved tab 0 is the tab on screen, the rest
    // open after it, names and all.
    let mut fresh = CrewApp::default();
    let map = fresh.restore_tabs(&[crate::sessionsave::SavedPane::crew(), sp]);
    assert_eq!(map.len(), 2);
    assert_eq!(map[0].1, fresh.tabs.active);
    assert_eq!(fresh.tabs.name(map[1].1), Some("web"));
}
