//! OSC 22 end to end on a real terminal pane: the program's shape under the
//! pointer, and every one of crew's own targets outranking it.
#![cfg(unix)]
use crate::app::CrewApp;
use crate::pane::{Pane, PaneContent, TermPane};
use crate::pointer::Over;
use crew_term::{GridSize, PtyTerm, TermModel};
use winit::window::CursorIcon;

const CELL: (f32, f32) = (8.0, 17.0);

fn term_pane(dir: Option<std::path::PathBuf>) -> Pane {
    let grid = GridSize { cols: 40, rows: 8 };
    let pty = PtyTerm::spawn_in(grid, "/bin/sh", &[], Some(&std::env::temp_dir())).unwrap();
    let input = pty.writer();
    Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Terminal(Box::new(TermPane {
            pty,
            input,
            cmd: None,
            cmd_since: None,
            tail: Default::default(),
            read_at: 0,
            spans: Default::default(),
            trail: Default::default(),
            find: None,
            images: Default::default(),
        })),
        grid,
        rect: crate::layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: None,
        dir,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: 0,
        zoom: None,
        tab: 0,
    }
}

/// `n` terminal panes laid out on a 1280×720 window, sized by one frame.
fn app_with(panes: Vec<Pane>, zoomed: bool, focused: usize) -> CrewApp {
    let mut app = CrewApp {
        geo_override: Some((CELL.0, CELL.1, 1280.0, 720.0, 1.0)),
        ..Default::default()
    };
    app.panes = panes;
    app.focused = focused;
    app.panes[focused].zoom = Some(zoomed);
    app.input.focused = false;
    app.reconcile_grid();
    // The first frame sizes each shell to its card, which moves what is
    // already on its screen — so text goes in after it.
    app.build_frame();
    app
}

fn feed(app: &mut CrewApp, pane: usize, bytes: &[u8]) {
    if let PaneContent::Terminal(t) = &mut app.panes[pane].content {
        t.pty.feed(bytes);
    }
}

/// The centre of content cell `(row, col)` of the pane whose row shows
/// `marker`, where the built frame actually drew it.
fn drawn_cell(app: &mut CrewApp, marker: &str, (row, col): (u16, u16)) -> (f32, f32) {
    let scenes = app.build_frame();
    let s = scenes
        .iter()
        .find(|s| {
            let text: String = s
                .cells
                .iter()
                .filter(|c| c.row == row)
                .map(|c| c.c)
                .collect();
            text.contains(marker)
        })
        .unwrap_or_else(|| panic!("no scene shows {marker:?}"));
    let (cw, ch) = CELL;
    (
        s.x + cw * (f32::from(col) + 0.5),
        s.y + ch * (f32::from(row) + 0.5),
    )
}

/// Point the pointer at content cell `(row, col)` and sync the hover state.
fn point(app: &mut CrewApp, marker: &str, at: (u16, u16)) -> Over {
    app.cursor = drawn_cell(app, marker, at);
    app.cursor_in = true;
    app.link_hover_sync();
    app.pointer_over()
}

fn setup() -> CrewApp {
    let mut app = app_with(vec![term_pane(None)], false, 0);
    feed(
        &mut app,
        0,
        b"\x1b[2J\x1b[H\r\n  go https://p.example.com/x",
    );
    app
}

#[test]
fn a_programs_shape_is_worn_over_its_cells_until_it_clears_it() {
    let _g = crate::app::theme_test_guard();
    let _h = crate::linkhover::test_guard();
    let mut app = setup();
    assert_eq!(point(&mut app, "p.example", (1, 1)), Over::Text);
    feed(&mut app, 0, b"\x1b]22;crosshair\x07");
    assert_eq!(
        point(&mut app, "p.example", (1, 1)),
        Over::Program(CursorIcon::Crosshair)
    );
    // A name winit does not know keeps crew's own.
    feed(&mut app, 0, b"\x1b]22;banana\x07");
    assert_eq!(point(&mut app, "p.example", (1, 1)), Over::Text);
    feed(&mut app, 0, b"\x1b]22;pointer\x1b\\\x1b]22;\x07");
    assert_eq!(point(&mut app, "p.example", (1, 1)), Over::Text);
}

/// A link under the pointer is crew's: Cmd opens it, so it keeps the hand it
/// promises however the program has asked for the pointer to look.
#[test]
fn a_link_beats_the_programs_shape() {
    let _g = crate::app::theme_test_guard();
    let _h = crate::linkhover::test_guard();
    let mut app = setup();
    feed(&mut app, 0, b"\x1b]22;crosshair\x07");
    assert_eq!(point(&mut app, "p.example", (1, 5)), Over::Link);
    assert_eq!(
        point(&mut app, "p.example", (1, 1)),
        Over::Program(CursorIcon::Crosshair)
    );
}

/// The card's legend row is crew's handle, not the program's cells.
#[test]
fn the_legend_row_beats_the_programs_shape() {
    let _g = crate::app::theme_test_guard();
    let _h = crate::linkhover::test_guard();
    let mut app = setup();
    feed(&mut app, 0, b"\x1b]22;crosshair\x07");
    let on = drawn_cell(&mut app, "p.example", (1, 1));
    app.cursor = (on.0, on.1 - CELL.1 * 3.0);
    app.cursor_in = true;
    assert_eq!(app.cursor_any_cell(), None, "not over a cell");
    assert!(!matches!(app.pointer_over(), Over::Program(_)));
}

/// Crew's own selection gesture keeps the text cursor.
#[test]
fn a_selection_drag_keeps_the_text_cursor() {
    let _g = crate::app::theme_test_guard();
    let _h = crate::linkhover::test_guard();
    let mut app = setup();
    feed(&mut app, 0, b"\x1b]22;crosshair\x07");
    app.cursor = drawn_cell(&mut app, "p.example", (1, 1));
    app.cursor_in = true;
    app.selection_press();
    app.cursor = drawn_cell(&mut app, "p.example", (1, 9));
    app.selection_drag();
    assert!(app.drag.is_some_and(|d| d.moved));
    assert_eq!(app.pointer_over(), Over::Text);
}

/// A program that sets its shape under a motionless pointer: the poll step,
/// not a mouse event, must put the hand on.
#[test]
fn a_shape_set_under_a_still_pointer_is_applied_by_the_poll_refresh() {
    let _g = crate::app::theme_test_guard();
    let _h = crate::linkhover::test_guard();
    let mut app = setup();
    point(&mut app, "p.example", (1, 1));
    app.pointer_sync();
    assert_eq!(app.cursor_icon, CursorIcon::Text);
    feed(&mut app, 0, b"\x1b]22;pointer\x07");
    assert_eq!(app.cursor_icon, CursorIcon::Text, "no mouse event yet");
    app.pointer_refresh();
    assert_eq!(app.cursor_icon, CursorIcon::Pointer);
}
