//! Which doors open a file ready to type in.
use crate::app::CrewApp;
use crate::pane::PaneContent;
use crate::viewpane::ViewPane;

/// Wait for the viewer `i` to land, through the app's own poll (which is what puts the caret in
/// a pane opened to edit).
fn landed(app: &mut CrewApp, i: usize) -> &mut ViewPane {
    for _ in 0..500 {
        let cols = app.panes[i].grid.cols;
        let PaneContent::View(v) = &mut app.panes[i].content else {
            panic!("a viewer");
        };
        v.poll_landing(cols);
        if !v.loading() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    match &mut app.panes[i].content {
        PaneContent::View(v) => v,
        _ => unreachable!(),
    }
}

/// A file picked to open — Cmd+P's `/view`, the Explorer — arrives ready to type in, like any
/// editor; a viewer crew opens on its own (`/out`, a link) is still a reader.
#[test]
fn a_picked_file_opens_ready_to_type() {
    let dir = std::env::temp_dir().join("viewedit-code-tests");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("picked.rs"), "fn a() {}\n").unwrap();
    let mut app = CrewApp {
        cwd: dir.clone(),
        ..Default::default()
    };
    app.open_to_edit("picked.rs");
    assert!(landed(&mut app, 0).caret.is_some(), "Cmd+P: editing");
    app.explorer_open(dir.join("picked.rs"));
    let last = app.panes.len() - 1;
    assert!(
        landed(&mut app, last).caret.is_some(),
        "the Explorer: editing"
    );
    app.open_view("picked.rs");
    let last = app.panes.len() - 1;
    assert!(
        landed(&mut app, last).caret.is_none(),
        "any other door: reading"
    );
}
