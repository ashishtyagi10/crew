//! Editing a markdown document in a viewer pane in the grid.
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::app::CrewApp;
use crate::pane::PaneContent;
use crate::viewpane::ViewPane;

/// An app holding one viewer pane on `text`, written to a file of its own, loaded and focused.
fn app_with(name: &str, text: &str) -> (CrewApp, std::path::PathBuf) {
    let dir = std::env::temp_dir().join("viewedit-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join(name);
    std::fs::write(&f, text).unwrap();
    let mut app = CrewApp {
        cwd: dir,
        ..Default::default()
    };
    app.open_view(name);
    for _ in 0..500 {
        let v = view(&mut app);
        v.poll();
        if !v.loading() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    app.focused = 0;
    app.input.focused = false;
    (app, f)
}

fn view(app: &mut CrewApp) -> &mut ViewPane {
    match &mut app.panes[0].content {
        PaneContent::View(v) => v,
        _ => panic!("a viewer pane"),
    }
}

fn key(app: &mut CrewApp, k: Key, mods: ModifiersState) -> bool {
    app.view_edit_input(&k, true, mods)
}

fn ch(s: &str) -> Key {
    Key::Character(s.into())
}

fn start(app: &mut CrewApp) {
    let (cols, rows) = (app.panes[0].grid.cols, app.panes[0].grid.rows);
    let input = crate::viewpane::keys::ViewInput::Insert;
    let act = crate::viewpane::keys::apply(view(app), input, cols, rows);
    assert!(matches!(act, Some(crate::viewpane::ViewAction::Editing)));
}

/// A reader until asked: no caret, and a letter is not the editor's.
#[test]
fn a_viewer_pane_is_a_reader_until_i() {
    let (mut app, _) = app_with("reader.md", "# Title\n\nbody\n");
    assert!(view(&mut app).caret.is_none());
    assert!(!key(&mut app, ch("x"), ModifiersState::empty()));
    start(&mut app);
    assert!(view(&mut app).caret.is_some());
}

/// Typing types; Cmd+Z is undo rather than the grid's zoom; Cmd+S writes the file.
#[test]
fn typing_undo_and_save_belong_to_the_editing_pane() {
    let (mut app, f) = app_with("typing.md", "# Title\n\nbody\n");
    start(&mut app);
    assert!(key(&mut app, ch("X"), ModifiersState::empty()));
    assert!(view(&mut app).dirty);
    assert!(key(&mut app, ch("z"), ModifiersState::SUPER));
    assert_eq!(view(&mut app).source_str(), Some("# Title\n\nbody\n"));
    assert!(key(&mut app, ch("Y"), ModifiersState::empty()));
    assert!(key(&mut app, ch("s"), ModifiersState::SUPER));
    let on_disk = std::fs::read_to_string(&f).unwrap();
    assert!(
        on_disk.contains('Y') && !view(&mut app).dirty,
        "{on_disk:?}"
    );
}

/// Cmd+arrows still walk the grid, even while a document is being edited.
#[test]
fn cmd_arrows_are_left_to_the_grid() {
    let (mut app, _) = app_with("arrows.md", "body\n");
    start(&mut app);
    assert!(!key(
        &mut app,
        Key::Named(NamedKey::ArrowLeft),
        ModifiersState::SUPER
    ));
    assert!(key(
        &mut app,
        Key::Named(NamedKey::ArrowLeft),
        ModifiersState::empty()
    ));
}

/// Esc goes back to reading and keeps the edits; closing then asks once before dropping them.
#[test]
fn unsaved_edits_survive_esc_and_the_first_close() {
    let (mut app, f) = app_with("close.md", "body\n");
    start(&mut app);
    key(&mut app, ch("Q"), ModifiersState::empty());
    assert!(key(
        &mut app,
        Key::Named(NamedKey::Escape),
        ModifiersState::empty()
    ));
    assert!(view(&mut app).caret.is_none() && view(&mut app).dirty);
    app.close_pane(0);
    assert_eq!(app.panes.len(), 1, "the first close is refused");
    assert!(view(&mut app).warned);
    app.close_pane(0);
    assert!(app.panes.is_empty(), "the second one closes");
    assert_eq!(
        std::fs::read_to_string(&f).unwrap(),
        "body\n",
        "and nothing was written"
    );
}

/// A re-read would throw the edits away: the first Cmd+R says so instead.
#[test]
fn a_reread_of_unsaved_edits_asks_first() {
    let (mut app, _) = app_with("reread.md", "body\n");
    start(&mut app);
    key(&mut app, ch("Q"), ModifiersState::empty());
    assert!(key(&mut app, ch("r"), ModifiersState::SUPER));
    assert!(
        view(&mut app).dirty && view(&mut app).warned,
        "refused once"
    );
    assert!(key(&mut app, ch("r"), ModifiersState::SUPER));
    assert!(!view(&mut app).dirty, "the second one re-reads");
}

/// Panes re-tile whenever one opens, closes or zooms. The caret's row and column belong to the
/// old width; only its byte survives, and the pane must find it again.
#[test]
fn a_pane_re_tiled_mid_edit_keeps_the_caret_on_its_byte() {
    let long = "word ".repeat(60);
    let (mut app, _) = app_with("retile.md", &format!("{long}\n\nend\n"));
    let rect = |w: f32| crate::layout::Rect {
        x: 0.0,
        y: 0.0,
        w,
        h: 400.0,
    };
    crate::panefit::relayout_one(&mut app.panes[0], rect(800.0), 10.0, 20.0);
    start(&mut app);
    for _ in 0..30 {
        key(
            &mut app,
            Key::Named(NamedKey::ArrowRight),
            ModifiersState::empty(),
        );
    }
    crate::panefit::relayout_one(&mut app.panes[0], rect(300.0), 10.0, 20.0);
    key(
        &mut app,
        Key::Named(NamedKey::ArrowRight),
        ModifiersState::empty(),
    );
    let cols = app.panes[0].grid.cols;
    let v = view(&mut app);
    let at = crate::viewpane::caret::offset_at(&v.lines_for(cols).lines, v.caret.unwrap());
    assert_eq!(
        at, v.caret_at,
        "the drawn caret and its byte agree after the re-tile"
    );
    assert_eq!(
        v.caret_at,
        Some(31),
        "and it moved one on from where it was"
    );
}

/// `/close all` used to loop until the grid was empty — with a pane refusing to close, forever.
/// A bulk close waits for the edits instead, and says whose they are.
#[test]
fn closing_every_pane_waits_for_unsaved_edits() {
    let (mut app, _) = app_with("bulk.md", "body\n");
    start(&mut app);
    key(&mut app, ch("Q"), ModifiersState::empty());
    app.close_all_panes();
    app.close_all_panes(); // the "again" that confirms a close-all
    assert_eq!(app.panes.len(), 1, "the edited pane is still there");
    app.close_other_panes();
    assert_eq!(app.panes.len(), 1, "and alone, it is no one's 'other'");
}
