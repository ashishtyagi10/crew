//! A shell's text stands as far off its frame as a chat's does: a column
//! further in than the cell past the stroke, with its pty a column narrower
//! (glass survey C#6). `linkclick_tests` holds clicks to the cell drawn.
use crate::app::CrewApp;
use crate::pane::{Pane, PaneContent, TermPane};
use crew_term::{GridSize, PtyTerm};

const CELL: (f32, f32) = (8.0, 17.0);

fn term_pane() -> Pane {
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
    }
}

#[test]
fn a_shell_starts_a_column_further_in_and_is_a_column_narrower() {
    let mut app = CrewApp {
        geo_override: Some((CELL.0, CELL.1, 1280.0, 720.0, 1.0)),
        ..Default::default()
    };
    app.panes = vec![term_pane()];
    app.input.focused = false;
    app.reconcile_grid();
    let scenes = app.build_frame();
    let p = &app.panes[0];
    let (inner, _) = crate::layout::card_inner_cells(p.rect.w, p.rect.h, CELL.0, CELL.1);
    assert_eq!(p.grid.cols, inner - 1, "the pty gives the column back");
    let content = scenes
        .iter()
        .find(|s| !s.bordered && !s.overlay && s.y == p.rect.y + CELL.1)
        .expect("the shell's content scene");
    assert_eq!(
        content.x,
        p.rect.x + 2.0 * CELL.0,
        "a cell and a half off the stroke"
    );
}
