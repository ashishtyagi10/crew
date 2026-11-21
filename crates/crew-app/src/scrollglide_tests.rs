use super::*;
use crate::motion::{set_level, MotionLevel};
use crate::pane::spawn_pane;
use crew_term::{GridSize, TermModel};

fn shell_with_history() -> Pane {
    let grid = GridSize { cols: 40, rows: 10 };
    let mut p = spawn_pane("sh", "sh", grid, Some(&std::env::temp_dir())).unwrap();
    if let PaneContent::Terminal(t) = &mut p.content {
        t.pty.feed(b"\x1b[H\x1b[2J\x1b[3J");
        t.pty.feed("row\r\n".repeat(200).as_bytes());
        t.pty.scroll_to_bottom();
    }
    p
}

fn offset(p: &Pane) -> usize {
    term_offset(p).unwrap()
}

/// Motion off: the scroll is the jump it always was, and nothing is queued.
#[test]
fn motion_off_jumps() {
    let _g = crate::appearanceguard::theme_test_guard();
    set_level(MotionLevel::Off);
    let mut p = shell_with_history();
    start(&mut p, 30);
    assert_eq!(offset(&p), 30);
    assert!(!p.glide.scroll.moving());
}

/// Motion on: nothing moves until frames are stepped, then the pane gets
/// there; Shift+Home measures the trip and stops at the top, owing nothing.
#[test]
fn a_terminal_glides_to_its_ends_and_stops_there() {
    let _g = crate::appearanceguard::theme_test_guard();
    set_level(MotionLevel::Full);
    let mut p = shell_with_history();
    start(&mut p, 30);
    assert_eq!(offset(&p), 0, "queued, not jumped");
    let mut panes = vec![p];
    while panes[0].glide.scroll.moving() {
        step_all(&mut panes, 16);
    }
    assert_eq!(offset(&panes[0]), 30);
    start_end(&mut panes[0], true);
    for _ in 0..60 {
        step_all(&mut panes, 16);
    }
    let top = offset(&panes[0]);
    assert!(top > 150, "reached the top: {top}");
    assert!(!panes[0].glide.scroll.moving());
    // Past the end: the glide notices nothing moved and gives up at once.
    start(&mut panes[0], 50);
    step_all(&mut panes, 16);
    step_all(&mut panes, 16);
    assert!(!panes[0].glide.scroll.moving(), "stopped at the end");
}

/// Typing snaps a terminal to its live bottom; a glide still heading up must
/// not drag the view back away from what was just typed.
#[test]
fn a_snap_to_the_bottom_cancels_the_glide() {
    let _g = crate::appearanceguard::theme_test_guard();
    set_level(MotionLevel::Full);
    let mut panes = vec![shell_with_history()];
    start(&mut panes[0], 60);
    step_all(&mut panes, 16);
    step_all(&mut panes, 16);
    assert!(offset(&panes[0]) > 0);
    if let PaneContent::Terminal(t) = &mut panes[0].content {
        t.pty.scroll_to_bottom();
    }
    step_all(&mut panes, 16);
    assert_eq!(offset(&panes[0]), 0);
    assert!(!panes[0].glide.scroll.moving());
}
