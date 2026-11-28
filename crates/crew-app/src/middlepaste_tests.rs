//! The middle click's text: the focused pane's selection, else any pane's.
use crate::app::CrewApp;
use crate::gridsel::CellSel;

/// A Far pane: one that draws text to select without a pty.
fn far() -> crate::pane::Pane {
    use crate::pane::{Pane, PaneContent};
    Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Far(crate::farpane::FarPane::new(std::env::temp_dir())),
        grid: crew_term::GridSize { cols: 80, rows: 24 },
        rect: crate::layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: Some("far".into()),
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: crate::anim::now_ms(),
        zoom: None,
        tab: 0,
    }
}

fn sel(pane: usize) -> Option<CellSel> {
    Some(CellSel {
        pane,
        anchor: (0, 0),
        cursor: (30, 2),
    })
}

#[test]
fn the_selection_in_any_pane_is_what_a_middle_click_pastes() {
    let _g = crate::app::theme_test_guard();
    let mut app = CrewApp::default();
    assert_eq!(app.selection_anywhere(), None, "no panes, nothing selected");
    app.panes.push(far());
    app.panes.push(far());
    app.focused = 0;
    assert_eq!(app.selection_anywhere(), None);
    app.cell_sel = sel(1);
    let want = app.pane_selection_text(1);
    assert!(want.is_some(), "the far pane draws text to select");
    assert_eq!(app.selection_anywhere(), want, "another pane's selection");
    app.input.focused = true;
    app.paste_text(want.clone().unwrap());
    assert!(!app.input.text.is_empty(), "the bar took it");
}
