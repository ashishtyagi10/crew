//! A click followed from the pixel under the pointer to the bytes the
//! program in the pane reads: a real pty whose program records its raw stdin.
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use winit::event::MouseButton;
use winit::keyboard::ModifiersState;

use crate::app::CrewApp;
use crate::pane::{Pane, PaneContent, TermPane};
use crew_term::{GridSize, PtyTerm, TermModel};

const CELL: (f32, f32) = (8.0, 17.0);

/// A pane whose program puts its tty in raw mode and copies stdin to `log`.
fn recording_pane(log: &Path) -> Pane {
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

fn wait_for(what: impl Fn() -> bool) {
    let until = Instant::now() + Duration::from_secs(5);
    while !what() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// An app with one recording pane, the parser told `modes`, and the pointer
/// on content cell (col 4, row 2) — where the built frame drew it.
fn app_on_cell(tag: &str, modes: &[u8]) -> (CrewApp, PathBuf) {
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
    wait_for(|| ready.exists());
    if let PaneContent::Terminal(t) = &mut app.panes[0].content {
        t.pty.feed(b"\x1b[2J\x1b[H\r\n\r\nMARK");
        t.pty.feed(modes);
    }
    let scenes = app.build_frame();
    let s = scenes
        .iter()
        .find(|s| s.cells.iter().any(|c| c.row == 2 && c.c == 'M'))
        .expect("the pane's content scene");
    app.cursor = (s.x + CELL.0 * 4.5, s.y + CELL.1 * 2.5);
    (app, log)
}

fn read_log(log: &Path, want: usize) -> Vec<u8> {
    wait_for(|| std::fs::metadata(log).is_ok_and(|m| m.len() as usize >= want));
    std::thread::sleep(Duration::from_millis(50));
    std::fs::read(log).unwrap_or_default()
}

#[test]
fn a_click_reaches_a_program_that_asked_for_the_mouse() {
    let (mut app, log) = app_on_cell("sgr", b"\x1b[?1000h\x1b[?1006h");
    assert!(app.report_press(MouseButton::Left));
    assert!(app.drag.is_none(), "no crew selection armed under it");
    assert!(app.report_release(MouseButton::Left));
    let got = read_log(&log, 18);
    let _ = std::fs::remove_file(&log);
    assert_eq!(String::from_utf8_lossy(&got), "\x1b[<0;5;3M\x1b[<0;5;3m");
}

#[test]
fn shift_or_a_program_without_the_mouse_keeps_crews_click() {
    let (mut app, _) = app_on_cell("shift", b"\x1b[?1000h\x1b[?1006h");
    app.mods = ModifiersState::SHIFT.into();
    assert!(!app.report_press(MouseButton::Left));
    assert!(!app.report_release(MouseButton::Left));
    let (mut app, _) = app_on_cell("plain", b"");
    assert!(!app.report_press(MouseButton::Left));
    assert!(
        !app.report_press(MouseButton::Right),
        "right-click still pastes"
    );
}

#[test]
fn a_drag_reaches_a_program_that_asked_for_drags() {
    let (mut app, log) = app_on_cell("drag", b"\x1b[?1002h\x1b[?1006h");
    assert!(app.report_press(MouseButton::Left));
    assert!(
        !app.report_motion(),
        "a move inside the pressed cell is not one"
    );
    app.cursor.0 += CELL.0;
    assert!(app.report_motion());
    assert!(!app.report_motion(), "once per cell crossed, not per pixel");
    assert!(app.report_release(MouseButton::Left));
    let got = read_log(&log, 30);
    let _ = std::fs::remove_file(&log);
    assert_eq!(
        String::from_utf8_lossy(&got),
        "\x1b[<0;5;3M\x1b[<32;6;3M\x1b[<0;6;3m"
    );
}

#[test]
fn a_bare_move_reaches_only_a_program_that_asked_for_every_move() {
    let (mut app, log) = app_on_cell("hover", b"\x1b[?1003h\x1b[?1006h");
    app.cursor.0 += CELL.0;
    assert!(app.report_motion());
    let got = read_log(&log, 10);
    let _ = std::fs::remove_file(&log);
    assert_eq!(String::from_utf8_lossy(&got), "\x1b[<35;6;3M");
    // Clicks only (1000), or drags only (1002) with no button down: silent.
    for (tag, modes) in [("click", &b"\x1b[?1000h"[..]), ("drag0", b"\x1b[?1002h")] {
        let (mut app, _) = app_on_cell(tag, modes);
        app.cursor.0 += CELL.0;
        assert!(!app.report_motion(), "{tag}");
    }
    // Shift keeps the move crew's.
    let (mut app, _) = app_on_cell("hshift", b"\x1b[?1003h");
    app.mods = ModifiersState::SHIFT.into();
    app.cursor.0 += CELL.0;
    assert!(!app.report_motion());
}
