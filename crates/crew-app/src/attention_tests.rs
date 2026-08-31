use super::*;
use crate::farpane::FarPane;
use crate::layout::Rect;
use crate::pane::{Pane, PaneContent};
use crew_term::GridSize;

fn pane() -> Pane {
    Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Far(FarPane::new(std::env::temp_dir())),
        grid: GridSize { cols: 80, rows: 24 },
        rect: Rect {
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
        born_ms: crate::anim::now_ms(),
    }
}

#[test]
fn glyph_names_the_event() {
    let a = |kind| Attention { kind, at_ms: 0 };
    assert_eq!(a(NotifyKind::Bell).glyph(), '!');
    assert_eq!(a(NotifyKind::Pattern).glyph(), '⚑');
    assert_eq!(a(NotifyKind::AgentDone).glyph(), '✓');
}

#[test]
fn pulses_then_settles() {
    let a = Attention {
        kind: NotifyKind::Bell,
        at_ms: 1000,
    };
    assert!(a.pulsing(1000));
    assert!(a.pulsing(1000 + PULSE_MS - 1));
    assert!(!a.pulsing(1000 + PULSE_MS));
}

#[test]
fn blinks_during_the_pulse_and_holds_steady_after() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let a = Attention {
        kind: NotifyKind::Bell,
        at_ms: 0,
    };
    assert!(a.visible(0), "fresh marker starts visible");
    assert!(!a.visible(BLINK_MS), "off phase");
    assert!(a.visible(2 * BLINK_MS), "on phase again");
    assert!(a.visible(PULSE_MS), "steady once the pulse ends");
    assert!(a.visible(PULSE_MS + BLINK_MS), "…and stays steady");
}

#[test]
fn raise_overwrites_with_the_newest_event() {
    let mut p = pane();
    raise(&mut p, NotifyKind::Pattern, 100);
    raise(&mut p, NotifyKind::Bell, 200);
    assert_eq!(
        p.attention,
        Some(Attention {
            kind: NotifyKind::Bell,
            at_ms: 200
        })
    );
}

#[test]
fn any_pulsing_only_while_a_marker_is_fresh() {
    let mut a = pane();
    let b = pane();
    assert!(!any_pulsing(&[], 0));
    raise(&mut a, NotifyKind::Bell, 1000);
    let panes = [a, b];
    assert!(any_pulsing(&panes, 1000 + PULSE_MS - 1));
    assert!(!any_pulsing(&panes, 1000 + PULSE_MS));
}

/// Reduce motion is an accessibility request, and blinking is the kind of
/// motion it is most specifically about. With motion off the marker is
/// simply *there*, from the first frame — it loses nothing, because what
/// it has to say is that it exists — and it asks for no frames to say it.
#[test]
fn a_marker_holds_still_when_motion_is_off() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Off);
    let a = Attention {
        kind: NotifyKind::Bell,
        at_ms: 0,
    };
    for t in [0, BLINK_MS, 2 * BLINK_MS, PULSE_MS] {
        assert!(a.visible(t), "off phase at {t} would have been a blink");
    }
    assert!(!a.pulsing(0), "and it costs no redraws");
    assert!(!any_pulsing(&[], 0));

    // …and with motion on it still blinks, so the test above is not just
    // agreeing with the default.
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    assert!(!a.visible(BLINK_MS), "off phase");
    assert!(a.pulsing(0));
}

/// The busy-pane spinner holds one frame too — one derivation for every
/// spinner in the app, so a new one cannot be added that ignores it.
#[test]
fn the_spinner_holds_one_frame_when_motion_is_off() {
    let _g = crate::app::motion_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Off);
    let frames: std::collections::HashSet<char> = (0..20)
        .map(|i| crate::update::spinner_frame(i * 100))
        .collect();
    assert_eq!(frames.len(), 1, "{frames:?}");
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let spun: std::collections::HashSet<char> = (0..20)
        .map(|i| crate::update::spinner_frame(i * 100))
        .collect();
    assert!(
        spun.len() > 5,
        "and it really does spin otherwise: {spun:?}"
    );
}
