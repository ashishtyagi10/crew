use crate::app::CrewApp;
use crate::inbox::{InboxPane, Stage};
use crate::pane::{Pane, PaneContent};

fn pane(content: PaneContent) -> Pane {
    Pane {
        glide: crate::glide::Glide::default(),
        content,
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
    }
}

fn tile() -> Pane {
    pane(PaneContent::Todo(crate::todopane::TodoPane::new()))
}

fn far() -> Pane {
    pane(PaneContent::Far(crate::farpane::FarPane::new(
        std::env::temp_dir(),
    )))
}

fn inbox() -> Pane {
    let p = InboxPane::with(Stage::Setup { found: None });
    pane(PaneContent::Inbox(Box::new(p)))
}

/// `panes` open, `focused` focused, and the grid as the next frame sees it.
fn app(panes: Vec<Pane>, focused: usize) -> CrewApp {
    let mut app = CrewApp::default();
    app.panes = panes;
    app.focus(focused);
    app
}

impl CrewApp {
    /// Focus pane `i` the way Cmd+N does, then run the frame's reconcile.
    fn focus(&mut self, i: usize) {
        self.focused = i;
        self.input.focused = false;
        self.reconcile_grid();
    }

    fn said(&self) -> String {
        self.status.clone().map(|(m, _)| m).unwrap_or_default()
    }
}

#[test]
fn the_inbox_far_settings_and_viewer_are_zoomed_and_the_rest_are_tiles() {
    assert!(inbox().zoomed() && far().zoomed());
    assert!(!tile().zoomed(), "the todo list is a tile");
    let mut docked = far();
    docked.zoom = Some(false);
    assert!(!docked.zoomed(), "Cmd+Z outranks the kind");
}

/// The bug this replaced: zoom was the APP's, so far's zoom passed to the
/// next pane focused, and far came back as a tile.
#[test]
fn zoom_belongs_to_the_pane_not_to_whatever_has_focus() {
    let mut app = app(vec![tile(), tile(), far()], 2);
    assert_eq!(app.zoomed_pane(), Some(2), "far opens on its own screen");
    app.focus(1);
    assert_eq!(app.zoomed_pane(), None, "a tile shows the grid");
    app.focus(2);
    assert_eq!(app.zoomed_pane(), Some(2), "and far gets the screen back");
}

#[test]
fn a_zoomed_pane_never_takes_a_tile() {
    let app = app(vec![tile(), inbox(), tile(), far()], 0);
    let mut on: Vec<usize> = app.grid.recent().to_vec();
    on.sort_unstable();
    assert_eq!(on, vec![0, 2]);
}

#[test]
fn cmd_z_zooms_a_tile_until_cmd_z_again_and_docks_the_inbox() {
    let mut app = app(vec![tile(), tile(), inbox()], 0);
    app.toggle_zoom();
    app.focus(1);
    assert!(!app.zoomed(), "leaving a zoomed tile shows the grid");
    app.focus(0);
    assert_eq!(app.zoomed_pane(), Some(0), "the zoom waited with its pane");
    app.toggle_zoom();
    assert!(!app.zoomed());

    app.focus(2);
    app.toggle_zoom();
    app.reconcile_grid();
    assert!(!app.zoomed(), "Cmd+Z on the inbox docks it");
    assert!(app.grid.recent().contains(&2), "…onto the grid");
}

#[test]
fn the_screen_step_walks_the_grid_then_each_zoomed_pane_and_wraps() {
    let mut app = app(vec![tile(), far(), tile(), inbox()], 2);
    app.step_screen(1);
    assert_eq!(app.zoomed_pane(), Some(1), "grid → far");
    app.step_screen(1);
    assert_eq!(app.zoomed_pane(), Some(3), "far → inbox");
    app.step_screen(1);
    assert_eq!(app.focused, 2, "inbox → the grid, on the tile used last");
    app.step_screen(-1);
    assert_eq!(app.zoomed_pane(), Some(3), "and back the other way");
}

#[test]
fn the_screen_step_with_one_screen_says_so_and_stays() {
    let mut app = app(vec![tile(), tile()], 1);
    app.step_screen(1);
    assert_eq!(app.focused, 1);
    assert!(app.said().contains("one screen open"), "{}", app.said());
}

#[test]
fn esc_on_the_inbox_goes_back_to_the_grid_and_keeps_the_mail() {
    let mut app = app(vec![tile(), tile(), inbox()], 1);
    app.focus(2);
    app.put_away(2);
    assert_eq!(app.focused, 1, "the tile you came from");
    assert!(app.panes.len() == 3 && !app.panes[2].hidden, "still open");
    assert!(app.said().contains("still open"), "{}", app.said());
}

#[test]
fn esc_on_the_inbox_with_no_grid_puts_it_in_the_nav() {
    let mut app = app(vec![inbox()], 0);
    app.put_away(0);
    assert!(app.panes[0].hidden && app.input.focused);
}

#[test]
fn closing_the_pane_on_screen_lands_on_the_grid_you_left() {
    let mut app = app(vec![tile(), tile(), tile(), far(), inbox()], 0);
    app.focus(3);
    app.close_pane(3);
    assert_eq!(app.focused, 0, "the last tile, not the inbox beside far");
    assert!(!app.zoomed());
}

#[test]
fn minimizing_the_pane_on_screen_lands_on_the_grid_you_left() {
    let mut app = app(vec![tile(), tile(), far(), inbox()], 0);
    app.focus(2);
    app.minimize_pane(2);
    assert_eq!(app.focused, 0);
}

#[test]
fn a_waiting_agent_badges_but_never_takes_the_screen_from_the_inbox() {
    let mut chat = crate::chat::tests::pane();
    chat.plan_pending = true;
    let mut app = app(vec![inbox(), pane(PaneContent::Chat(chat))], 0);
    (app.panes[0].born_ms, app.panes[1].born_ms) = (1, 2);
    app.last_input_ms = 0; // hands-off since forever
    app.tick_blocked(100_000);
    assert_eq!(app.focused, 0, "still reading");
    assert!(app.panes[1].attention.is_some(), "…with the badge up");
}

#[test]
fn the_screen_chord_is_cmd_opt_on_a_mac_and_ctrl_alt_off_it() {
    use crate::chordglyph::{mac, offmac};
    use crate::ctrlchord::{grid_held, screen_held};
    use winit::keyboard::ModifiersState as M;
    assert!(screen_held(M::SUPER | M::ALT, true));
    assert!(!screen_held(M::CONTROL | M::ALT, true), "Spaces");
    assert!(screen_held(M::CONTROL | M::ALT, false));
    assert!(!grid_held(M::CONTROL | M::ALT, false), "never both");
    let chord = "Cmd+Opt+\u{2190}\u{2192}";
    assert_eq!(offmac(chord), "Ctrl+Alt+\u{2190}\u{2192}");
    assert_eq!(mac(chord), "\u{2325}\u{2318}\u{2190}\u{2192}");
}
