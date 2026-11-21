use super::*;
use crate::motion::{set_level, MotionLevel};
use crate::pane::spawn_pane;
use crew_term::{GridSize, TermModel};

/// Every frame's rows until the glide stops, at 60 fps.
fn run(g: &mut ScrollGlide) -> Vec<i32> {
    let mut steps = Vec::new();
    while g.moving() {
        steps.push(g.step(16));
        assert!(steps.len() < 120, "a scroll glide must settle: {steps:?}");
    }
    steps
}

/// A page travels — over several frames, never backwards, never more than
/// asked — and arrives exactly, in about the third of a second it promises.
#[test]
fn a_page_travels_and_lands_exactly() {
    let mut g = ScrollGlide::default();
    g.push(40);
    let steps = run(&mut g);
    assert_eq!(steps.iter().sum::<i32>(), 40, "{steps:?}");
    assert!(steps.iter().all(|&d| d >= 0), "never backwards: {steps:?}");
    assert!(
        steps.iter().filter(|&&d| d != 0).count() > 4,
        "a glide, not a jump: {steps:?}"
    );
    assert!(steps.len() * 16 <= 400, "too slow: {} ms", steps.len() * 16);
    // It starts gently: the first frame is a few rows, not most of the page.
    assert!(steps[0] < 20, "{steps:?}");
}

/// Down is the mirror of up.
#[test]
fn down_mirrors_up() {
    let mut up = ScrollGlide::default();
    up.push(25);
    let mut down = ScrollGlide::default();
    down.push(-25);
    let neg: Vec<i32> = run(&mut down).iter().map(|d| -d).collect();
    assert_eq!(run(&mut up), neg);
}

/// A second notch mid-flight adds to the trip and keeps the speed: the sum is
/// both notches, and the motion never pauses or reverses at the seam.
#[test]
fn a_second_notch_mid_flight_extends_the_same_motion() {
    let mut g = ScrollGlide::default();
    g.push(3);
    let first = g.step(16);
    g.push(3);
    let rest = run(&mut g);
    assert_eq!(first + rest.iter().sum::<i32>(), 6);
    assert!(rest.iter().all(|&d| d >= 0), "{rest:?}");
    // And the reverse notch turns it around rather than queueing behind it.
    let mut g = ScrollGlide::default();
    g.push(9);
    let went = g.step(16) + g.step(16);
    g.push(-9);
    let back: i32 = run(&mut g).iter().sum();
    assert_eq!(went + back, 0);
}

/// Stopping drops what is owed, so the redraw registry goes quiet.
#[test]
fn stop_owes_nothing() {
    let mut g = ScrollGlide::default();
    g.push(12);
    assert!(g.moving());
    g.stop();
    assert!(!g.moving());
    assert_eq!(g.step(16), 0);
}

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
