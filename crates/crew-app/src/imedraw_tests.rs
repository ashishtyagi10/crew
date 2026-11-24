//! A composition reaches the frame at the caret of whatever takes text, and
//! the window is told where that caret is.
use super::bar_caret_col;
use crate::app::CrewApp;
use crate::imepreedit::cell_rect;
use crate::pane::{Pane, PaneContent, TermPane};
use crew_render::PaneScene;
use crew_term::{GridSize, PtyTerm};
use crew_theme::deco::DecoLine;
use winit::event::Ime;

pub(crate) fn shell() -> Pane {
    let grid = GridSize { cols: 40, rows: 8 };
    let pty = PtyTerm::spawn_in(grid, "/bin/cat", &[], Some(&std::env::temp_dir())).unwrap();
    let input = pty.writer();
    let content = PaneContent::Terminal(Box::new(TermPane {
        pty,
        input,
        cmd: None,
        cmd_since: None,
        tail: Default::default(),
        read_at: 0,
        spans: Default::default(),
        trail: Default::default(),
        images: Default::default(),
    }));
    Pane {
        glide: crate::glide::Glide::default(),
        content,
        grid,
        rect: crate::layout::Rect {
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
        zoom: None,
        tab: 0,
    }
}

fn app() -> CrewApp {
    let mut a = CrewApp {
        geo_override: Some((8.0, 17.0, 1280.0, 720.0, 1.0)),
        panes: vec![shell()],
        focused: 0,
        ..Default::default()
    };
    a.input.focused = false;
    a
}

/// The focused shell's content scene: the one focused, unframed scene.
fn shell_scene(scenes: &[PaneScene]) -> &PaneScene {
    scenes.iter().find(|s| s.focused && !s.glass).unwrap()
}

fn row_text(s: &PaneScene, row: u16) -> Vec<(u16, char, DecoLine)> {
    let mut v: Vec<_> = s
        .cells
        .iter()
        .filter(|c| c.row == row && c.c != ' ')
        .map(|c| (c.col, c.c, c.deco.line))
        .collect();
    v.sort_by_key(|c| c.0);
    v
}

/// にほ being composed in a shell is drawn at its cursor, underlined, each
/// glyph two columns wide; committed, the overlay is gone.
#[cfg(unix)]
#[test]
fn a_shell_draws_the_composition_at_its_cursor() {
    let _g = crate::app::theme_test_guard();
    let mut a = app();
    let (col, row) = match &a.panes[0].content {
        PaneContent::Terminal(t) => t.pty.ime_cell().unwrap(),
        _ => unreachable!(),
    };
    a.on_ime(Ime::Preedit("にほ".into(), Some((6, 6))));
    let scenes = a.build_frame();
    let drawn = row_text(shell_scene(&scenes), row);
    assert_eq!(
        drawn,
        [
            (col, 'に', DecoLine::Single),
            (col + 2, 'ほ', DecoLine::Single)
        ]
    );
    a.on_ime(Ime::Commit("日本".into()));
    let scenes = a.build_frame();
    assert!(
        row_text(shell_scene(&scenes), row).is_empty(),
        "the grid's own row"
    );
}

/// While the window takes IME, its cursor area is the shell's cursor cell,
/// read off the same scene the cell is drawn in.
#[cfg(unix)]
#[test]
fn the_cursor_area_is_the_shells_cursor_cell() {
    let _g = crate::app::theme_test_guard();
    let mut a = app();
    a.ime.on = Some(true);
    let scenes = a.build_frame();
    let s = shell_scene(&scenes);
    let at = match &a.panes[0].content {
        PaneContent::Terminal(t) => t.pty.ime_cell().unwrap(),
        _ => unreachable!(),
    };
    assert_eq!(a.ime.area, Some(cell_rect((s.x, s.y), at, 8.0, 17.0)));
    assert!(
        s.x > 0.0 && s.y > 0.0,
        "inside the window, not at its origin"
    );
}

/// In the input bar the composition replaces the caret, after the text.
#[cfg(unix)]
#[test]
fn the_input_bar_draws_it_after_the_text() {
    let _g = crate::app::theme_test_guard();
    let mut a = app();
    a.input.focused = true;
    a.input.text = "ab".into();
    a.on_ime(Ime::Preedit("日".into(), None));
    let scenes = a.build_frame();
    let bar = scenes.iter().find(|s| s.cells.iter().any(|c| c.c == '日'));
    let cell = bar.unwrap().cells.iter().find(|c| c.c == '日').unwrap();
    assert_eq!(cell.col, bar_caret_col("ab", 100));
    assert_eq!(cell.col, 6);
    assert_eq!(cell.deco.line, DecoLine::Single);
    assert!(
        !bar.unwrap()
            .cells
            .iter()
            .any(|c| c.c == crate::inputbar_render::CARET),
        "the composition stands where the caret was"
    );
}

/// The bar's caret column as `bar_caret_col` reads it is where the bar draws
/// its caret: short text, a line scrolled to follow it, and one of wide glyphs.
#[test]
fn the_bar_caret_column_is_where_the_caret_is_drawn() {
    let _g = crate::app::theme_test_guard();
    let mut a = app();
    a.input.focused = true;
    for text in [
        "ab".to_string(),
        "x".repeat(400),
        "日".repeat(200),
        "a日".repeat(150),
    ] {
        a.input.text = text.clone();
        let scenes = a.build_frame();
        let bar = scenes
            .iter()
            .find(|s| s.cells.iter().any(|c| c.c == crate::inputbar_render::CARET))
            .unwrap();
        let caret = bar
            .cells
            .iter()
            .find(|c| c.c == crate::inputbar_render::CARET);
        let cols = (bar.w / 8.0).floor() as u16;
        assert_eq!(
            caret.unwrap().col,
            bar_caret_col(&text, cols),
            "{}",
            text.len()
        );
    }
}

#[cfg(unix)]
#[path = "imeshot_tests.rs"]
mod shot;
