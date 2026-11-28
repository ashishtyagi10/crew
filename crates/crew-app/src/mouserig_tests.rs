//! The rig for [`super`]: an app with one pane whose program records the
//! raw bytes it reads (split out for the line cap).
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::app::CrewApp;
use crate::pane::{Pane, PaneContent, TermPane};
use crew_term::{GridSize, PtyTerm, TermModel};

pub(crate) const CELL: (f32, f32) = (8.0, 17.0);

/// A pane whose program puts its tty in raw mode and copies stdin to `log`.
pub(crate) fn recording_pane(log: &Path) -> Pane {
    let grid = GridSize { cols: 40, rows: 8 };
    let script = format!(
        "stty raw -echo; : > '{0}.ready'; exec cat > '{0}'",
        log.display()
    );
    let args = vec!["-c".to_string(), script];
    let pty = PtyTerm::spawn_in(grid, "/bin/sh", &args, Some(&std::env::temp_dir())).unwrap();
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

/// Polls `what` for up to 5 s; false means it timed out. Callers that need
/// it to come true assert on it — a silent timeout hid a wrong byte count.
pub(crate) fn wait_for(what: impl Fn() -> bool) -> bool {
    let until = Instant::now() + Duration::from_secs(5);
    while !what() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(20));
    }
    what()
}

/// Rows of text as `(row, text)`, for a failure to say what was drawn.
pub(crate) fn rows(cells: impl Iterator<Item = (u16, char)>) -> Vec<(u16, String)> {
    let mut out: Vec<(u16, String)> = Vec::new();
    for (row, c) in cells.filter(|c| c.1 != ' ') {
        match out.iter_mut().find(|r| r.0 == row) {
            Some(r) => r.1.push(c),
            None => out.push((row, c.to_string())),
        }
    }
    out
}

/// An app with one recording pane, the parser told `modes`, and the pointer
/// on content cell (col 4, row 2) — where the built frame drew it.
pub(crate) fn app_on_cell(tag: &str, modes: &[u8]) -> (CrewApp, PathBuf) {
    let log = std::env::temp_dir().join(format!("crew-mouse-{tag}-{}", std::process::id()));
    let ready = PathBuf::from(format!("{}.ready", log.display()));
    let _ = std::fs::remove_file(&log);
    let _ = std::fs::remove_file(&ready);
    let mut app = CrewApp {
        geo_override: Some((CELL.0, CELL.1, 1280.0, 720.0, 1.0)),
        ..Default::default()
    };
    app.panes = vec![recording_pane(&log)];
    app.input.focused = false;
    app.reconcile_grid();
    app.build_frame();
    assert!(
        wait_for(|| ready.exists()),
        "{tag}: the recording shell never came up"
    );
    if let PaneContent::Terminal(t) = &mut app.panes[0].content {
        t.pty.feed(b"\x1b[2J\x1b[H\r\n\r\nMARK");
        t.pty.feed(modes);
    }
    let scenes = app.build_frame();
    let s = scenes
        .iter()
        .find(|s| s.cells.iter().any(|c| c.row == 2 && c.c == 'M'))
        .unwrap_or_else(|| {
            // Once seen under the release gate and never reproduced since
            // (2026-11-16): say what the parser holds against what was drawn.
            let PaneContent::Terminal(t) = &app.panes[0].content else {
                unreachable!()
            };
            let (p, held) = (
                &app.panes[0],
                rows(t.pty.cells(false).iter().map(|c| (c.row, c.c))),
            );
            let drawn: Vec<_> = scenes
                .iter()
                .map(|s| rows(s.cells.iter().map(|c| (c.row, c.c))))
                .collect();
            panic!(
                "{tag}: no MARK on row 2. exited={} grid={:?} rect={:?} hidden={} tab={} \
                motion={:?} density={:?} theme={:?}\nparser: {held:?}\ndrawn: {drawn:?}",
                t.pty.exited(),
                p.grid,
                p.rect,
                p.hidden,
                p.tab,
                crate::motion::level(),
                crate::density::level(),
                crew_theme::current_id()
            )
        });
    app.cursor = (s.x + CELL.0 * 4.5, s.y + CELL.1 * 2.5);
    (app, log)
}

pub(crate) fn read_log(log: &Path, want: usize) -> Vec<u8> {
    let full = wait_for(|| std::fs::metadata(log).is_ok_and(|m| m.len() as usize >= want));
    assert!(full, "the program never read {want} bytes");
    std::thread::sleep(Duration::from_millis(50));
    std::fs::read(log).unwrap_or_default()
}
