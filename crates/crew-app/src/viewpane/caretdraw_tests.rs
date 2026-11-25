//! The caret is drawn wherever it can stand — including the stop after a
//! line's last character, which has no rendered cell of its own and is where
//! the caret sits while you type at the end of a line.
use crate::app::CrewApp;
use crate::pane::PaneContent;
use crate::viewpane::caret::Step;

fn walk(name: &str, body: &str) {
    let dir = std::env::temp_dir().join("viewedit-caretshow-tests");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(name), body).unwrap();
    let mut app = CrewApp {
        cwd: dir.clone(),
        ..Default::default()
    };
    app.open_to_edit(name);
    for _ in 0..500 {
        let cols = app.panes[0].grid.cols;
        let PaneContent::View(v) = &mut app.panes[0].content else {
            panic!("a viewer");
        };
        v.poll_landing(cols);
        if !v.loading() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let (cols, rows) = (app.panes[0].grid.cols, app.panes[0].grid.rows);
    let PaneContent::View(v) = &mut app.panes[0].content else {
        unreachable!()
    };
    for step in 0..body.len() + 2 {
        let c = v.caret.expect("opened ready to type");
        let lit: Vec<_> = v
            .cells(cols, rows)
            .into_iter()
            .filter(|c| c.cursor.is_rule())
            .map(|c| (c.row as usize, c.col))
            .collect();
        assert_eq!(
            lit,
            vec![(c.row - v.scroll, c.col)],
            "{name} step {step}: the caret at {c:?} must be drawn once, where it is"
        );
        v.move_caret(Step::Right, cols, rows);
    }
}

#[test]
fn the_caret_is_drawn_at_every_stop_including_line_ends() {
    walk("a.rs", "fn a() {}\n\nlet x = 1;\n");
    walk("a.md", "# T\n\nab\n\ncd\n");
    walk("a.txt", "ab\n\ncd\n");
}

/// Lines as wide as the pane and wider: the stop after a wrapped row's last
/// character must still be drawn, not lost past the right edge.
#[test]
fn the_caret_is_drawn_on_wide_and_wrapped_lines() {
    for n in [78, 79, 80, 81, 150] {
        let line = "x".repeat(n);
        walk("w.txt", &format!("{line}\nab\n"));
        walk("w.rs", &format!("// {line}\nfn a() {{}}\n"));
    }
}
