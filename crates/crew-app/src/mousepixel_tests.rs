//! SGR-Pixels (DECSET 1016) from the pointer to the bytes a real program
//! reads: the rig's cell is 8 x 17 px, the pointer 36.25 / 42.5 px into the
//! text area (col 4, row 2), so the 1-based pixel is (37, 43).
use winit::event::MouseButton;

use crate::mouseroute::tests::rig::*;

const PRESS: &str = "\x1b[<0;37;43M";
const RELEASE: &str = "\x1b[<0;37;43m";

fn text_area(app: &crate::app::CrewApp) -> (f32, f32) {
    let g = app.panes[0].grid;
    (f32::from(g.cols) * CELL.0, f32::from(g.rows) * CELL.1)
}

#[test]
fn a_click_is_reported_in_pixels_from_the_text_origin_beating_1006() {
    let (mut app, log) = app_on_cell("px-click", b"\x1b[?1000h\x1b[?1006h\x1b[?1016h");
    app.cursor.0 += 0.25; // off the whole-pixel boundary: float noise can't flip the floor
    assert!(app.cursor.0 > 40.0, "the origin is offset from the window");
    assert!(app.report_press(MouseButton::Left));
    assert!(app.report_release(MouseButton::Left));
    let got = read_log(&log, PRESS.len() * 2);
    let _ = std::fs::remove_file(&log);
    assert_eq!(String::from_utf8_lossy(&got), format!("{PRESS}{RELEASE}"));
}

#[test]
fn x10_alone_with_1016_still_gets_pixels_and_a_reset_goes_back_to_cells() {
    let (mut app, log) = app_on_cell("px-x10", b"\x1b[?1000h\x1b[?1016h");
    app.cursor.0 += 0.25;
    assert!(app.report_press(MouseButton::Left));
    let got = read_log(&log, PRESS.len());
    assert_eq!(String::from_utf8_lossy(&got), PRESS);
    let _ = std::fs::remove_file(&log);
    let (mut app, log) = app_on_cell("px-off", b"\x1b[?1000h\x1b[?1006h\x1b[?1016h\x1b[?1016l");
    assert!(app.report_press(MouseButton::Left));
    let got = read_log(&log, 9);
    let _ = std::fs::remove_file(&log);
    assert_eq!(String::from_utf8_lossy(&got), "\x1b[<0;5;3M");
}

#[test]
fn without_a_mouse_mode_1016_alone_reports_nothing() {
    let (mut app, _) = app_on_cell("px-alone", b"\x1b[?1016h");
    assert!(!app.report_press(MouseButton::Left));
}

#[test]
fn motion_is_heard_per_pixel_not_per_cell() {
    let (mut app, log) = app_on_cell("px-move", b"\x1b[?1003h\x1b[?1016h");
    app.cursor.0 += 0.25;
    assert!(app.report_motion());
    app.cursor.0 += 1.0; // one pixel, same cell
    assert!(app.report_motion());
    assert!(!app.report_motion(), "no move, no report");
    let want = "\x1b[<35;37;43M\x1b[<35;38;43M";
    let got = read_log(&log, want.len());
    let _ = std::fs::remove_file(&log);
    assert_eq!(String::from_utf8_lossy(&got), want);
}

#[test]
fn a_drag_off_the_pane_releases_clamped_to_the_text_area() {
    let (mut app, log) = app_on_cell("px-clamp", b"\x1b[?1002h\x1b[?1016h");
    app.cursor.0 += 0.25;
    assert!(app.report_press(MouseButton::Left));
    let (w, h) = text_area(&app);
    app.cursor = (app.cursor.0 + 5000.0, app.cursor.1 + 5000.0);
    assert!(app.report_release(MouseButton::Left));
    let want = format!("{PRESS}\x1b[<0;{};{}m", w as u32, h as u32);
    let got = read_log(&log, want.len());
    let _ = std::fs::remove_file(&log);
    assert_eq!(String::from_utf8_lossy(&got), want);
    // And the other way: left of and above the origin pins to pixel 1.
    let (mut app, log) = app_on_cell("px-low", b"\x1b[?1002h\x1b[?1016h");
    assert!(app.report_press(MouseButton::Left));
    app.cursor = (-500.0, -500.0);
    assert!(app.report_release(MouseButton::Left));
    let want = "\x1b[<0;37;43M\x1b[<0;1;1m";
    let got = read_log(&log, want.len());
    let _ = std::fs::remove_file(&log);
    assert_eq!(String::from_utf8_lossy(&got), want);
}

#[test]
fn the_wheel_is_reported_in_pixels_too() {
    let (mut app, log) = app_on_cell("px-wheel", b"\x1b[?1000h\x1b[?1006h\x1b[?1016h");
    app.cursor.0 += 0.25;
    app.scroll_at_cursor(1, true);
    app.scroll_at_cursor(-1, true);
    let want = "\x1b[<64;37;43M\x1b[<65;37;43M";
    let got = read_log(&log, want.len());
    let _ = std::fs::remove_file(&log);
    assert_eq!(String::from_utf8_lossy(&got), want);
}

#[test]
fn pixels_are_in_the_cell_size_the_pty_reports_not_the_window_scale() {
    let (mut app, log) = app_on_cell("px-scale", b"\x1b[?1000h\x1b[?1016h");
    app.cursor.0 += 0.25;
    // The pty is told 16 x 34 px cells while the drawn cell is 8 x 17: the
    // same pointer is twice as many reported pixels, the same cell.
    if let crate::pane::PaneContent::Terminal(t) = &mut app.panes[0].content {
        t.pty.set_cell_px(16, 34);
    }
    assert!(app.report_press(MouseButton::Left));
    let want = "\x1b[<0;73;86M"; // floor(72.5) + 1, floor(85) + 1
    let got = read_log(&log, want.len());
    let _ = std::fs::remove_file(&log);
    assert_eq!(String::from_utf8_lossy(&got), want);
}
