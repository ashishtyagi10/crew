//! Addressing across windows. A pane nobody can name is a pane nobody can ask, so these are
//! about the roster and the routing rather than about the asking itself (which is `askpump`'s).
use super::*;
use crate::app::CrewApp;
use crate::farpane::FarPane;
use crate::ipc_types::PaneCard;
use crate::layout::Rect;
use crate::pane::{Pane, PaneContent};
use crew_term::GridSize;

fn pane(name: &str) -> Pane {
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
        name: (!name.is_empty()).then(|| name.to_string()),
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: 0,
    }
}

/// Two windows, the second holding a pane called `schema`.
fn two_windows() -> Crew {
    let mut first = CrewApp::default();
    first.panes.push(pane("shell"));
    let mut crew = Crew::new(first);
    let mut second = CrewApp::default();
    second.panes.push(pane("schema"));
    crew.canvases.push(second);
    crew
}

fn roster(crew: &mut Crew) -> Vec<PaneCard> {
    let (tx, rx) = std::sync::mpsc::channel();
    crew.route_request(Request::Panes { v: 1 }, tx, 0);
    match rx.try_recv() {
        Ok(Reply::Roster { panes }) => panes,
        other => panic!("expected a roster, got {other:?}"),
    }
}

/// Which canvas would answer an ask for `to`.
fn canvas_of(crew: &Crew, to: &str) -> Option<usize> {
    crew.canvas_for(&Request::Ask {
        v: 1,
        from: "tester".into(),
        to: to.into(),
        question: "?".into(),
        id: "a1".into(),
    })
}

#[test]
fn a_second_windows_panes_are_addressable_by_id() {
    assert_eq!(crate::panes_roster::pane_id(0, 3), "p3");
    assert_eq!(
        crate::panes_roster::pane_id(1, 0),
        "w1p0",
        "the first window keeps the spelling every script already knows"
    );
}

#[test]
fn a_window_prefix_is_read_off_an_address() {
    use crate::askroute::split_window;
    assert_eq!(split_window("p0"), (None, "p0"));
    assert_eq!(split_window("w1p0"), (Some(1), "p0"));
    assert_eq!(split_window("w12p3"), (Some(12), "p3"));
}

#[test]
fn a_pane_name_that_starts_with_w_is_not_a_window_prefix() {
    // `worker`, `web`, and a pane somebody actually called `w1p0` — a prefix rule that eats
    // names makes those panes unreachable.
    use crate::askroute::split_window;
    assert_eq!(split_window("worker"), (None, "worker"));
    assert_eq!(split_window("web"), (None, "web"));
    assert_eq!(split_window("w1"), (None, "w1"));
    assert_eq!(split_window("w1pane"), (None, "w1pane"));
}

#[test]
fn the_roster_lists_every_window_with_the_id_that_reaches_it() {
    // The bug this closes: `crew panes` listed the launch window only, so a pane in the second
    // one could not be discovered, named, or asked.
    let mut crew = two_windows();
    let ids: Vec<String> = roster(&mut crew).into_iter().map(|c| c.id).collect();
    assert_eq!(ids, ["p0", "w1p0"]);
}

#[test]
fn an_ask_reaches_the_window_its_address_names() {
    let crew = two_windows();
    assert_eq!(canvas_of(&crew, "p0"), Some(0));
    assert_eq!(canvas_of(&crew, "w1p0"), Some(1));
}

#[test]
fn a_pane_addressed_by_name_is_found_in_whichever_window_holds_it() {
    // A name belongs to the pane, not to the window it happens to be in, so `crew ask schema`
    // must keep working when somebody moves it to a second window.
    let crew = two_windows();
    assert_eq!(canvas_of(&crew, "schema"), Some(1));
    assert_eq!(canvas_of(&crew, "shell"), Some(0));
}

#[test]
fn an_address_that_names_nothing_reaches_no_window() {
    let crew = two_windows();
    assert_eq!(canvas_of(&crew, "nobody"), None);
    assert_eq!(canvas_of(&crew, "w9p0"), None, "no ninth window");
    assert_eq!(canvas_of(&crew, "w1p7"), None, "no eighth pane in it");
}
