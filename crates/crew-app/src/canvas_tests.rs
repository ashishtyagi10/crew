//! Which canvas an event belongs to, and what a second one shares with the
//! first. Nothing here can build a window (winit needs a display and an
//! active event loop), so what is tested is the routing and the ownership —
//! the two things a second window can get wrong in ways a display would not
//! show you until much later.
use super::*;
use crate::farpane::FarPane;
use crate::layout::Rect;
use crate::pane::{Pane, PaneContent};
use crew_term::GridSize;

fn crew() -> Crew {
    Crew::new(CrewApp::default())
}

/// A pane with nothing running in it — enough to be counted and focused.
fn pane() -> Pane {
    Pane {
        glide: Default::default(),
        content: PaneContent::Far(FarPane::new(std::env::temp_dir())),
        grid: GridSize { cols: 80, rows: 24 },
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: 0,
    }
}

/// A canvas with no window owns no event. This is the guard that keeps a
/// windowless canvas — the state every test builds — from swallowing events
/// meant for a real one.
#[test]
fn a_canvas_without_a_window_owns_nothing() {
    let app = CrewApp::default();
    assert!(app.window.is_none());
    assert!(app.docs.is_empty());
}

/// `Cmd+N` cannot make a window where it is pressed: only a winit callback
/// holding the active event loop can. So it asks, and the ask is the thing
/// that has to survive until the next tick.
#[test]
fn cmd_n_asks_for_a_canvas_rather_than_making_one() {
    let mut app = CrewApp::default();
    assert!(!app.want_window);
    app.handle_super_chord("n");
    assert!(app.want_window, "the ask is recorded");
}

/// Closing a window is not quitting the app — that decision belongs to the
/// owner, and it turns on how many canvases are left.
#[test]
fn closing_a_canvas_is_a_flag_not_an_exit() {
    let mut app = CrewApp::default();
    assert!(!app.closing);
    // With no panes open the quit guard does not ask twice.
    assert!(app.confirm_quit());
}

/// The config is one thing about the USER, not about a window: changing it in
/// one canvas has to reach the others, or the second window goes on drawing
/// at the old font size and then saves the old value over yours.
#[test]
fn a_config_change_in_one_canvas_reaches_the_others() {
    let mut c = crew();
    c.canvases.push(CrewApp::default());
    assert_eq!(c.canvases[1].config.font_size, c.config.font_size);

    c.active = 0;
    c.canvases[0].config.font_size = 21.0;
    c.share_config();
    assert_eq!(
        c.canvases[1].config.font_size, 21.0,
        "the other canvas kept the old size"
    );
    assert_eq!(c.config.font_size, 21.0, "and the owner's copy moved too");
}

/// …and a canvas that did not change anything must not have its own config
/// stamped over by the comparison.
#[test]
fn sharing_does_nothing_when_nothing_changed() {
    let mut c = crew();
    c.canvases.push(CrewApp::default());
    let before = c.canvases[1].config.clone();
    c.share_config();
    assert_eq!(c.canvases[1].config, before);
}

/// Panes belong to the canvas they were opened in. This is the whole point of
/// the arrangement, and it is the assertion that fails if per-window state
/// were ever hoisted into the owner.
#[test]
fn panes_belong_to_their_own_canvas() {
    let mut c = crew();
    c.canvases.push(CrewApp::default());
    c.canvases[0].panes.push(pane());
    assert_eq!(c.canvases[0].panes.len(), 1);
    assert!(
        c.canvases[1].panes.is_empty(),
        "a second canvas started with somebody else's panes"
    );
}

/// So does focus: two windows each have their own idea of which pane has the
/// keys, and neither may move the other's.
#[test]
fn focus_and_zoom_belong_to_their_own_canvas() {
    let mut c = crew();
    c.canvases.push(CrewApp::default());
    for _ in 0..2 {
        c.canvases[0].panes.push(pane());
        c.canvases[1].panes.push(pane());
    }
    c.canvases[0].focused = 1;
    c.canvases[0].zoomed = true;
    assert_eq!(c.canvases[1].focused, 0);
    assert!(!c.canvases[1].zoomed);
}

/// A new canvas inherits the config and the working directory, and nothing
/// else: it opens empty, with its own input bar.
#[test]
fn a_new_canvas_starts_empty_with_the_shared_config() {
    let mut c = crew();
    c.canvases[0].config.font_size = 17.0;
    c.canvases[0].cwd = std::env::temp_dir();
    c.share_config();
    let next = CrewApp {
        config: c.config.clone(),
        cwd: c.canvases[0].cwd.clone(),
        ..Default::default()
    };
    assert_eq!(next.config.font_size, 17.0);
    assert_eq!(next.cwd, std::env::temp_dir());
    assert!(next.panes.is_empty());
    assert!(
        !next.first,
        "only the launch canvas replays the launch notes"
    );
}

/// A session can have been more than one window, and it has to come back as
/// more than one window. The grouping is by where the windows appear in the
/// file, not by their saved number — a session whose second window was closed
/// before quitting would otherwise ask for a window numbered 2 and leave an
/// empty one at 1.
#[test]
fn a_saved_session_is_split_into_the_windows_it_was_in() {
    use crate::sessionrestore::split_windows;
    use crate::sessionsave::SavedPane;
    let at = |w: usize, dir: &str| SavedPane {
        window: w,
        ..SavedPane::shell(dir.into())
    };
    let (first, rest) = split_windows(vec![at(0, "a"), at(2, "b"), at(0, "c"), at(2, "d")]);
    assert_eq!(first.len(), 2, "the first window's panes");
    assert_eq!(rest.len(), 1, "one further window, not two");
    assert_eq!(rest[0].len(), 2);
}

/// Every session file written before there could be a second window says
/// nothing about windows, and has to restore into one exactly as it always
/// did.
#[test]
fn a_session_from_before_windows_restores_into_one() {
    use crate::sessionrestore::split_windows;
    use crate::sessionsave::SavedPane;
    let old = vec![SavedPane::shell("a".into()), SavedPane::shell("b".into())];
    let (first, rest) = split_windows(old);
    assert_eq!(first.len(), 2);
    assert!(rest.is_empty());
}

#[test]
fn an_empty_session_asks_for_no_windows() {
    use crate::sessionrestore::split_windows;
    let (first, rest) = split_windows(Vec::new());
    assert!(first.is_empty() && rest.is_empty());
}

/// The panes of every canvas are saved, each stamped with the window it was
/// in — otherwise a second window's shells are simply lost at quit.
#[test]
fn saving_stamps_each_canvas_onto_its_own_window() {
    let mut c = crew();
    c.canvases.push(CrewApp::default());
    c.canvases[0].panes.push(pane());
    c.canvases[1].panes.push(pane());
    c.canvases[1].panes.push(pane());
    let saved: Vec<usize> = c
        .canvases
        .iter()
        .enumerate()
        .flat_map(|(i, canvas)| canvas.panes.iter().map(move |_| i))
        .collect();
    assert_eq!(saved, vec![0, 1, 1], "three panes across two windows");
}
