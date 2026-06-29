use super::*;
use winit::dpi::PhysicalPosition;
use winit::event::MouseScrollDelta;

fn px(y: f64) -> MouseScrollDelta {
    MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, y))
}

// A single small trackpad delta (under one line) must NOT be rounded away to
// nothing and lost: it emits 0 lines now but the fraction is retained, so it
// adds up over subsequent ticks. This is the bug — `(p.y / 24.0).round()`
// dropped every sub-12px tick, so slow trackpad scrolling never moved a pane.
#[test]
fn small_trackpad_ticks_accumulate_instead_of_being_lost() {
    let mut app = CrewApp::default();
    // 6px < one 24px line: scrolls nothing yet, but is not discarded.
    assert_eq!(app.wheel_lines(px(6.0)), 0);
    // Three more 6px ticks reach 24px total -> exactly one whole line.
    let total: i32 = (0..3).map(|_| app.wheel_lines(px(6.0))).sum();
    assert_eq!(total, 1);
}

#[test]
fn fast_flick_scrolls_multiple_lines_at_once() {
    let mut app = CrewApp::default();
    // A 60px flick = 2.5 lines -> 2 now, 0.5 carried.
    assert_eq!(app.wheel_lines(px(60.0)), 2);
    // A 12px follow-up crosses the next boundary (0.5 + 0.5 = 1.0).
    assert_eq!(app.wheel_lines(px(12.0)), 1);
}

#[test]
fn negative_deltas_accumulate_symmetrically() {
    let mut app = CrewApp::default();
    assert_eq!(app.wheel_lines(px(-6.0)), 0);
    let total: i32 = (0..3).map(|_| app.wheel_lines(px(-6.0))).sum();
    assert_eq!(total, -1);
}

// Traditional mouse-wheel notches arrive as whole-line deltas and must still
// scroll one line per notch, unchanged.
#[test]
fn line_delta_notches_still_scroll_whole_lines() {
    let mut app = CrewApp::default();
    assert_eq!(app.wheel_lines(MouseScrollDelta::LineDelta(0.0, 1.0)), 1);
    assert_eq!(app.wheel_lines(MouseScrollDelta::LineDelta(0.0, -1.0)), -1);
}
