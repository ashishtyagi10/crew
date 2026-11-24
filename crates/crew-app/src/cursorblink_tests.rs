// The app-level tests run `/bin/cat` in their panes: Windows builds the helpers and runs none.
#![cfg_attr(not(unix), allow(dead_code, unused_imports))]
use super::*;
use crew_theme::deco::CursorShape;

const T0: u64 = 10_000;

/// Right after the caret moves or a key lands it is lit, for the whole solid
/// stretch — typing never catches it in its dark half.
#[test]
fn the_caret_is_solid_right_after_activity() {
    for dt in [0, 1, SOLID_MS / 2, SOLID_MS - 1] {
        assert!(visible(T0 + dt, T0, true, true), "+{dt}");
    }
    assert_eq!(next_toggle(T0, T0, true, true), Some(T0 + SOLID_MS));
    assert_eq!(next_toggle(T0 + 100, T0, true, true), Some(T0 + SOLID_MS));
}

/// Past the solid stretch it goes dark first, then lit, then dark — each half
/// [`PERIOD_MS`] long, and `next_toggle` names the edge `visible` flips at.
#[test]
fn past_the_solid_stretch_it_alternates_dark_and_lit() {
    let s = T0 + SOLID_MS;
    for (at, lit) in [
        (s, false),
        (s + PERIOD_MS - 1, false),
        (s + PERIOD_MS, true),
        (s + 2 * PERIOD_MS - 1, true),
        (s + 2 * PERIOD_MS, false),
        (s + 7 * PERIOD_MS + 3, true),
    ] {
        assert_eq!(visible(at, T0, true, true), lit, "at +{}", at - T0);
        let next = next_toggle(at, T0, true, true).unwrap();
        assert!(next > at, "the next toggle is in the future");
        assert_eq!(
            visible(next - 1, T0, true, true),
            lit,
            "same phase until it"
        );
        assert_eq!(visible(next, T0, true, true), !lit, "flips exactly at it");
    }
}

/// Motion off, or a steady shape: always drawn, and never a frame asked for.
#[test]
fn motion_off_and_steady_shapes_never_blink_or_ask_for_frames() {
    for at in [T0, T0 + SOLID_MS, T0 + SOLID_MS + PERIOD_MS / 2, T0 + 9_999] {
        assert!(visible(at, T0, true, false), "motion off");
        assert!(visible(at, T0, false, true), "steady shape");
        assert_eq!(next_toggle(at, T0, true, false), None);
        assert_eq!(next_toggle(at, T0, false, true), None);
    }
}

fn blinking_shell() -> Pane {
    let mut p = crate::glassworkshot_tests::shell();
    if let PaneContent::Terminal(t) = &mut p.content {
        crew_term::TermModel::feed(&mut t.pty, b"\x1b[5 q");
    }
    p
}

fn cursor_of(p: &Pane, focused: bool) -> CursorShape {
    p.cells(focused)
        .iter()
        .map(|c| c.cursor.shape)
        .find(|s| *s != CursorShape::None)
        .unwrap_or(CursorShape::None)
}

/// `vim`'s insert-mode bar, in two panes: the focused one's caret is there
/// in the solid stretch and the lit half and gone in the dark half; the
/// other pane's outline is there throughout.
#[cfg(unix)]
#[test]
fn only_the_focused_panes_caret_blinks() {
    let mut panes = vec![blinking_shell(), blinking_shell()];
    let s = T0 + SOLID_MS;
    for (now, lit) in [(T0, true), (s + 10, false), (s + PERIOD_MS + 10, true)] {
        step(&mut panes, Some(0), 0, true, now);
        let want = if lit {
            CursorShape::Beam
        } else {
            CursorShape::None
        };
        assert_eq!(cursor_of(&panes[0], true), want, "+{}", now - T0);
        assert_eq!(cursor_of(&panes[1], false), CursorShape::Hollow);
        // Even asked as if focused, the unfocused pane was never darkened.
        assert_eq!(cursor_of(&panes[1], true), CursorShape::Beam);
    }
    // Motion off: lit at the very phase that was dark.
    step(&mut panes, Some(0), 0, false, s + 10);
    assert_eq!(cursor_of(&panes[0], true), CursorShape::Beam);
}

/// A key restarts the solid stretch; and a toggle owes exactly one repaint,
/// at its instant — nothing before, nothing again after.
#[cfg(unix)]
#[test]
fn a_key_relights_it_and_each_toggle_is_owed_one_frame() {
    let mut panes = vec![blinking_shell()];
    step(&mut panes, Some(0), 0, true, T0);
    assert!(!take_due(&mut panes, T0 + SOLID_MS - 1));
    assert!(take_due(&mut panes, T0 + SOLID_MS));
    assert!(!take_due(&mut panes, T0 + SOLID_MS + 1), "owed once");
    let key = T0 + SOLID_MS + 50;
    step(&mut panes, Some(0), key, true, key + 10);
    assert_eq!(
        cursor_of(&panes[0], true),
        CursorShape::Beam,
        "a key relights it"
    );
    // A steady shape (vim back in normal mode) is never owed a frame.
    if let PaneContent::Terminal(t) = &mut panes[0].content {
        crew_term::TermModel::feed(&mut t.pty, b"\x1b[2 q");
    }
    step(&mut panes, Some(0), 0, true, T0 + 99_999);
    assert!(!take_due(&mut panes, u64::MAX));
}

/// Through the real frame: the focused pane's caret leaves the drawn scenes
/// in its dark half and the other pane's outline stays.
#[cfg(unix)]
#[test]
fn the_drawn_frame_drops_only_the_focused_caret_in_the_dark_half() {
    let _g = crate::app::theme_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let mut a = crate::app::CrewApp {
        geo_override: Some((8.0, 17.0, 1280.0, 720.0, 1.0)),
        panes: vec![blinking_shell(), blinking_shell()],
        focused: 0,
        ..Default::default()
    };
    a.input.focused = false;
    let carets = |a: &mut crate::app::CrewApp| -> Vec<CursorShape> {
        let scenes = a.build_frame();
        let cells = scenes.iter().flat_map(|s| s.cells.iter());
        cells
            .map(|c| c.cursor.shape)
            .filter(|s| *s != CursorShape::None)
            .collect()
    };
    let lit = carets(&mut a);
    assert!(
        lit.contains(&CursorShape::Beam),
        "solid on the first frame: {lit:?}"
    );
    if let PaneContent::Terminal(t) = &mut a.panes[0].content {
        // The app clock starts with the process: wait until it is old
        // enough to have a past this far back.
        let back = SOLID_MS + 100;
        let now = crate::anim::now_ms();
        std::thread::sleep(std::time::Duration::from_millis(back.saturating_sub(now)));
        t.trail.blink.since = crate::anim::now_ms() - back;
    }
    assert_eq!(carets(&mut a), vec![CursorShape::Hollow], "dark half");
}
