use super::*;

const SURFACE: (f32, f32) = (1000.0, 800.0);

/// A card in the top-left quadrant, and one in the bottom-right.
fn top_left() -> Rect {
    Rect {
        x: 100.0,
        y: 80.0,
        w: 300.0,
        h: 240.0,
    }
}
fn bottom_right() -> Rect {
    Rect {
        x: 600.0,
        y: 480.0,
        w: 300.0,
        h: 240.0,
    }
}

/// Run frames until the glide settles, or give up. Returns the frame count so
/// a test can assert the motion is bounded — an animation that never settles
/// repaints an idle crew forever, which is the one thing the render loop is
/// built to avoid.
fn settle(f: &mut WashFocus, target: Option<Rect>) -> u32 {
    for n in 1..=600 {
        f.step(target, SURFACE, 16, MotionLevel::Full);
        if !f.moving() {
            return n;
        }
    }
    panic!("glide never settled");
}

/// An app that has never focused anything draws the wash crew always drew:
/// centred, unmoved. Every headless shot test and every first frame is this.
#[test]
fn the_resting_state_is_the_page_centre() {
    let f = WashFocus::default();
    assert_eq!(f.uniform(), ((0.5, 0.5), PULL_NONE));
    assert!(!f.moving(), "a resting focus must not ask for frames");
}

/// The point of the feature: the light ends up under the focused card, on the
/// same side of the page.
#[test]
fn the_light_gathers_under_the_focused_card() {
    let mut f = WashFocus::default();
    settle(&mut f, Some(top_left()));
    let ((x, y), pull) = f.uniform();
    assert_eq!(pull, PULL);
    // The card's centre: (250, 200) of 1000x800.
    assert!(
        (x - 0.25).abs() < 1e-3 && (y - 0.25).abs() < 1e-3,
        "{x} {y}"
    );

    settle(&mut f, Some(bottom_right()));
    let ((x, y), _) = f.uniform();
    assert!(
        (x - 0.75).abs() < 1e-3 && (y - 0.75).abs() < 1e-3,
        "{x} {y}"
    );
}

/// It GLIDES: a focus change must not land in one frame, or the page-wide
/// field of light reads as a cut. And it must settle, in a time a person
/// would call "a moment" rather than "eventually".
#[test]
fn the_light_travels_rather_than_cutting() {
    let mut f = WashFocus::default();
    settle(&mut f, Some(top_left()));
    f.step(Some(bottom_right()), SURFACE, 16, MotionLevel::Full);
    let ((x, _), _) = f.uniform();
    assert!(
        x > 0.25 && x < 0.35,
        "one frame should be a step, not the whole trip: {x}"
    );
    assert!(f.moving(), "a live glide must keep asking for frames");
    let frames = settle(&mut f, Some(bottom_right()));
    assert!(
        (10..=90).contains(&frames),
        "settling took {frames} frames at 16ms — not a moment"
    );
}

/// Motion off is a genuine off: the final state, drawn once, with nothing
/// rescheduled.
#[test]
fn motion_off_snaps_and_schedules_nothing() {
    let mut f = WashFocus::default();
    f.step(Some(bottom_right()), SURFACE, 16, MotionLevel::Off);
    assert_eq!(f.uniform(), ((0.75, 0.75), PULL));
    assert!(!f.moving());
}

/// Nothing focused fades the gather out instead of dragging the light back
/// across the page: the centre is irrelevant once the pull is zero, and
/// moving both at once would swing a bright field over every pane on its way
/// to standing still.
#[test]
fn losing_focus_dims_the_gather_where_it_stands() {
    let mut f = WashFocus::default();
    settle(&mut f, Some(bottom_right()));
    let mut min_x = 1.0f32;
    for _ in 0..200 {
        f.step(None, SURFACE, 16, MotionLevel::Full);
        let ((x, _), _) = f.uniform();
        min_x = min_x.min(x);
        if !f.moving() {
            break;
        }
    }
    assert!(!f.moving(), "the fade must settle");
    assert_eq!(f.uniform().1, PULL_NONE, "the pull must reach nothing");
    assert!(
        min_x > 0.74,
        "the centre must hold while the pull fades, dipped to {min_x}"
    );
}

/// A surface with no area, or a card with none, is not a focus target — it is
/// a frame mid-resize, and following it would divide by nothing.
#[test]
fn a_degenerate_rect_or_surface_is_not_a_target() {
    let zero = Rect {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
    };
    assert_eq!(target_uv(Some(zero), SURFACE), None);
    assert_eq!(target_uv(Some(top_left()), (0.0, 0.0)), None);
    assert_eq!(target_uv(None, SURFACE), None);
}

/// A card hanging off the edge mid-glide must not throw the light off with
/// it — the centre stays somewhere on the page.
#[test]
fn a_card_off_the_edge_is_clamped_to_the_page() {
    let off = Rect {
        x: -4_000.0,
        y: 5_000.0,
        w: 300.0,
        h: 240.0,
    };
    assert_eq!(target_uv(Some(off), SURFACE), Some((0.0, 1.0)));
}

/// Whatever the pull is set to, the orbit's centre can never be dragged off
/// the page: the pools would take their falloff with them and the far side
/// would go flat. The bound is the geometry, not the constant — a card's
/// centre is at most half a page from the middle, so the move is at most
/// `PULL/2`.
#[test]
fn the_orbit_centre_can_never_leave_the_page() {
    let mut f = WashFocus::default();
    for r in [
        Rect {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        },
        Rect {
            x: SURFACE.0 - 1.0,
            y: SURFACE.1 - 1.0,
            w: 1.0,
            h: 1.0,
        },
        top_left(),
        bottom_right(),
    ] {
        f.step(Some(r), SURFACE, 16, MotionLevel::Off);
        let ((x, y), pull) = f.uniform();
        let moved = |v: f32| (v - 0.5).abs() * pull;
        assert!(moved(x) < 0.5 && moved(y) < 0.5, "{x} {y} at pull {pull}");
    }
}
