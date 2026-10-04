//! A Cmd+click followed end to end: from the pixel the pointer is on, through
//! the cell the frame drew there, to what opens. `OPENED` records the system
//! opener's targets, so no browser opens on the machine running these.
#![cfg(unix)]
use super::{logical_line, OPENED};
use crate::app::CrewApp;
use crate::pane::{Pane, PaneContent, TermPane};
use crew_term::{GridSize, PtyTerm, TermModel};

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
    app.zoomed = zoomed;
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

fn cmd_click(app: &mut CrewApp, at: (f32, f32)) -> (bool, Vec<String>) {
    OPENED.with(|o| o.borrow_mut().clear());
    app.cursor = at;
    let acted = app.cmd_click_at_cursor();
    (acted, OPENED.with(|o| o.borrow().clone()))
}

/// The cell a click resolves is the cell it landed on. Content is drawn one
/// cell in from the card's left edge and one row down from its top, and the
/// hit-test only took the row off — every click read the character to the
/// right of the one under the pointer. In every layout: one to four panes,
/// side by side and zoomed.
#[test]
fn every_pane_resolves_the_cell_it_was_drawn_on() {
    for n in 1..=4 {
        for zoomed in [false, true] {
            for focus in 0..n {
                let mut app = app_with((0..n).map(|_| term_pane(None)).collect(), zoomed, focus);
                for k in 0..n {
                    feed(
                        &mut app,
                        k,
                        format!("\x1b[2J\x1b[H\r\n  go https://p{k}.example.com/x").as_bytes(),
                    );
                }
                let shown: Vec<usize> = if zoomed {
                    vec![focus]
                } else {
                    (0..n).collect()
                };
                for k in shown {
                    let marker = format!("p{k}.example.com");
                    let case = format!("{n} panes, zoomed {zoomed}, focus {focus}, pane {k}");
                    // Column 5 is the URL's `h`; column 4 the space before it.
                    let on = drawn_cell(&mut app, &marker, (1, 5));
                    assert_eq!(
                        cmd_click(&mut app, on).1,
                        [format!("https://{marker}/x")],
                        "{case}"
                    );
                    let off = drawn_cell(&mut app, &marker, (1, 4));
                    assert_eq!(cmd_click(&mut app, off).1, Vec::<String>::new(), "{case}");
                }
            }
        }
    }
}

/// A URL longer than the pane is written across rows; a click on either
/// half opens the whole of it.
#[test]
fn a_wrapped_url_opens_whole_from_either_row() {
    let mut app = app_with(vec![term_pane(None)], false, 0);
    let cols = usize::from(app.panes[0].grid.cols);
    let url = format!("https://example.com/{}", "a".repeat(cols));
    feed(&mut app, 0, format!("\x1b[2J\x1b[H{url}").as_bytes());
    for (row, col) in [(0, 3), (1, 2)] {
        let at = drawn_cell(&mut app, "aaa", (row, col));
        assert_eq!(
            cmd_click(&mut app, at),
            (true, vec![url.clone()]),
            "row {row}"
        );
    }
}

/// Claude Code writes `Update(src/main.rs)`, relative to the directory it
/// runs in: the click opens that file from the pane's directory, not crew's.
#[test]
fn a_path_opens_from_the_directory_of_the_pane_that_printed_it() {
    let dir = std::env::temp_dir().join(format!("crew-linkclick-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    let f = dir.join("sub/note.md");
    std::fs::write(&f, "# note\n").unwrap();
    let mut app = app_with(vec![term_pane(Some(dir.clone()))], false, 0);
    app.cwd = std::env::temp_dir();
    feed(&mut app, 0, b"\x1b[2J\x1b[H* Update(sub/note.md)");
    let at = drawn_cell(&mut app, "Update(", (0, 12));
    assert_eq!(
        cmd_click(&mut app, at),
        (true, vec![]),
        "a path opens here, not in a browser"
    );
    assert!(
        app.panes
            .iter()
            .any(|p| matches!(&p.content, PaneContent::View(v) if v.path == f)),
        "the viewer opened {f:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_logical_line_joins_the_rows_that_wrap() {
    let rows: Vec<Vec<char>> = ["ab", "cd", "ef", "gh"]
        .map(|r| r.chars().collect())
        .to_vec();
    // Rows 1 and 2 wrap onto the next: 1-2-3 is one line.
    let wraps = [false, true, true, false];
    let text =
        |(line, at, first): (Vec<char>, usize, usize)| (line.into_iter().collect(), at, first);
    let joined: (String, _, _) = text(logical_line(&rows, &wraps, 2, 1).unwrap());
    assert_eq!(joined, ("cdefgh".into(), 3, 1));
    let alone: (String, _, _) = text(logical_line(&rows, &wraps, 0, 0).unwrap());
    assert_eq!(alone, ("ab".into(), 0, 0));
    assert!(logical_line(&rows, &wraps, 4, 0).is_none());
}
