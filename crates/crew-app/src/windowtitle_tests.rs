//! The window is called what the focused pane is called.
use crate::app::CrewApp;
use crate::farpane::FarPane;
use crate::layout::Rect;
use crate::pane::{Pane, PaneContent};
use crew_term::GridSize;

fn far(name: Option<&str>) -> Pane {
    Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Far(FarPane::new(std::env::temp_dir())),
        grid: GridSize { cols: 80, rows: 24 },
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: name.map(str::to_string),
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

/// agent smith's window read `Chat — Crew`; a `/name`d pane is called that.
#[test]
fn a_named_pane_names_the_window() {
    let mut app = CrewApp::default();
    app.panes.push(far(Some("agent smith")));
    assert_eq!(app.focused_title(), "agent smith \u{2014} Crew");
    app.panes[0].name = None;
    assert_eq!(
        app.focused_title(),
        "Far \u{2014} Crew",
        "unnamed keeps its kind"
    );
}

/// With the nav hidden nothing said which tab you were in; with two tabs
/// the window names the active one and where it sits, one tab stays plain.
#[test]
fn more_than_one_tab_names_the_tab() {
    let mut app = CrewApp::default();
    app.panes.push(far(Some("agent smith")));
    app.panes[0].tab = app.tabs.active;
    assert_eq!(app.focused_title(), "agent smith \u{2014} Crew", "one tab");
    let web = app.tabs.add();
    app.tabs.get_mut(web).unwrap().name = Some("web".into());
    app.panes[0].tab = web;
    app.tabs.active = web;
    assert_eq!(
        app.focused_title(),
        "agent smith \u{2014} web (2/2) \u{2014} Crew"
    );
    // An unnamed tab is called after its first pane: not said twice.
    app.tabs.get_mut(web).unwrap().name = None;
    assert_eq!(
        app.focused_title(),
        "agent smith \u{2014} (2/2) \u{2014} Crew"
    );
}
