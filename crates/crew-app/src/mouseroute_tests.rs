//! A click followed from the pixel under the pointer to the bytes the
//! program in the pane reads: a real pty whose program records its raw stdin.
use winit::event::MouseButton;
use winit::keyboard::ModifiersState;

#[path = "mouserig_tests.rs"]
pub(crate) mod rig;
use rig::*;

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
    let got = read_log(&log, 28);
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

#[test]
fn a_press_on_the_find_bar_row_is_swallowed() {
    let (mut app, log) = app_on_cell("findbar", b"\x1b[?1000h\x1b[?1006h");
    assert!(app.term_find_open());
    // Row 2 is not the bar's (row 0): the program still gets that click.
    assert!(app.report_press(MouseButton::Left));
    assert!(app.mouse.held.is_some());
    app.mouse.held = None;
    app.cursor.1 -= CELL.1 * 2.0;
    assert!(app.report_press(MouseButton::Left), "swallowed on the bar");
    assert!(app.mouse.held.is_none(), "and nothing was reported");
    assert!(app.drag.is_none(), "no selection starts under it");
    let _ = std::fs::remove_file(&log);
}
